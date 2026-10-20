//! Composition and lifecycle of the single remote-content window.

use tauri::{
    webview::{NewWindowResponse, PermissionResponse},
    Manager, WebviewUrl, WebviewWindowBuilder,
};

use crate::{navigation, platform, profile};

pub(crate) fn run() -> tauri::Result<()> {
    let builder = tauri::Builder::default();
    #[cfg(windows)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init());
    builder
        .setup(|app| {
            if let Err(error) = create_window(app) {
                // Tauri panics on setup Err; report here before requesting clean exit.
                platform::show_startup_error(&error);
                app.handle().exit(1);
            } else {
                #[cfg(windows)]
                crate::updater::start(app.handle().clone());
            }
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

fn create_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder =
        WebviewWindowBuilder::new(app, "main", WebviewUrl::External(navigation::HOME.parse()?))
            .title("Discard")
            .inner_size(1200.0, 800.0)
            .theme(Some(tauri::Theme::Dark))
            .background_color(tauri::window::Color(36, 36, 41, 255))
            .on_navigation(|url| navigation::allowed(url.as_str()))
            .on_new_window(|_, _| NewWindowResponse::Deny)
            .on_download(|_, _| false)
            // Wry otherwise auto-grants media capture at the WKWebView origin layer.
            // Default explicitly requests normal engine permission behavior (Prompt on macOS).
            .on_permission_request(|_, _| PermissionResponse::Default);

    if let Some(directory) = profile::directory(app.handle())? {
        builder = builder.data_directory(directory);
    }
    let window = platform::configure(builder).build()?;
    platform::attach(&window)?;
    Ok(())
}
