//! Composition and lifecycle of the single remote-content window.

use tauri::{webview::NewWindowResponse, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::{navigation, platform, profile};

pub(crate) fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let mut builder = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(navigation::HOME.parse()?),
            )
            .title("Discard")
            .inner_size(1200.0, 800.0)
            .theme(Some(tauri::Theme::Dark))
            .background_color(tauri::window::Color(30, 31, 34, 255))
            .on_navigation(|url| navigation::allowed(url.as_str()))
            .on_new_window(|_, _| NewWindowResponse::Deny)
            .on_download(|_, _| false);

            if let Some(directory) = profile::directory(app.handle())? {
                builder = builder.data_directory(directory);
            }
            let window = platform::configure(builder).build()?;
            platform::attach(&window)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                // Also exit on macOS, whose default keeps an app alive without windows.
                window.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!())
}
