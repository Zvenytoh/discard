#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unsafe_op_in_unsafe_fn)]

mod app;
mod mods;
mod navigation;
mod platform;
mod profile;
mod tray;
#[cfg(windows)]
mod updater;

fn main() -> std::process::ExitCode {
    match app::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            platform::show_startup_error(&error);
            std::process::ExitCode::FAILURE
        }
    }
}
