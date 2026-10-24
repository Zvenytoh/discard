//! STA-owned WebView2 adaptation; all callbacks are event-driven.

use std::{
    cell::{Cell, RefCell},
    mem::size_of,
    rc::Rc,
};
use tauri::Manager;
use webview2_com::{
    take_pwstr,
    Microsoft::Web::WebView2::Win32::{
        ICoreWebView2, ICoreWebView2Controller, ICoreWebView2Settings9, ICoreWebView2_19,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL as NORMAL,
        COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC,
    },
    NavigationStartingEventHandler, NewWindowRequestedEventHandler,
};
use windows::{
    core::{w, Interface, Result, BOOL, PWSTR},
    Win32::{
        Foundation::{COLORREF, HWND},
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
        },
        UI::{
            Shell::{SetCurrentProcessExplicitAppUserModelID, ShellExecuteW},
            WindowsAndMessaging::{IsIconic, IsWindowVisible, SW_SHOWNORMAL},
        },
    },
};

thread_local! {
    static BACKGROUND: RefCell<Option<Rc<Background>>> = const { RefCell::new(None) };
}

struct Background {
    hwnd: HWND,
    controller: ICoreWebView2Controller,
    help: (ICoreWebView2, i64),
    navigation_token: i64,
    visible: Cell<Option<bool>>,
}

impl Drop for Background {
    fn drop(&mut self) {
        // SAFETY: subscription and engine are released on their owning UI STA.
        if let Err(error) = unsafe { self.help.0.remove_NewWindowRequested(self.help.1) } {
            eprintln!("Could not remove help handler: {error}");
        }
        // SAFETY: this subscription belongs to the same STA-owned webview.
        if let Err(error) = unsafe { self.help.0.remove_NavigationStarting(self.navigation_token) }
        {
            eprintln!("Could not remove browser link handler: {error}");
        }
    }
}

fn current() -> Option<Rc<Background>> {
    BACKGROUND.with(|slot| slot.borrow().clone())
}

fn open_browser(input: &str) {
    let Some(url) = crate::navigation::browser_url(input) else {
        return;
    };
    // SAFETY: parsed HTTP(S) URL is passed as the shell target, never as a command.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &windows::core::HSTRING::from(url.as_str()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        eprintln!("Could not open link in the default browser");
    }
}

pub(super) fn focus_content() {
    if let Some(state) = current() {
        // SAFETY: only called by native window controls on the owning UI STA.
        if let Err(error) = unsafe {
            state
                .controller
                .MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC)
        } {
            eprintln!("Could not return focus to Discord: {error}");
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
        let mut host = HWND::default();
        controller.ParentWindow(&mut host)?;
        super::titlebar::above_webview(host)?;
        let web = controller.CoreWebView2()?;
        let settings = web.Settings()?;
        // This host has no web-to-native app API, including Tauri IPC.
        settings.SetAreHostObjectsAllowed(false)?;
        settings.SetIsWebMessageEnabled(false)?;
        settings
            .cast::<ICoreWebView2Settings9>()?
            .SetIsNonClientRegionSupportEnabled(true)?;
        let mut help_token = 0;
        web.add_NewWindowRequested(
            &NewWindowRequestedEventHandler::create(Box::new(|_, args| {
                let Some(args) = args else { return Ok(()) };
                args.SetHandled(true)?;
                let mut initiated = BOOL(0);
                args.IsUserInitiated(&mut initiated)?;
                if !initiated.as_bool() {
                    return Ok(());
                }
                let mut uri = PWSTR::null();
                args.Uri(&mut uri)?;
                let uri = take_pwstr(uri);
                open_browser(&uri);
                Ok(())
            })),
            &mut help_token,
        )?;
        let mut navigation_token = 0;
        web.add_NavigationStarting(
            &NavigationStartingEventHandler::create(Box::new(|_, args| {
                let Some(args) = args else { return Ok(()) };
                let mut uri = PWSTR::null();
                args.Uri(&mut uri)?;
                let uri = take_pwstr(uri);
                if crate::navigation::allowed(&uri) || uri == "about:blank" {
                    return Ok(());
                }
                args.SetCancel(true)?;
                let mut initiated = BOOL(0);
                let mut redirected = BOOL(0);
                args.IsUserInitiated(&mut initiated)?;
                args.IsRedirected(&mut redirected)?;
                if initiated.as_bool() && !redirected.as_bool() {
                    open_browser(&uri);
                }
                Ok(())
            })),
            &mut navigation_token,
        )?;
        // Silence does not prove that an outgoing call/share is idle. Keep the
        // engine's normal target rather than trimming an active encoder.
        if let Ok(memory) = web.cast::<ICoreWebView2_19>() {
            if let Err(error) = memory.SetMemoryUsageTargetLevel(NORMAL) {
                eprintln!("Could not request normal WebView memory target: {error}");
            }
        }
        Background {
            hwnd,
            controller,
            help: (web, help_token),
            navigation_token,
            visible: Cell::new(None),
        }
    };
    let previous = BACKGROUND.with(|slot| slot.borrow_mut().replace(Rc::new(state)));
    drop(previous);
    theme();
    // SAFETY: native drag-region support and the security settings are configured
    // before this first real navigation, on the controller's owning STA.
    unsafe {
        if let Some(state) = current() {
            state
                .controller
                .CoreWebView2()?
                .Navigate(&windows::core::HSTRING::from(crate::navigation::HOME))?;
        }
    }
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
    }
}

pub(super) fn attach(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    if let Err(error) = super::titlebar::attach(window) {
        eprintln!("Custom title bar unavailable; restoring the native frame: {error}");
        window.set_decorations(true)?;
        window.set_shadow(true)?;
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
