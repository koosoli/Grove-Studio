//! A plain-text log of what the app does, for finding out why a run stops.
//! It lives in `%LOCALAPPDATA%\Grove Studio\grove-studio.log` and is appended to on every run.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(|root| PathBuf::from(root).join("Grove Studio").join("grove-studio.log"))
}

/// Appends one timestamped line. Logging must never stop the app, so errors are ignored.
pub fn log(message: impl AsRef<str>) {
    let Some(path) = path() else { return };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.);
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("unnamed");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{seconds:.3} [{name}] {}", message.as_ref());
    }
}

/// Records every panic, on any thread, with its location and a backtrace.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        log(format!("PANIC: {info}"));
        log(format!(
            "backtrace:\n{}",
            std::backtrace::Backtrace::force_capture()
        ));
    }));
}
