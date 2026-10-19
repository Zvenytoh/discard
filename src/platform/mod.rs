//! Small native adapters; remote content receives no native commands.

#[cfg(target_os = "macos")]
mod macos;

pub(crate) fn configure(
    builder: tauri::webview::WebviewWindowBuilder<'_, tauri::Wry, tauri::App>,
) -> tauri::webview::WebviewWindowBuilder<'_, tauri::Wry, tauri::App> {
    #[cfg(target_os = "macos")]
    let builder = macos::configure(builder);
    #[cfg(windows)]
    let builder = builder.disable_drag_drop_handler();
    builder
}

pub(crate) fn attach(_window: &tauri::WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

pub(crate) fn show_error(message: &str) {
    eprintln!("{message}");
    #[cfg(windows)]
    {
        use windows::{core::HSTRING, Win32::UI::WindowsAndMessaging::*};
        // SAFETY: strings are owned for the synchronous call; no parent is required.
        unsafe {
            MessageBoxW(
                None,
                &HSTRING::from(message),
                windows::core::w!("Discard"),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}
