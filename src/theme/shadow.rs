/// Serde for `iced::Shadow` as a nested object:
/// `{ "color": "#000000", "offset_x": 0.0, "offset_y": 10.0, "blur_radius": 20.0 }`.
use iced::{Color, Shadow, Vector};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
pub struct ShadowSpec {
    #[serde(with = "super::hex_color")]
    color: Color,
    offset_x: f32,
    offset_y: f32,
    blur_radius: f32,
}

pub fn serialize<S: Serializer>(shadow: &Shadow, s: S) -> Result<S::Ok, S::Error> {
    ShadowSpec {
        color: shadow.color,
        offset_x: shadow.offset.x,
        offset_y: shadow.offset.y,
        blur_radius: shadow.blur_radius,
    }
    .serialize(s)
}

pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Shadow, D::Error> {
    let spec = ShadowSpec::deserialize(d)?;
    Ok(Shadow {
        color: spec.color,
        offset: Vector::new(spec.offset_x, spec.offset_y),
        blur_radius: spec.blur_radius,
    })
}
