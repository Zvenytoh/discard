//! Composition and lifecycle of the single remote-content window.

use tauri::{
    webview::{NewWindowResponse, PermissionResponse},
    Manager, WebviewUrl, WebviewWindowBuilder,
};

use crate::{navigation, platform, profile, tray};

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
                crate::platform::audio::initialize(app.handle());
                if let Err(error) = tray::create(app) {
                    // A failed tray must never leave an inaccessible hidden app.
                    eprintln!("Tray unavailable; closing the window will quit: {error}");
                }
                #[cfg(windows)]
                {
                    if let Err(error) = crate::platform::activity::start(app.handle()) {
                        eprintln!("Game detection unavailable: {error}");
                    }
                    crate::platform::rpc::start(app.handle());
                    crate::updater::start(app.handle().clone());
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                #[cfg(windows)]
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    crate::platform::activity::stop(window.app_handle());
                    crate::platform::rpc::stop(window.app_handle());
                    crate::platform::audio::stop(window.app_handle());
                }
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    if window.app_handle().tray_by_id("main").is_some() {
                        match window.hide() {
                            Ok(()) => {
                                api.prevent_close();
                                if let Err(error) =
                                    platform::refresh_visibility(window.app_handle())
                                {
                                    eprintln!("Could not refresh hidden renderer: {error}");
                                }
                            }
                            Err(error) => {
                                eprintln!("Could not hide window: {error}");
                                window.app_handle().exit(0);
                            }
                        }
                    } else {
                        window.app_handle().exit(0);
                    }
                }
            }
        })
        .run(tauri::generate_context!())
}

fn create_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Windows configures native drag regions before the first Discord navigation.
    let start = if cfg!(windows) {
        "about:blank"
    } else {
        navigation::HOME
    };
    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(start.parse()?))
        .title("Discard")
        .on_page_load(|window, _| {
            #[cfg(windows)]
            {
                crate::platform::activity::replay(window.app_handle());
                crate::platform::rpc::replay(window.app_handle());
                crate::platform::audio::replay(window.app_handle());
            }
        })
        .on_document_title_changed(|window, title| {
            let title = title
                .trim_end_matches(" | Discord")
                .trim_end_matches(" - Discord");
            let title = if title.is_empty() || title == "Discord" {
                "Amis"
            } else {
                title
            };
            if let Err(error) = window.set_title(&title.chars().take(160).collect::<String>()) {
                eprintln!("Could not update window title: {error}");
            }
        })
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
    // Vencord plugins must capture our getDisplayMedia wrapper, not bypass it.
    let builder = platform::configure(builder);
    let window = crate::mods::configure(builder, app.handle())?.build()?;
    platform::attach(&window)?;
    Ok(())
}
