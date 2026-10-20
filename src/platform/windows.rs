//! STA-owned WebView2 adaptation; all callbacks are event-driven.

use std::{
    cell::{Cell, RefCell},
    mem::size_of,
    rc::Rc,
};
use tauri::Manager;
use webview2_com::{
    IsDocumentPlayingAudioChangedEventHandler,
    Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Controller, ICoreWebView2_19, ICoreWebView2_8,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW as LOW,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL as NORMAL,
    },
};
use windows::{
    core::{w, Interface, Result, BOOL},
    Win32::{
        Foundation::{COLORREF, HWND},
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
        },
        UI::{
            Shell::SetCurrentProcessExplicitAppUserModelID,
            WindowsAndMessaging::{GetForegroundWindow, IsIconic, IsWindowVisible},
        },
    },
};

thread_local! {
    static BACKGROUND: RefCell<Option<Rc<Background>>> = const { RefCell::new(None) };
}

struct Background {
    hwnd: HWND,
    controller: ICoreWebView2Controller,
    memory: Option<ICoreWebView2_19>,
    audio: Option<(ICoreWebView2_8, i64)>,
    memory_enabled: Cell<bool>,
    visible: Cell<Option<bool>>,
    memory_level: Cell<Option<i32>>,
}

impl Drop for Background {
    fn drop(&mut self) {
        if let Some((audio, token)) = &self.audio {
            // SAFETY: native state and its subscription are released on the owning UI STA.
            if let Err(error) = unsafe { audio.remove_IsDocumentPlayingAudioChanged(*token) } {
                eprintln!("Could not remove audio observer: {error}");
            }
        }
    }
}

fn current() -> Option<Rc<Background>> {
    BACKGROUND.with(|slot| slot.borrow().clone())
}

pub(super) fn open_inbox() {
    let Some(state) = current() else { return };
    // SAFETY: invoked only by this window's native caption on its owning STA.
    // This one-shot DOM click uses the website's own inbox; no bridge is exposed.
    unsafe {
        let result = state.controller.CoreWebView2().and_then(|web| web.ExecuteScript(
            w!("(()=>{const b=document.querySelector('[aria-label=\"Boîte de réception\"], [aria-label=\"Inbox\"]');if(b)b.click()})()"), None));
        if let Err(error) = result {
            eprintln!("Could not open Discord inbox: {error}");
        }
    }
}

pub(super) fn identity() {
    // SAFETY: the static UTF-16 identifier is valid for this synchronous process call.
    if let Err(error) = unsafe { SetCurrentProcessExplicitAppUserModelID(w!("Zvenytoh.Discard")) } {
        eprintln!("Could not set taskbar identity: {error}");
    }
}

fn initialize(controller: ICoreWebView2Controller, hwnd: HWND) -> Result<()> {
    // SAFETY: Tauri invokes with_webview on the controller's owning UI STA.
    let state = unsafe {
        let web = controller.CoreWebView2()?;
        let settings = web.Settings()?;
        // This host has no web-to-native app API, including Tauri IPC.
        settings.SetAreHostObjectsAllowed(false)?;
        settings.SetIsWebMessageEnabled(false)?;
        let audio = web.cast::<ICoreWebView2_8>().ok().and_then(|audio| {
            let mut token = 0;
            match audio.add_IsDocumentPlayingAudioChanged(
                &IsDocumentPlayingAudioChangedEventHandler::create(Box::new(|_, _| {
                    apply();
                    Ok(())
                })),
                &mut token,
            ) {
                Ok(()) => Some((audio, token)),
                Err(error) => {
                    eprintln!("Audio observation unavailable; using normal memory mode: {error}");
                    None
                }
            }
        });
        let memory = web.cast::<ICoreWebView2_19>().ok();
        Background {
            hwnd,
            controller,
            memory_enabled: Cell::new(audio.is_some() && memory.is_some()),
            memory,
            audio,
            visible: Cell::new(None),
            memory_level: Cell::new(None),
        }
    };
    let previous = BACKGROUND.with(|slot| slot.borrow_mut().replace(Rc::new(state)));
    drop(previous);
    theme();
    apply();
    Ok(())
}

fn theme() {
    let Some(state) = current() else { return };
    // SAFETY: hwnd is the live Tauri window on this UI thread. Each attribute
    // receives the exact value type/size; unsupported DWM attributes are optional.
    unsafe {
        let dark = BOOL(1);
        let caption = COLORREF(0x00292424); // Discord dark background: #242429 (COLORREF is BGR).
        let text = COLORREF(0x00f5f3f2);
        let _ = DwmSetWindowAttribute(
            state.hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const _ as _,
            size_of::<BOOL>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            state.hwnd,
            DWMWA_CAPTION_COLOR,
            &caption as *const _ as _,
            size_of::<COLORREF>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            state.hwnd,
            DWMWA_TEXT_COLOR,
            &text as *const _ as _,
            size_of::<COLORREF>() as u32,
        );
    }
}

pub(super) fn apply() {
    let Some(state) = current() else { return };
    // SAFETY: window/audio callbacks and with_webview run on this UI STA; Rc
    // keeps state alive through reentrant COM calls without a RefCell borrow.
    unsafe {
        let visible = IsWindowVisible(state.hwnd).as_bool() && !IsIconic(state.hwnd).as_bool();
        if state.visible.get() != Some(visible) {
            match state.controller.SetIsVisible(visible) {
                Ok(()) => state.visible.set(Some(visible)),
                Err(error) => eprintln!("Could not update WebView visibility: {error}"),
            }
        }
        let mut playing = BOOL(0);
        let observed = state
            .audio
            .as_ref()
            .is_some_and(|(audio, _)| audio.IsDocumentPlayingAudio(&mut playing).is_ok());
        let level = if state.memory_enabled.get()
            && observed
            && !playing.as_bool()
            && GetForegroundWindow() != state.hwnd
        {
            LOW
        } else {
            NORMAL
        };
        if let Some(memory) = &state.memory {
            if state.memory_level.get() != Some(level.0) {
                match memory.SetMemoryUsageTargetLevel(level) {
                    Ok(()) => state.memory_level.set(Some(level.0)),
                    Err(error) => {
                        state.memory_enabled.set(false);
                        eprintln!("Memory optimization unavailable: {error}");
                        if memory.SetMemoryUsageTargetLevel(NORMAL).is_ok() {
                            state.memory_level.set(Some(NORMAL.0));
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn attach(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    if let Err(error) = super::titlebar::attach(window) {
        eprintln!("Custom title bar unavailable; keeping the native frame: {error}");
    }
    let app = window.app_handle().clone();
    // WebView2's ParentWindow is Wry's child container, not the native top-level.
    let hwnd = window.hwnd()?.0 as usize;
    window.with_webview(move |webview| {
        let hwnd = HWND(hwnd as *mut std::ffi::c_void);
        if let Err(error) = initialize(webview.controller(), hwnd) {
            super::show_error(&format!("Could not secure Discord's WebView: {error}"));
            app.exit(1);
        }
    })?;
    // Tauri delivers these native window events on its UI STA. Capture no
    // window/COM owner here, avoiding a window -> listener -> window cycle.
    window.on_window_event(|event| {
        match event {
            tauri::WindowEvent::Focused(_)
            | tauri::WindowEvent::Resized(_)
            | tauri::WindowEvent::ScaleFactorChanged { .. } => {
                apply();
            }
            tauri::WindowEvent::ThemeChanged(_) => {
                theme();
            }
            tauri::WindowEvent::Destroyed => {
                // Tauri dispatches native window events on its owning UI thread.
                let state = BACKGROUND.with(|slot| slot.borrow_mut().take());
                drop(state);
            }
            _ => {}
        }
    });
    Ok(())
}
