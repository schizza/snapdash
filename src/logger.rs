use std::fs::OpenOptions;
use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;

use tracing_appender::non_blocking::WorkerGuard;
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
/// Returns a `WorkerGuard` that **must be kept alive** for the lifetime of the
/// process — dropping it flushes any pending log lines from the non-blocking
/// file writer.
///
/// Env var `RUST_LOG` controls filtering; default is `snapdash=info,warn`.
pub fn init() -> anyhow::Result<WorkerGuard> {
    let log_path = log_path()?;

    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("cannot open log file {}", log_path.display()))?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("snapdash=info,warn"));

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true);

    let stdout_layer = fmt::layer().with_writer(std::io::stdout).pretty();

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}

fn log_path() -> anyhow::Result<PathBuf> {
    let proj = ProjectDirs::from("dev", "snapdash", "Snapdash")
        .context("cannot determine app data dir")?;
    Ok(proj.data_dir().join("debug.log"))
}
