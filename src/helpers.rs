//! Simple human-readable size
pub fn humanize_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    match n {
        n if n >= GB => format!("{:.1} GB", n as f64 / GB as f64),
        n if n >= MB => format!("{:.1} MB", n as f64 / MB as f64),
        n if n >= KB => format!("{:.1} KB", n as f64 / KB as f64),
        n => format!("{n} B"),
    }
}
