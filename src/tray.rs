//! Native tray ownership and explicit open/quit actions; no page access.

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

use crate::platform;

pub(crate) fn create(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_menu(app.handle())?;

    let icon = app
        .default_window_icon()
        .ok_or("Missing tray icon")?
        .clone();

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Discard")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => restore(app),

            "quit" => app.exit(0),

            "toggle-vencord" | "toggle-activity" => {
                let name = if event.id.as_ref() == "toggle-vencord" {
                    "vencord"
                } else {
                    "activity"
                };

                if let Err(error) = crate::mods::toggle(app, name) {
                    platform::show_error(&format!("Could not save {name} preference: {error}"));
                }
            }

            #[cfg(windows)]
            "audio-refresh" => {
                let result = build_menu(app).and_then(|menu| {
                    app.tray_by_id("main")
                        .ok_or_else(|| "Tray unavailable".into())
                        .and_then(|tray| tray.set_menu(Some(menu)).map_err(Into::into))
                });
                if let Err(error) = result {
                    platform::show_error(&format!("Could not refresh audio targets: {error}"));
                }
            }
            #[cfg(windows)]
            "audio-stop" => platform::audio::stop(app),
            #[cfg(windows)]
            id if id.starts_with("audio:") => {
                if let Some((pid, created)) = id[6..].split_once(':') {
                    if let (Ok(pid), Ok(created)) = (pid.parse(), created.parse()) {
                        if let Err(error) = platform::audio::select(app, pid, created) {
                            platform::show_error(&error.to_string());
                        }
                    }
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,

                    button_state: MouseButtonState::Up,

                    ..
                }
            ) {
                restore(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn build_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open", "Ouvrir Discard", true, None::<&str>)?;

    let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;

    let config = app.state::<crate::mods::Config>();

    let vencord = MenuItem::with_id(
        app,
        "toggle-vencord",
        if config.vencord {
            "Désactiver Vencord et redémarrer"
        } else {
            "Activer Vencord et redémarrer"
        },
        true,
        None::<&str>,
    )?;

    let activity = MenuItem::with_id(
        app,
        "toggle-activity",
        if config.activity {
            "Désactiver la détection des jeux et redémarrer"
        } else {
            "Activer la détection des jeux et redémarrer"
        },
        config.vencord,
        None::<&str>,
    )?;

    let menu = Menu::with_items(app, &[&open, &vencord, &activity])?;
    #[cfg(windows)]
    {
        let audio = tauri::menu::Submenu::new(app, "Audio du stream (experimental)", true)?;
        let refresh = MenuItem::with_id(
            app,
            "audio-refresh",
            "Actualiser les applications",
            true,
            None::<&str>,
        )?;
        let stop = MenuItem::with_id(
            app,
            "audio-stop",
            "Desactiver l'audio natif",
            true,
            None::<&str>,
        )?;
        audio.append_items(&[&refresh, &stop])?;
        let targets = match platform::audio::targets() {
            Ok(targets) => targets,
            Err(error) => {
                eprintln!("Audio target enumeration unavailable: {error}");
                audio.append(&MenuItem::with_id(
                    app,
                    "audio-unavailable",
                    "Applications indisponibles - actualiser",
                    false,
                    None::<&str>,
                )?)?;
                Vec::new()
            }
        };
        for target in targets {
            audio.append(&MenuItem::with_id(
                app,
                format!("audio:{}:{}", target.pid, target.created),
                &target.title,
                true,
                None::<&str>,
            )?)?;
        }
        menu.append(&audio)?;
    }
    menu.append(&quit)?;
    Ok(menu)
}

pub(crate) fn restore(app: &tauri::AppHandle) {
    // Both tray and second-launch callbacks use the same UI-thread transition.
    let owner = app.clone();
    if let Err(error) = app.run_on_main_thread(move || {
        if let Some(window) = owner.get_webview_window("main") {
            let result = window
                .show()
                .and_then(|()| window.unminimize())
                // Restore rendering even when Windows refuses foreground focus.
                .and_then(|()| platform::refresh_visibility(&owner));
            if let Err(error) = result {
                platform::show_error(&format!("Could not restore Discard: {error}"));
            } else if let Err(error) = window.set_focus() {
                // Foreground restrictions are not a failed tray restoration.
                eprintln!("Discard restored but foreground focus was refused: {error}");
            }
        }
    }) {
        platform::show_error(&format!("Could not schedule window restoration: {error}"));
    }
}
