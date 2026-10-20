//! Small native adapters; remote content receives no native commands.

#[cfg(windows)]
use tauri::Manager;

#[cfg(windows)]
pub(crate) mod activity;
#[cfg(windows)]
pub(crate) mod audio;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
pub(crate) mod rpc;
#[cfg(windows)]
mod titlebar;
#[cfg(windows)]
mod windows;

pub(crate) fn configure(
    builder: tauri::webview::WebviewWindowBuilder<'_, tauri::Wry, tauri::App>,
) -> tauri::webview::WebviewWindowBuilder<'_, tauri::Wry, tauri::App> {
    #[cfg(windows)]
    windows::identity();
    #[cfg(windows)]
    let builder = builder
        .decorations(false)
        .shadow(false)
        .initialization_script(titlebar::PAGE_STYLE);
    #[cfg(target_os = "macos")]
    let builder = macos::configure(builder);
    #[cfg(windows)]
    let builder = builder.initialization_script(
        include_str!("audio.js").replace(
            "__DISCARD_WORKLET__",
            &serde_json::to_string(include_str!("audio-worklet.js"))
                .expect("static worklet serialization"),
        ),
    );
    // Native drag/drop events are unused; let Discord receive HTML file drops.
    builder.disable_drag_drop_handler()
}

pub(crate) fn attach(_window: &tauri::WebviewWindow) -> tauri::Result<()> {
    #[cfg(windows)]
    return windows::attach(_window);
    #[cfg(not(windows))]
    Ok(())
}

pub(crate) fn show_error(message: &str) {
    eprintln!("{message}");
    #[cfg(windows)]
    {
        use ::windows::{core::HSTRING, Win32::UI::WindowsAndMessaging::*};
        // SAFETY: strings are owned for the synchronous call; no parent is required.
        unsafe {
            MessageBoxW(
                None,
                &HSTRING::from(message),
                ::windows::core::w!("Discard"),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}

pub(crate) fn show_startup_error(error: &dyn std::fmt::Display) {
    show_error(&format!(
        "Discard could not start. Close other Discard windows and try again.\n\
         Your session data has been preserved.\n\n{error}"
    ));
}

pub(crate) fn refresh_visibility(app: &tauri::AppHandle) -> tauri::Result<()> {
    #[cfg(windows)]
    if let Some(webview) = app.get_webview_window("main") {
        webview.with_webview(|_| windows::apply())?;
    }
    #[cfg(not(windows))]
    let _ = app;
    Ok(())
}
