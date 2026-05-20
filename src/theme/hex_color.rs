/// Serde (de)serialization for `iced::Color` as a hex string.
/// Accepts `#rrggbb` (opaque) or `#rrggbbaa` (with alpha). Used via
/// `#[serde(with = "hex_color")]` on Palette's color fields.use iced::Color;
use iced::Color;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S: Serializer>(color: &Color, s: S) -> Result<S::Ok, S::Error> {
    let [r, g, b, a] = color.into_rgba8();
    let hex = if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    };
    s.serialize_str(&hex)
}

pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Color, D::Error> {
    let raw = String::deserialize(d)?;
    parse_hex(&raw).map_err(serde::de::Error::custom)
}

fn parse_hex(raw: &str) -> Result<Color, String> {
    let s = raw.trim().trim_start_matches('#');

    let byte = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&s[range], 16).map_err(|e| format!("invalid hex '{raw}': {e}"))
    };

    match s.len() {
        6 => Ok(Color::from_rgb8(byte(0..2)?, byte(2..4)?, byte(4..6)?)),
        8 => Ok(Color::from_rgba8(
            byte(0..2)?,
            byte(2..4)?,
            byte(4..6)?,
            byte(6..8)? as f32 / 255.0,
        )),
        _ => Err(format!(
            "hex color must be 6 or 8 digits, got '{raw}' ({} digits)",
            s.len()
        )),
    }
}
