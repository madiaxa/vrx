#![forbid(unsafe_code)]
mod app;
mod cli;
mod message;
#[cfg(feature = "smoke")]
mod native_bench;
mod ui;

fn main() -> std::process::ExitCode {
    match cli::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("VRX: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
