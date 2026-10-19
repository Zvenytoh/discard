//! Persistent engine data locations; never reads or migrates account data.

use std::{error::Error, path::PathBuf};

pub(crate) fn directory(_app: &tauri::AppHandle) -> Result<Option<PathBuf>, Box<dyn Error>> {
    #[cfg(windows)]
    let path = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "LOCALAPPDATA is unavailable")
    })?)
    .join("Discard")
    .join("WebView2");

    #[cfg(target_os = "linux")]
    let path = {
        use tauri::Manager;
        _app.path().app_local_data_dir()?.join("webview")
    };

    #[cfg(not(target_os = "macos"))]
    {
        std::fs::create_dir_all(&path)?;
        Ok(Some(path))
    }
    #[cfg(target_os = "macos")]
    Ok(None)
}
