//! Every expectation here is stated from the definition of HSV, by
//! hand, and never by re-running the generator's own arithmetic. A test
//! that computes the answer the way the code computes it can only ever
//! agree with the code, including when the code is wrong.

use super::*;

/// The rendered field, addressable by column and row.
struct Field(Vec<u8>);

impl Field {
    fn render() -> Self {
        Self(super::render())
    }

    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * WIDTH + x) * 4) as usize;
        self.0[offset..offset + 4]
            .try_into()
            .expect("four bytes per pixel")
    }
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const OPAQUE: u8 = 255;

#[test]
fn the_buffer_is_one_opaque_rgba_pixel_per_cell() {
    let field = Field::render();

    assert_eq!(field.0.len(), (WIDTH * HEIGHT * 4) as usize);
    assert!(
        field.0.chunks_exact(4).all(|pixel| pixel[3] == OPAQUE),
        "the field is a solid surface, so no pixel may be see-through"
    );
}

/// Saturation 0 is white at every hue, whatever the hue is: with S = 0
/// the HSV chroma is 0, so R = G = B = V, and V is pinned to 1 across
/// the whole field.
#[test]
fn the_top_row_is_pure_white() {
    let field = Field::render();

    for x in 0..WIDTH {
        assert_eq!(field.at(x, 0), WHITE, "column {x} of the top row");
    }
}

/// At hue 0 the HSV definition reduces to R = V and G = B = V(1 - S).
/// With V = 1 that is a straight wash of red into white, and the column
/// runs from saturation 0 at the top to saturation 100 at the bottom.
#[test]
fn the_hue_zero_column_runs_white_to_red() {
    let field = Field::render();

    assert_eq!(field.at(0, 0), WHITE);
    assert_eq!(field.at(0, HEIGHT - 1), [255, 0, 0, OPAQUE]);

    for y in 0..HEIGHT {
        let saturation = y as f32 / (HEIGHT - 1) as f32;
        let washed = (255.0 * (1.0 - saturation)).round() as u8;

        assert_eq!(
            field.at(0, y),
            [255, washed, washed, OPAQUE],
            "row {y} of the hue-zero column"
        );
    }
}

/// The bottom row is saturation 100, where HSV puts the six vertices of
/// the colour wheel at hues 0, 60, 120, 180, 240 and 300.
///
/// The columns are `round(hue * 255 / 359)`, and they do not land on
/// the vertices exactly, because 359 degrees do not divide evenly into
/// 255 steps. The nearest column is off by at most half a step, which
/// is 359 / 255 / 2 = 0.704 degrees. Inside a 60-degree sector the
/// ramping component covers the full 0..255 range, so 0.704 degrees is
/// worth at most 0.704 / 60 * 255 = 2.99 levels. Hence three.
///
/// That residue is the honest cost of a 256-wide texture, and it is two
/// orders of magnitude smaller than the 5.8-degree error a seven-stop
/// smoothstep gradient would introduce at the same points.
#[test]
fn the_bottom_row_reaches_the_six_pure_hue_vertices() {
    const TOLERANCE: i32 = 3;

    let field = Field::render();
    let bottom = HEIGHT - 1;

    let vertices: [(u32, [u8; 3], &str); 6] = [
        (0, [255, 0, 0], "red at hue 0"),
        (43, [255, 255, 0], "yellow at hue 60"),
        (85, [0, 255, 0], "green at hue 120"),
        (128, [0, 255, 255], "cyan at hue 180"),
        (170, [0, 0, 255], "blue at hue 240"),
        (213, [255, 0, 255], "magenta at hue 300"),
    ];

    for (x, expected, name) in vertices {
        let actual = field.at(x, bottom);

        assert_eq!(actual[3], OPAQUE, "{name}");

        for channel in 0..3 {
            let drift = i32::from(actual[channel]) - i32::from(expected[channel]);

            assert!(
                drift.abs() <= TOLERANCE,
                "{name}: channel {channel} is {} but should be within {TOLERANCE} of {}",
                actual[channel],
                expected[channel]
            );
        }
    }
}

/// Two points worked out longhand from HSV, one in each half of the
/// saturation axis and in different sectors of the wheel, so that a
/// generator that got a sector boundary or an axis direction wrong
/// cannot slip past the white row and the pure-hue row.
#[test]
fn interior_pixels_match_the_hsv_definition() {
    let field = Field::render();

    // Column 32, row 112. H = 32 * 359 / 255 = 45.051, S = 112 / 127 =
    // 0.88189, V = 1. H is in the first sector, so C = SV = 0.88189,
    // X = C(1 - |H/60 mod 2 - 1|) = 0.88189 * 0.75085 = 0.66217, and
    // m = V - C = 0.11811. (R, G, B) = (C, X, 0) + m = (1, 0.78028,
    // 0.11811), which is (255, 199, 30) at eight bits.
    assert_eq!(field.at(32, 112), [255, 199, 30, OPAQUE]);

    // Column 192, row 16. H = 192 * 359 / 255 = 270.306, S = 16 / 127 =
    // 0.12598, V = 1. H is in the fifth sector, so C = 0.12598,
    // X = C(1 - |H/60 mod 2 - 1|) = 0.12598 * 0.50510 = 0.06363, and
    // m = 0.87402. (R, G, B) = (X, 0, C) + m = (0.93765, 0.87402, 1),
    // which is (239, 223, 255) at eight bits.
    assert_eq!(field.at(192, 16), [239, 223, 255, OPAQUE]);
}

/// The whole point of computing the field once: every widget and every
/// size preset draws the same handle, so the renderer uploads one
/// texture and scales it, rather than one per widget.
#[test]
fn one_texture_serves_every_caller() {
    assert_eq!(handle().id(), handle().id());
}
