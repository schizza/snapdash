//! The colour field's texture: hue across, saturation down, computed
//! once as RGBA bytes and drawn scaled.
//!
//! ## Why a texture and not two gradient quads
//!
//! Because iced's gradients cannot draw this correctly, and the reason
//! is arithmetic rather than taste.
//!
//! Both the quad and the triangle gradient shaders interpolate between
//! stops with `smoothstep` rather than linearly. `smoothstep(t) =
//! 3t^2 - 2t^3` departs from `t` by up to 9.6% of the segment it spans.
//! On the saturation axis that is a ten-point error: at t = 0.211 the
//! shader emits 0.115, so the field would show 11.5% saturation exactly
//! where the marker claims 21%.
//!
//! On the hue axis the natural stops are the six vertices of the colour
//! wheel plus the end of the range, which is seven stops and six
//! segments of about 60 degrees, so the same 9.6% is about 5.8 degrees.
//! It is not spread evenly either: it lands as wide plateaus of pure
//! red, yellow and green sitting on the stops, with the transitions
//! between them compressed. Subdividing is not a way out, because
//! `iced::gradient::Linear` holds at most eight stops in total.
//!
//! A slider whose colours are a few percent off is merely ugly. A
//! colour picker whose marker sits on a different colour from the one
//! it names is wrong, because naming the colour under the marker is the
//! entire job of the widget. That is what disqualifies gradients here
//! rather than just degrading them.
//!
//! ## Why not a wgpu shader
//!
//! It would be exact, and it would be wgpu-only. iced's default
//! features include the `tiny-skia` software renderer, which is what
//! runs on a machine with no usable adapter: a remote desktop session,
//! a VM, an old GPU. A custom shader primitive draws nothing at all
//! there. For an application that is made entirely of these widgets, a
//! hole shaped like "the colour picker is invisible on some machines"
//! is not a hole we can leave.
//!
//! So the field is computed exactly in Rust, once, and drawn scaled.
//! Every backend gets the same pixels.
//!
//! Recorded in `docs/adr/0005-the-colour-field-is-a-texture.md`.

use std::sync::OnceLock;

use iced::widget::image::Handle;

/// Texture width, and therefore the number of distinct hues the field
/// can show.
///
/// 256 columns over 359 degrees puts each column 1.4 degrees from its
/// neighbour, so the worst a marker can be wrong about its own hue is
/// 0.7 degrees. That is roughly three of 255 levels on the ramping
/// channel, against the 5.8 degrees a gradient would cost.
pub const WIDTH: u32 = 256;

/// Texture height, and therefore the number of distinct saturations.
///
/// Half the width, matching the shape of the field on screen: it is
/// drawn about twice as wide as it is tall, so steps of equal size in
/// both directions want half as many rows as columns. 128 rows over 100
/// percent still lands every saturation the marker can name within 0.4
/// of a percentage point of a row.
pub const HEIGHT: u32 = 128;

/// The hue of the rightmost column, in degrees.
///
/// 359 and not 360, deliberately. Hue is an angle, so 360 is the same
/// red the leftmost column already shows; capping one degree short
/// keeps every column a distinct colour and keeps the axis a plain
/// range rather than a wrapping one.
pub const MAX_HUE: f32 = 359.0;

/// The saturation of the bottom row, in percent, matching the range
/// Home Assistant's `hs_color` attribute uses.
pub const MAX_SATURATION: f32 = 100.0;

/// The colour field, as an image handle every caller can draw.
pub fn handle() -> Handle {
    static TEXTURE: OnceLock<Handle> = OnceLock::new();

    TEXTURE
        .get_or_init(|| Handle::from_rgba(WIDTH, HEIGHT, render()))
        .clone()
}

/// The colour field as RGBA bytes, row-major from the top left.
///
/// Both axes are inclusive of their ends, so the division is by one
/// less than the extent: column 0 is hue 0 and column 255 is
/// [`MAX_HUE`], row 0 is saturation 0 and row 127 is
/// [`MAX_SATURATION`]. Getting that off by one would put the pure hues
/// one column short of the right edge and leave the field unable to
/// name a fully saturated colour at all.
fn render() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);

    for row in 0..HEIGHT {
        let saturation = row as f32 / (HEIGHT - 1) as f32;

        for column in 0..WIDTH {
            let hue = column as f32 * MAX_HUE / (WIDTH - 1) as f32;

            pixels.extend_from_slice(&pixel(hue, saturation));
        }
    }

    pixels
}

/// One pixel of the field, from HSV with the value axis pinned to 1.
///
/// Value is pinned because this field sets hue and saturation only.
/// Brightness is its own axis with its own control, and a
/// `light.turn_on` that omits `brightness` leaves the lamp's brightness
/// where the user put it. A field that dimmed towards black down one
/// edge would be showing an axis it does not set.
///
/// `saturation` is the 0..=1 fraction rather than the 0..=100 percent,
/// because that is the form the HSV arithmetic wants.
fn pixel(hue: f32, saturation: f32) -> [u8; 4] {
    let sector = hue / 60.0;

    // The standard formulation: chroma is the full swing between the
    // brightest and darkest channel, the ramping channel is chroma
    // scaled by how far into the sector the hue sits, and `floor` lifts
    // the whole triple so the darkest channel lands on V - C.
    let chroma = saturation;
    let ramp = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let floor = 1.0 - chroma;

    let (red, green, blue) = match sector as u32 {
        0 => (chroma, ramp, 0.0),
        1 => (ramp, chroma, 0.0),
        2 => (0.0, chroma, ramp),
        3 => (0.0, ramp, chroma),
        4 => (ramp, 0.0, chroma),
        _ => (chroma, 0.0, ramp),
    };

    [
        level(red + floor),
        level(green + floor),
        level(blue + floor),
        u8::MAX,
    ]
}

/// A 0..=1 colour component as an eight-bit level.
///
/// Rounded rather than truncated: truncation biases every channel
/// downwards by up to a level, which over a field this size shows up as
/// a visible darkening towards the saturated edge.
fn level(component: f32) -> u8 {
    (component * 255.0).round() as u8
}

#[cfg(test)]
mod tests;
