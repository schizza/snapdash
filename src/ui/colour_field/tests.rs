//! Seam S4: the pointer-to-colour mapping, on its own.
//!
//! Every expectation is stated from the field's geometry - hue across
//! from 0 to 359, saturation down from 0 to 100 - and never by running
//! the mapping's own arithmetic against itself.

use super::*;

/// A Normal-preset field, placed away from the origin so a mapping that
/// forgot to subtract the field's own position cannot pass.
fn field() -> Rectangle {
    Rectangle::new(Point::new(24.0, 90.0), Size::new(172.0, 86.0))
}

/// The four corners and the centre, which is where a flipped axis, a
/// swapped pair or an off-by-one in the extents shows up.
///
/// The centre is 180 and not 179.5 because the wheel stops one degree
/// short of the full turn - 359 has no exact midpoint in whole degrees,
/// and the axis carries whole degrees.
#[test]
fn the_corners_and_the_centre_name_the_colours_the_field_shows_there() {
    let bounds = field();
    let at = |x: f32, y: f32| {
        colour_at(
            bounds,
            Point::new(x, y),
            Modifiers::default(),
            Point::ORIGIN,
        )
    };

    assert_eq!(at(bounds.x, bounds.y), (0.0, 0.0), "top left is white");
    assert_eq!(
        at(bounds.x + bounds.width, bounds.y),
        (MAX_HUE, 0.0),
        "top right is the far end of the wheel, still white"
    );
    assert_eq!(
        at(bounds.x, bounds.y + bounds.height),
        (0.0, MAX_SATURATION),
        "bottom left is pure red"
    );
    assert_eq!(
        at(bounds.x + bounds.width, bounds.y + bounds.height),
        (MAX_HUE, MAX_SATURATION),
        "bottom right is the far end, fully saturated"
    );
    assert_eq!(
        at(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0
        ),
        (180.0, 50.0),
        "the centre of the field"
    );
}

/// The whole of "the drag keeps tracking when the pointer leaves the
/// field". A cursor past an edge is not ignored and does not wrap; it
/// names the colour on the edge it went past, so a fast drag out of the
/// widget leaves the light on the last colour the user actually pointed
/// at rather than wherever the gesture happened to be abandoned.
#[test]
fn a_cursor_outside_the_field_clamps_to_its_edge() {
    let bounds = field();
    let at = |x: f32, y: f32| {
        colour_at(
            bounds,
            Point::new(x, y),
            Modifiers::default(),
            Point::ORIGIN,
        )
    };

    assert_eq!(at(bounds.x - 400.0, bounds.y - 400.0), (0.0, 0.0));
    assert_eq!(
        at(
            bounds.x + bounds.width + 400.0,
            bounds.y + bounds.height + 400.0
        ),
        (MAX_HUE, MAX_SATURATION)
    );
    // One axis out and the other in: the axis still inside must not be
    // dragged to an edge with it.
    assert_eq!(
        at(bounds.x - 400.0, bounds.y + bounds.height / 2.0),
        (0.0, 50.0)
    );
}

/// The axes carry whole steps, so the mapping hands back whole steps.
/// Anything else would leave the readout rounding one way, the pending
/// value holding another and the service call truncating to a third.
#[test]
fn every_point_in_the_field_names_a_whole_step() {
    let bounds = field();

    for x in 0..=bounds.width as u32 {
        for y in 0..=bounds.height as u32 {
            let (hue, saturation) = colour_at(
                bounds,
                Point::new(bounds.x + x as f32, bounds.y + y as f32),
                Modifiers::default(),
                Point::ORIGIN,
            );

            assert_eq!(hue, hue.trunc(), "hue at ({x}, {y})");
            assert_eq!(saturation, saturation.trunc(), "saturation at ({x}, {y})");
            assert!((0.0..=MAX_HUE).contains(&hue), "hue at ({x}, {y})");
            assert!(
                (0.0..=MAX_SATURATION).contains(&saturation),
                "saturation at ({x}, {y})"
            );
        }
    }
}

/// A layout that gave the field no room must not produce a NaN, because
/// a NaN here travels all the way to a service call.
#[test]
fn a_field_with_no_extent_names_a_colour_anyway() {
    let bounds = Rectangle::new(Point::ORIGIN, Size::new(0.0, 0.0));

    assert_eq!(
        colour_at(
            bounds,
            Point::new(10.0, 10.0),
            Modifiers::default(),
            Point::ORIGIN
        ),
        (0.0, 0.0)
    );
}

/// The marker has to land back where the pointer was, or it would drift
/// away from the colour it names as the user drags.
#[test]
fn the_marker_sits_where_the_pointer_was() {
    let bounds = field();

    for (x, y) in [
        (0.0, 0.0),
        (bounds.width, 0.0),
        (0.0, bounds.height),
        (bounds.width, bounds.height),
        (bounds.width / 2.0, bounds.height / 2.0),
    ] {
        let pointer = Point::new(bounds.x + x, bounds.y + y);
        let (hue, saturation) = colour_at(bounds, pointer, Modifiers::default(), Point::ORIGIN);
        let marker = position_of(bounds, hue, saturation);

        // Within half a step of a pixel: the colour is quantised to whole
        // degrees and whole percent, and the marker sits on the quantised
        // colour rather than between two of them.
        assert!(
            (marker.x - pointer.x).abs() <= bounds.width / MAX_HUE,
            "x: {marker:?} against {pointer:?}"
        );
        assert!(
            (marker.y - pointer.y).abs() <= bounds.height / MAX_SATURATION,
            "y: {marker:?} against {pointer:?}"
        );
    }
}
