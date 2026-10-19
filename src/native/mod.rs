//! The UI thread owns the window, COM apartment and WebView2 controller.

mod webview;

use std::{cell::RefCell, rc::Rc};
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller;
use windows::{
    core::{w, HSTRING},
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::{
            Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
            LibraryLoader::GetModuleHandleW,
        },
        UI::{
            HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
            WindowsAndMessaging::*,
        },
    },
};

const FAILED: u32 = WM_APP + 1;
thread_local! {
    static APP: RefCell<Option<Rc<App>>> = const { RefCell::new(None) };
}

struct App {
    hwnd: HWND,
    controller: RefCell<Option<ICoreWebView2Controller>>,
    error: RefCell<Option<String>>,
}

fn app() -> Option<Rc<App>> {
    APP.with(|slot| slot.borrow().clone())
}

pub(crate) fn show_error(message: &str) {
    // SAFETY: both UTF-16 strings outlive this synchronous call.
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(message),
            w!("Discard"),
            MB_OK | MB_ICONERROR,
        );
    }
}

struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        // SAFETY: constructed after successful initialization on this same thread.
        unsafe { CoUninitialize() };
    }
}

impl App {
    fn controller(&self) -> Option<ICoreWebView2Controller> {
        self.controller.borrow().clone()
    }

    fn resize(&self) -> windows::core::Result<()> {
        if let Some(controller) = self.controller() {
            let mut bounds = RECT::default();
            // SAFETY: the live window and controller belong to this UI thread.
            unsafe {
                GetClientRect(self.hwnd, &mut bounds)?;
                controller.SetBounds(bounds)?;
            }
        }
        Ok(())
    }

    fn report_failure(&self, error: impl std::fmt::Display) {
        *self.error.borrow_mut() = Some(error.to_string());
        // SAFETY: no pointers are posted and this live window belongs to the STA.
        if unsafe { PostMessageW(Some(self.hwnd), FAILED, WPARAM(0), LPARAM(0)) }.is_err() {
            // SAFETY: the window is owned by this thread. Destruction needs no COM borrow.
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

pub(crate) fn run() -> Result<(), String> {
    let base = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is unavailable.")?;
    let profile = std::path::PathBuf::from(base)
        .join("Discard")
        .join("WebView2");
    std::fs::create_dir_all(&profile).map_err(|error| error.to_string())?;

    // SAFETY: all Win32 and COM objects stay on this thread. Structures and UTF-16
    // strings live through their calls; the controller is closed before COM exits.
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|error| error.to_string())?;
        let _com = Com;
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
            .map_err(|error| error.to_string())?;
        let instance = HINSTANCE(GetModuleHandleW(None).map_err(|error| error.to_string())?.0);
        let class = WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Discard.Native.1"),
            lpfnWndProc: Some(window_proc),
            hCursor: LoadCursorW(None, IDC_ARROW).map_err(|error| error.to_string())?,
            hIcon: LoadIconW(None, IDI_APPLICATION).map_err(|error| error.to_string())?,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(windows::core::Error::from_thread().to_string());
        }
        let hwnd = CreateWindowExW(
            Default::default(),
            class.lpszClassName,
            w!("Discard"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1200,
            800,
            None,
            None,
            Some(instance),
            None,
        )
        .map_err(|error| error.to_string())?;
        let state = Rc::new(App {
            hwnd,
            controller: RefCell::new(None),
            error: RefCell::new(None),
        });
        APP.with(|slot| *slot.borrow_mut() = Some(state.clone()));
        let _ = ShowWindow(hwnd, SW_SHOW);
        if let Err(error) = state.create_webview(&HSTRING::from(profile.as_os_str())) {
            let _ = DestroyWindow(hwnd);
            return Err(format!(
                "WebView2 could not start. Install the Evergreen Runtime.\n{error}"
            ));
        }

        let mut message = MSG::default();
        let result = loop {
            let status = GetMessageW(&mut message, None, 0, 0).0;
            if status == -1 {
                break Err(windows::core::Error::from_thread().to_string());
            }
            if status == 0 {
                break Ok(());
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        };
        if app().is_some() {
            let _ = DestroyWindow(hwnd);
        }
        let failure = state.error.borrow_mut().take();
        // Drop retained state and COM interfaces before the apartment guard.
        drop(state);
        result?;
        failure.map_or(Ok(()), Err)
    }
}

/// # Safety
/// Windows invokes this procedure on the UI thread. WM_DPICHANGED's RECT is
/// borrowed from Windows and is valid only during this callback.
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: handles belong to this UI thread; message payloads follow Win32's ABI.
    unsafe {
        match msg {
            WM_SIZE => {
                if let Some(state) = app() {
                    if let Err(error) = state.resize() {
                        state.report_failure(error);
                    }
                }
                return LRESULT(0);
            }
            WM_MOVE => {
                if let Some(controller) = app().and_then(|state| state.controller()) {
                    if let Err(error) = controller.NotifyParentWindowPositionChanged() {
                        if let Some(state) = app() {
                            state.report_failure(error);
                        }
                    }
                }
            }
            WM_DPICHANGED => {
                let rect = &*(lp.0 as *const RECT);
                if let Err(error) = SetWindowPos(
                    hwnd,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                ) {
                    if let Some(state) = app() {
                        state.report_failure(error);
                    }
                }
                return LRESULT(0);
            }
            FAILED => {
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
            WM_DESTROY => {
                let state = APP.with(|slot| slot.borrow_mut().take());
                if let Some(state) = state {
                    let controller = state.controller.borrow_mut().take();
                    if let Some(controller) = controller {
                        let _ = controller.Close();
                    }
                }
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}
