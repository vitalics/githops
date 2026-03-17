//! Structured logger for githops verbose output.
//!
//! Initialised once in `main()` via [`init`]. Any module can then call
//! [`log`] (or the convenience macros [`info!`], [`verbose!`], [`error!`],
//! [`trace!`]) without threading state through every function signature.
//!
//! # Default format
//! ```text
//! [HH:MM:SS.mmm] [KIND] (layer) message
//! ```
//!
//! # Custom template
//! Pass `--verbose-template` with any string containing the tokens:
//! - `$t` – timestamp
//! - `$k` – kind  (INFO / VERBOSE / ERROR / TRACE)
//! - `$l` – layer (schema validation / yaml resolve / yaml exec)
//! - `$m` – message

use colored::Colorize;
use std::sync::OnceLock;
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Severity / kind of a log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Verbose,
    Error,
    Trace,
}

/// Subsystem that produced the log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLayer {
    /// JSON-schema generation and validation.
    SchemaValidation,
    /// YAML config loading and include resolution.
    YamlResolve,
    /// Hook command execution.
    YamlExec,
}

// ---------------------------------------------------------------------------
// Global logger
// ---------------------------------------------------------------------------

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub struct Logger {
    pub verbose: bool,
    /// Template string; default `"[$t] [$k] ($l) $m"`.
    pub template: String,
}

/// Initialise the global logger. Must be called once, before any [`log`] call.
pub fn init(verbose: bool, template: String) {
    LOGGER.get_or_init(|| Logger { verbose, template });
}

/// Emit a log entry if the logger is initialised and the kind is active.
///
/// When `verbose = false` only `INFO` and `ERROR` entries are emitted.
pub fn log(kind: LogKind, layer: LogLayer, msg: &str) {
    let logger = match LOGGER.get() {
        Some(l) => l,
        None => return,
    };

    if !logger.verbose && !matches!(kind, LogKind::Info | LogKind::Error) {
        return;
    }

    let time = current_time_str();

    let kind_str = match kind {
        LogKind::Info    => "INFO",
        LogKind::Verbose => "VERBOSE",
        LogKind::Error   => "ERROR",
        LogKind::Trace   => "TRACE",
    };

    let layer_str = match layer {
        LogLayer::SchemaValidation => "schema validation",
        LogLayer::YamlResolve      => "yaml resolve",
        LogLayer::YamlExec         => "yaml exec",
    };

    let line = logger.template
        .replace("$t", &time)
        .replace("$k", kind_str)
        .replace("$l", layer_str)
        .replace("$m", msg);

    let colored_line = match kind {
        LogKind::Info    => line.normal(),
        LogKind::Verbose => line.cyan(),
        LogKind::Error   => line.red().bold(),
        LogKind::Trace   => line.dimmed(),
    };

    eprintln!("{}", colored_line);
}

// ---------------------------------------------------------------------------
// Convenience macros
// ---------------------------------------------------------------------------

/// Emit an INFO-level log entry.
#[macro_export]
macro_rules! log_info {
    ($layer:expr, $msg:expr) => {
        $crate::logger::log($crate::logger::LogKind::Info, $layer, $msg)
    };
    ($layer:expr, $fmt:literal, $($arg:tt)*) => {
        $crate::logger::log($crate::logger::LogKind::Info, $layer, &format!($fmt, $($arg)*))
    };
}

/// Emit a VERBOSE-level log entry (suppressed unless `-v` is set).
#[macro_export]
macro_rules! log_verbose {
    ($layer:expr, $msg:expr) => {
        $crate::logger::log($crate::logger::LogKind::Verbose, $layer, $msg)
    };
    ($layer:expr, $fmt:literal, $($arg:tt)*) => {
        $crate::logger::log($crate::logger::LogKind::Verbose, $layer, &format!($fmt, $($arg)*))
    };
}

/// Emit an ERROR-level log entry.
#[macro_export]
macro_rules! log_error {
    ($layer:expr, $msg:expr) => {
        $crate::logger::log($crate::logger::LogKind::Error, $layer, $msg)
    };
    ($layer:expr, $fmt:literal, $($arg:tt)*) => {
        $crate::logger::log($crate::logger::LogKind::Error, $layer, &format!($fmt, $($arg)*))
    };
}

/// Emit a TRACE-level log entry (suppressed unless `-v` is set).
#[macro_export]
macro_rules! log_trace {
    ($layer:expr, $msg:expr) => {
        $crate::logger::log($crate::logger::LogKind::Trace, $layer, $msg)
    };
    ($layer:expr, $fmt:literal, $($arg:tt)*) => {
        $crate::logger::log($crate::logger::LogKind::Trace, $layer, &format!($fmt, $($arg)*))
    };
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns the current UTC wall-clock time as `HH:MM:SS.mmm`.
fn current_time_str() -> String {
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = dur.as_secs();
    let millis = dur.subsec_millis();
    let h = (total_secs / 3600) % 24;
    let m = (total_secs / 60) % 60;
    let s = total_secs % 60;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, millis)
}
