//! Crate-wide logging. Three macros:
//!
//! - `tlog!` — gated debug log to `tutor.log`. Off by default; enable with `RHE_LOG=1`. Format args are only evaluated when enabled (cheap fast-path on every event).
//! - `info!` — always-on user status. Goes to stderr with a `rhe: ` prefix. Used for "loading", "ready", and similar startup chatter the user expects to see.
//! - `rerror!` — always-on error report. Goes to stderr with a `rhe error: ` prefix. Used for failures the user should know about (init errors, file-load rejects, etc.).
//!
//! When `RHE_LOG=1` is set, every chord-key event, drill step transition, and engine emission gets a timestamped line in `tutor.log`. The Enter key in the tutor window truncates the file so a single repro attempt can be isolated.
//!
//! Log path defaults to `<cwd>/tutor.log`; override with `RHE_LOG_PATH`.

use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static ENABLED: AtomicBool = AtomicBool::new(false);
static STATE: OnceLock<Mutex<LogState>> = OnceLock::new();

struct LogState {
    file: File,
    path: PathBuf,
    start: Instant,
}

/// Resolve the log path. `RHE_LOG_PATH` wins; otherwise `tutor.log` in the current working directory.
fn log_path() -> PathBuf {
    if let Some(p) = std::env::var_os("RHE_LOG_PATH") {
        return PathBuf::from(p);
    }
    std::env::current_dir()
        .unwrap_or_default()
        .join("tutor.log")
}

/// Initialize the logger if `RHE_LOG=1`. Idempotent — calling twice keeps the first state. Truncates the existing file on first init so each rhe launch starts clean.
pub fn init() {
    if std::env::var_os("RHE_LOG").is_none() {
        return;
    }
    let path = log_path();
    let file = match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("rhe error: log init failed at {}: {}", path.display(), e);
            return;
        }
    };
    let state = LogState {
        file,
        path: path.clone(),
        start: Instant::now(),
    };
    let _ = STATE.set(Mutex::new(state));
    ENABLED.store(true, Ordering::Relaxed);
    eprintln!("rhe: logging tutor events to {}", path.display());
    log_line("rhe: log start");
}

/// Truncate the log file and write a fresh header. Bound to Enter in the tutor window so the user can isolate a single repro attempt.
pub fn reset() {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let Some(mtx) = STATE.get() else { return };
    let Ok(mut state) = mtx.lock() else { return };
    let _ = state.file.set_len(0);
    let _ = state.file.seek(SeekFrom::Start(0));
    state.start = Instant::now();
    let _ = writeln!(state.file, "[00000000ms] rhe: log reset");
    let _ = state.file.flush();
}

/// Append a line. Cheap fast-path when logging is off (one atomic load).
pub fn log_line(line: &str) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let Some(mtx) = STATE.get() else { return };
    let Ok(mut state) = mtx.lock() else { return };
    let elapsed = state.start.elapsed().as_millis();
    let _ = writeln!(state.file, "[{:>8}ms] {}", elapsed, line);
    let _ = state.file.flush();
}

/// Cheap predicate so callers can skip building a `format!` argument when the logger is disabled.
#[inline]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// `tlog!("...", args)` formats and writes a single line to the gated debug log, but only when logging is enabled — when disabled the format args aren't evaluated at all.
#[macro_export]
macro_rules! tlog {
    ($($arg:tt)*) => {
        if $crate::log::enabled() {
            $crate::log::log_line(&format!($($arg)*));
        }
    };
}

/// `info!("...", args)` always emits a single line to stderr with a `rhe: ` prefix. Use for user-visible status messages that aren't error conditions (startup banners, ready-to-use confirmations, etc.).
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        eprintln!("rhe: {}", format!($($arg)*));
    };
}

/// `rerror!("...", args)` always emits a single line to stderr with a `rhe error: ` prefix. Use for problem reports the user should see (init failures, dropped-file rejections, backend errors).
#[macro_export]
macro_rules! rerror {
    ($($arg:tt)*) => {
        eprintln!("rhe error: {}", format!($($arg)*));
    };
}
