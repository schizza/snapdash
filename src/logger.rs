use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

use directories::ProjectDirs;

pub enum LogType {
    Info,
    Warn,
    Error,
    DoNotLog,
}

static LOG_FILE: OnceLock<PathBuf> = OnceLock::new();

pub fn error(msg: impl AsRef<str>) {
    append(&format!("ERROR: {}", msg.as_ref()));
}

pub fn warn(msg: impl AsRef<str>) {
    append(&format!("WARNING: {}", msg.as_ref()));
}

pub fn info(msg: impl AsRef<str>) {
    append(&format!("INFO: {}", msg.as_ref()));
}

fn log_path() -> &'static PathBuf {
    LOG_FILE.get_or_init(|| {
        let proj = ProjectDirs::from("dev", "snapdash", "Snapdash")
            .expect("cannot determine app data dir");

        let dir = proj.data_dir();
        create_dir_all(dir).ok();

        dir.join("debug.log")
    })
}

fn append(msg: &str) {
    let path = log_path();

    let now = std::time::SystemTime::now();
    let ts = chrono::DateTime::<chrono::Local>::from(now).format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("{ts} {msg}");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}
