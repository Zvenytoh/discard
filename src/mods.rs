//! Bundled browser mod; no remote native commands or runtime script downloads.
use std::path::PathBuf;
use tauri::Manager;

pub(crate) struct Config {
    directory: PathBuf,
    pub(crate) vencord: bool,
    pub(crate) activity: bool,
}

pub(crate) fn configure<'a>(
    builder: tauri::WebviewWindowBuilder<'a, tauri::Wry, tauri::App>,
    app: &tauri::AppHandle,
) -> Result<tauri::WebviewWindowBuilder<'a, tauri::Wry, tauri::App>, Box<dyn std::error::Error>> {
    let directory = app.path().app_config_dir()?;
    let config = Config {
        vencord: !directory.join("vencord.disabled").try_exists()?,
        activity: !directory.join("activity.disabled").try_exists()?,
        directory,
    };
    let builder = if config.vencord {
        let css = serde_json::to_string(include_str!("../vendor/vencord/browser.css"))?;
        builder.initialization_script(include_str!("stream.js")).initialization_script(format!(
            "(() => {{ if (window.top !== window || location.origin !== 'https://discord.com') return;\
             const bundledCss = {css};\
             Object.defineProperty(window, 'VencordNative', {{configurable:true, set(value) {{\
               value.native.getRendererCss = async () => bundledCss;\
               Object.defineProperty(window, 'VencordNative', {{value, writable:true, configurable:true}});\
             }}}});\n{}\nwindow.Vencord = Vencord;\n{}\n}})();",
            include_str!("../vendor/vencord/browser.js"),
            include_str!("mods.js"),
        ))
    } else {
        builder
    };
    app.manage(config);
    Ok(builder)
}

pub(crate) fn toggle(app: &tauri::AppHandle, name: &str) -> std::io::Result<()> {
    let config = app.state::<Config>();
    let marker = config.directory.join(format!("{name}.disabled"));
    if marker.try_exists()? {
        std::fs::remove_file(marker)?;
    } else {
        std::fs::create_dir_all(&config.directory)?;
        std::fs::write(marker, b"Disabled through the native tray menu.\n")?;
    }
    app.request_restart();
    Ok(())
}
