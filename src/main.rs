#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(windows)]
mod native;
#[cfg(windows)]
mod navigation;

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    match native::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            native::show_error(&error);
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
compile_error!("Discard requires Windows, Win32 and Microsoft Edge WebView2.");
