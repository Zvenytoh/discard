use tauri::{webview::WebviewWindowBuilder, App, Wry};

pub(super) fn configure(
    builder: WebviewWindowBuilder<'_, Wry, App>,
) -> WebviewWindowBuilder<'_, Wry, App> {
    builder
        .data_store_identifier([
            213, 50, 140, 206, 145, 20, 70, 192, 190, 120, 110, 183, 97, 61, 185, 142,
        ])
        .background_throttling(tauri::utils::config::BackgroundThrottlingPolicy::Disabled)
}
