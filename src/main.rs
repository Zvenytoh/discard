#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unsafe_op_in_unsafe_fn)]

mod app;
mod navigation;
mod platform;
mod profile;

fn main() -> std::process::ExitCode {
    match app::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            platform::show_error(&format!(
                "Discard could not start. Close other Discard windows and try again.\n\
                 Your session data has been preserved.\n\n{error}"
            ));
            std::process::ExitCode::FAILURE
        }
    }
}
