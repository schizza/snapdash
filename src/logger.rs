use std::fs::OpenOptions;
use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// UI status hint
#[derive(Clone, Copy, Debug)]
pub enum LogType {
    Info,
    Warn,
    Error,
    DoNotLog,
}

/// Initialize the global tracing subscriber.
///
/// Always installs a stdout layer. File-logging is *best-effort*: if we
/// cannot create or open the log file (read-only fs, permission denied,
/// sandbox restriction), the reason is written to stderr and the app
/// continues with stdout-only logging. File logging is not a startup
/// prerequisite.
///
/// Returns `Some(WorkerGuard)` when the file layer is active — the guard
/// must be kept alive for the process lifetime so pending writes flush on
/// exit. `None` means stdout-only logging is in effect.
pub fn init() -> Option<WorkerGuard> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("snapdash=info,warn"));

    let stdout_layer = fmt::layer().with_writer(std::io::stdout).pretty();

    match try_file_writer() {
        Ok((non_blocking, guard)) => {
            let file_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true);

            tracing_subscriber::registry()
                .with(filter)
                .with(stdout_layer)
                .with(file_layer)
                .init();

            Some(guard)
        }
        Err(e) => {
            // Subscriber is not installed - use stderr directly.
            eprintln!("warning: file logging disabled: {e:#}");

            tracing_subscriber::registry()
                .with(filter)
                .with(stdout_layer)
                .init();

            tracing::warn!(error = %e, "file logging disabled; stdout only");

            None
        }
    }
}

fn try_file_writer() -> anyhow::Result<(NonBlocking, WorkerGuard)> {
    let log_path = log_path()?;

    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create log dir {}", parent.display()))?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("cannot open log file {}", log_path.display()))?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);
    Ok((non_blocking, guard))
}

pub fn log_path() -> anyhow::Result<PathBuf> {
    let proj = ProjectDirs::from("dev", "snapdash", "Snapdash")
        .context("cannot determine app data dir")?;
    Ok(proj.data_dir().join("debug.log"))
}

/// Truncates the active log file to zero bytes. The non-blocking
/// appender keeps writing from offset 0 afterwards, so future log
/// lines just continue normally. A few in-flight messages from the
/// appender's internal queue may still land at the top of the
/// freshly-cleared file — that's a tracing-appender quirk we can't
/// avoid without restarting the subscriber.
pub fn clear_log() -> anyhow::Result<()> {
    let path = log_path()?;

    // I logging was disabled at startup, the file may not exists.
    // Treat that as already cleared
    if !path.exists() {
        return Ok(());
    }

    std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)
        .with_context(|| format!("truncate log {}", path.display()))?;

    tracing::info!("log file cleared");
    Ok(())
}

pub fn get_log_size() -> anyhow::Result<u64> {
    let path = log_path()?;

    if !path.exists() {
        anyhow::bail!(format!("log file not found {}", path.display()))
    };

    let log_size = std::fs::metadata(&path)
        .with_context(|| format!("error reading metadata of log file {}", path.display()))?
        .len();

    Ok(log_size)
}
