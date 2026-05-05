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

/// Compresses "<number> <unit>" strings to SI-prefixed form when
/// the magnitude is far from 1.0. Recognizes plain SI base units
/// (W, Wh, V, A, Hz, B) and skips already-prefixed values
/// ("1.5 kW") to avoid double-scaling.
///
/// Pass-through for non-numerical strings ("On"), unsupported units
/// ("°C"), or values within reasonable range (1..1000).
pub fn humanize_magnitude(raw: &str) -> String {
    let trimmed = raw.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let num_str = parts.next().unwrap_or("");
    let unit = parts.next().unwrap_or("").trim();

    let Ok(n) = num_str.parse::<f64>() else {
        return raw.to_string();
    };

    // Only scale plain bases — skip "kW", "MWh" (already prefixed).
    if !matches!(unit, "W" | "Wh" | "V" | "A" | "Hz" | "B" | "VA" | "VAr") {
        return raw.to_string();
    }

    let abs = n.abs();
    let (scaled, prefix) = if abs >= 1_000_000_000.0 {
        (n / 1_000_000_000.0, "G")
    } else if abs >= 1_000_000.0 {
        (n / 1_000_000.0, "M")
    } else if abs >= 1_000.0 {
        (n / 1_000.0, "k")
    } else if abs > 0.0 && abs < 1e-9 {
        (n * 1e12, "p")
    } else if abs > 0.0 && abs < 1e-6 {
        (n * 1e9, "n")
    } else if abs > 0.0 && abs < 0.001 {
        (n * 1_000_000.0, "µ")
    } else if abs > 0.0 && abs < 0.01 {
        (n * 1_000.0, "m")
    } else {
        // 1..1000 range — no compression needed
        return raw.to_string();
    };

    let formated = format!("{:.2}", scaled);
    let trimmed = formated.trim_end_matches('0').trim_end_matches('.');

    let display = if trimmed.is_empty() || trimmed == "-" {
        formated.as_str()
    } else {
        trimmed
    };

    format!("{} {}{}", display, prefix, unit)
}
