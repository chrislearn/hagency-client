// this stub is necessary because some platforms require building
// as dll (mobile / wasm) and some require to be built as executable
// unfortunately cargo doesn't facilitate this without a main.rs stub

// Hide the command prompt console window on Windows, if desired.
// TODO: move this into Makepad itself as an addition to the `MAKEPAD` env var.
#![cfg_attr(
    all(any(feature = "hide_windows_console", packaging_build), target_os = "windows"),
    windows_subsystem = "windows",
)]

#[cfg(feature = "standalone")]
fn main() {
    #[cfg(target_os = "windows")]
    if let Some(result) = hagency_app::updates::run_helper_if_requested() {
        if result.is_err() { std::process::exit(1); }
        return;
    }
    if let Some(result)=hagency::native_owner::run_guardian_if_requested() {
        if result.is_err(){std::process::exit(1);}return;
    }
    #[cfg(target_os = "windows")]
    let _instance = match hagency_app::desktop_integration::acquire_instance() {
        Ok(Some(instance)) => instance,
        Ok(None) => return,
        Err(error) => { eprintln!("Could not initialize Hagency: {error}"); return; }
    };
    hagency_app::app::app_main();
    #[cfg(target_os = "windows")]
    hagency_app::updates::commit_after_shutdown();
}

/// Without `standalone`, Rinx is a library for a host such as OctoSense.
#[cfg(not(feature = "standalone"))]
fn main() {}
