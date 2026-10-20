//! One startup check, with native consent and a signed Inno installer.

use std::{io::Write, os::windows::process::CommandExt, process::Command, time::Duration};

use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

pub(crate) fn start(app: tauri::AppHandle) {
    // Portable builds are updated manually; never install into an arbitrary folder.
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let Some(directory) = executable.parent() else {
        return;
    };
    if !directory.join("unins000.exe").is_file() {
        return;
    }
    let directory = directory.to_path_buf();
    tauri::async_runtime::spawn(async move {
        let result = async {
            let Some(mut update) = app.updater_builder()
                .timeout(Duration::from_secs(10))
                .build()?.check().await? else { return Ok::<_, Box<dyn std::error::Error + Send + Sync>>(()) };
            // Restrict manifest-controlled downloads before following GitHub's CDN redirect.
            let expected = format!("https://github.com/Zvenytoh/discard/releases/download/v{}/Discard-Setup.exe", update.version);
            if update.download_url.as_str() != expected {
                return Err("Unexpected update download URL".into());
            }
            let prompt_app = app.clone();
            let version = update.version.clone();
            let accepted = tauri::async_runtime::spawn_blocking(move || {
                prompt_app.dialog().message(format!(
                    "Discard {version} is available. Download and install it now? Discard will close and restart after installation."
                )).title("Discard update")
                    .buttons(MessageDialogButtons::OkCancelCustom("Install update".into(), "Later".into()))
                    .blocking_show()
            }).await?;
            if !accepted { return Ok(()) }
            update.timeout = Some(Duration::from_secs(120));
            let installation = async {
                // download() verifies the signature AND its signed version before returning.
                let bytes = update.download(|_, _| {}, || {}).await?;
                let mut file = tempfile::Builder::new().prefix("Discard-update-").suffix(".exe").tempfile()?;
                file.write_all(&bytes)?;
                file.as_file().sync_all()?;
                let (_, path) = file.keep()?;
                let launch = Command::new(&path)
                    .args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CLOSEAPPLICATIONS", "/DISCARDUPDATE=1"])
                    .arg(format!("/DIR={}", directory.display()))
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW; installer owns its UI.
                    .spawn();
                if let Err(error) = launch {
                    std::fs::remove_file(path)?;
                    return Err(error.into());
                }
                app.exit(0);
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            }.await;
            if let Err(error) = installation {
                eprintln!("Update installation failed: {error}");
                app.dialog().message("The update could not be downloaded, verified or started. Your current installation has been kept. Please try again next time you open Discard.")
                    .title("Discard update").kind(MessageDialogKind::Error).show(|_| {});
            }
            Ok(())
        }.await;
        // Offline, missing feed or invalid metadata must never prevent normal startup.
        if let Err(error) = result {
            eprintln!("Startup update check: {error}");
        }
    });
}
