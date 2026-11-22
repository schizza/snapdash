//! Seam S4: the pointer-to-colour mapping, on its own.
//!
//! Every expectation is stated from the field's geometry - hue across
//! from 0 to 359, saturation down from 0 to 100 - and never by running
//! the mapping's own arithmetic against itself.
//!
//! One test at the bottom is not S4 at all: it drives `Widget::update`
//! to say that a scroll event reaches the widget. That belongs at S2 by
//! rights, but S2 drives `Snapdash::update` with messages and a widget
//! event has no message to arrive as - the whole question is whether the
//! widget turns one into the other. So it is asked here, of the widget,
//! with a bare tree and no window.

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

/// Shift locks the axis that has moved less since the press, and the
/// locked axis holds the value the press gave it rather than freezing
/// wherever the pointer happened to cross.
///
/// Both directions are checked from one press, because a lock that only
/// ever holds saturation would pass a test that only ever drags sideways
/// - and sideways is the drag this feature is mostly wanted for.
#[test]
fn shift_locks_the_axis_that_has_moved_less_since_the_press() {
    let bounds = field();
    // A quarter across and halfway down: hue 90, saturation 50.
    let pressed_at = Point::new(bounds.x + 43.0, bounds.y + 43.0);

    // Mostly sideways, to halfway across and 60% down.
    let sideways = Point::new(bounds.x + 86.0, bounds.y + 51.6);

    assert_eq!(
        colour_at(bounds, sideways, Modifiers::default(), pressed_at),
        (180.0, 60.0),
        "unlocked, the pointer names both axes"
    );
    assert_eq!(
        colour_at(bounds, sideways, Modifiers::SHIFT, pressed_at),
        (180.0, 50.0),
        "saturation moved less, so it stays where the press put it"
    );

    // Mostly downwards, to a quarter across plus a little and 75% down.
    let downwards = Point::new(bounds.x + 47.3, bounds.y + 64.5);

    assert_eq!(
        colour_at(bounds, downwards, Modifiers::default(), pressed_at),
        (99.0, 75.0),
        "unlocked, the pointer names both axes"
    );
    assert_eq!(
        colour_at(bounds, downwards, Modifiers::SHIFT, pressed_at),
        (90.0, 75.0),
        "hue moved less, so it stays where the press put it"
    );
}

/// Saturation reaches exactly 0 and exactly 100 without the pointer
/// having to reach the very edge, so "keep this fully saturated while I
/// change the hue" does not need a steady hand.
///
/// The magnet is stated in points of the field rather than as a fraction
/// of it, so the assertions here are in points too: two points inside
/// the edge snaps, and a little further in does not - the axis still has
/// to be able to say 5 and 95.
#[test]
fn saturation_reaches_exactly_zero_and_one_hundred_near_the_edges() {
    let bounds = field();
    let at = |y: f32| {
        colour_at(
            bounds,
            Point::new(bounds.x + 86.0, y),
            Modifiers::default(),
            Point::ORIGIN,
        )
    };

    assert_eq!(
        at(bounds.y + 2.0),
        (180.0, 0.0),
        "two points below the top edge is white, and the hue is untouched"
    );
    assert_eq!(
        at(bounds.y + bounds.height - 2.0),
        (180.0, 100.0),
        "two points above the bottom edge is fully saturated"
    );

    // Past the magnet the axis is itself again. Without this the magnet
    // would be indistinguishable from an axis that simply cannot express
    // the saturations near its ends.
    assert_eq!(
        at(bounds.y + 4.3),
        (180.0, 5.0),
        "just past the magnet, saturation is read off the field again"
    );
    assert_eq!(at(bounds.y + bounds.height - 4.3), (180.0, 95.0));
}

/// Alt keeps a quarter of the movement *since the press*, not a quarter
/// of the position in the field. A quarter of the position would jump
/// the colour to somewhere near the left edge the moment the key went
/// down, which is the opposite of a fine adjustment.
///
/// The second half of this is the marker detaching from the pointer,
/// which is what every tool with a fine-drag modifier does and what
/// makes the modifier legible: the ring stops following the hand, so the
/// hand can see it is being listened to differently.
#[test]
fn alt_scales_movement_to_a_quarter_of_it_relative_to_the_press() {
    let bounds = field();
    // A quarter across and halfway down: hue 90, saturation 50.
    let pressed_at = Point::new(bounds.x + 43.0, bounds.y + 43.0);
    // Half the field to the right and 40% of it down.
    let cursor = Point::new(bounds.x + 129.0, bounds.y + 77.4);

    assert_eq!(
        colour_at(bounds, cursor, Modifiers::default(), pressed_at),
        (269.0, 90.0),
        "unscaled, the colour is read straight off the pointer"
    );
    assert_eq!(
        colour_at(bounds, cursor, Modifiers::ALT, pressed_at),
        (135.0, 60.0),
        "a quarter of the way from the press towards the pointer"
    );

    // The marker is drawn from the colour, so a colour that is no longer
    // the pointer's puts the marker somewhere the pointer is not. Stated
    // against the geometry: an eighth of the field to the right of the
    // press and a tenth of it down.
    let marker = position_of(bounds, 135.0, 60.0);

    assert!(
        (marker.x - (pressed_at.x + 21.5)).abs() <= bounds.width / MAX_HUE,
        "{marker:?}"
    );
    assert!(
        (marker.y - (pressed_at.y + 8.6)).abs() <= bounds.height / MAX_SATURATION,
        "{marker:?}"
    );
}

/// The wheel nudges hue a step at a time, and Shift moves it onto
/// saturation. It is the only way to reach a colour the field cannot
/// resolve under a pointer at all - the eight saturations the magnets
/// give up, and any single degree of hue at the Small preset.
#[test]
fn the_wheel_nudges_hue_by_a_step_and_shift_and_the_wheel_nudges_saturation() {
    assert_eq!(
        nudged((100.0, 50.0), 1.0, Modifiers::default()),
        (101.0, 50.0),
        "up the wheel is up the axis"
    );
    assert_eq!(
        nudged((100.0, 50.0), -1.0, Modifiers::default()),
        (99.0, 50.0)
    );
    assert_eq!(
        nudged((100.0, 50.0), 1.0, Modifiers::SHIFT),
        (100.0, 51.0),
        "shift moves the same nudge onto the other axis"
    );
    assert_eq!(nudged((100.0, 50.0), -1.0, Modifiers::SHIFT), (100.0, 49.0));

    // The ends hold rather than wrapping. Hue is an angle, so wrapping
    // is arithmetically defensible and would still be wrong here: the
    // axis stops one degree short of the full turn precisely so that it
    // stays a plain range, and a wheel that jumped from the far red back
    // to the near one would be the only place in the widget where it
    // did not.
    assert_eq!(
        nudged((MAX_HUE, MAX_SATURATION), 1.0, Modifiers::default()),
        (MAX_HUE, MAX_SATURATION)
    );
    assert_eq!(nudged((0.0, 0.0), -1.0, Modifiers::SHIFT), (0.0, 0.0));

    // Home Assistant's own reading is not on a step - a light in
    // `color_temp` mode reports the colour its white corresponds to, to
    // three decimal places - and a nudge has to land on one, or the
    // readout, the pending value and the service call would go back to
    // being three roundings of one number. Both axes round, because they
    // travel as one `hs_color` pair however few of them moved.
    assert_eq!(
        nudged((28.391, 65.659), 1.0, Modifiers::default()),
        (29.0, 66.0)
    );
}

/// A wheel and a trackpad describe the same gesture in different units,
/// and the field has to answer both in steps.
#[test]
fn a_wheel_s_lines_and_a_trackpad_s_pixels_both_come_out_in_notches() {
    let lines = |x, y| notches(mouse::ScrollDelta::Lines { x, y });
    let pixels = |x, y| notches(mouse::ScrollDelta::Pixels { x, y });

    assert_eq!(lines(0.0, 1.0), 1.0, "one detent of a mouse wheel");
    assert_eq!(lines(0.0, -2.0), -2.0);

    assert_eq!(
        pixels(0.0, PIXELS_PER_NOTCH),
        1.0,
        "a line's worth of a precise device is a detent"
    );
    assert!(
        pixels(0.0, 4.0) > 0.0 && pixels(0.0, 4.0) < 1.0,
        "a few pixels is a fraction of one, for the caller to accumulate"
    );

    // macOS puts a shift-held wheel on the horizontal axis, so the
    // horizontal component is not noise to be discarded - on that
    // platform it is the whole of the shift-and-wheel gesture.
    assert_eq!(lines(1.0, 0.0), 1.0, "a wheel macOS has turned sideways");
    // And when there is a vertical component it wins, so the sideways
    // wobble of a two-finger swipe cannot cancel the swipe out.
    assert_eq!(lines(-1.0, 2.0), 2.0);
}

/// The modifiers compose: Shift and Alt together lock and refine at
/// once, and the composed answer is neither of the two taken alone.
///
/// This is the assertion that says the two are edits of the same point
/// rather than two behaviours with a third one wired up between them.
/// The same press and the same cursor as the two tests above, so all
/// four answers can be read against each other.
#[test]
fn shift_and_alt_together_lock_and_refine_at_once() {
    let bounds = field();
    let pressed_at = Point::new(bounds.x + 43.0, bounds.y + 43.0);
    let cursor = Point::new(bounds.x + 129.0, bounds.y + 77.4);

    // Sideways by half the field against downwards by 40% of it, so
    // saturation is the axis that moved less and the lock holds it at
    // the press. Alt takes the hue a quarter of the way instead of all
    // of it.
    assert_eq!(
        colour_at(
            bounds,
            cursor,
            Modifiers::SHIFT | Modifiers::ALT,
            pressed_at
        ),
        (135.0, 50.0)
    );

    // Neither modifier alone, and not the plain answer either.
    assert_eq!(
        colour_at(bounds, cursor, Modifiers::SHIFT, pressed_at),
        (269.0, 50.0)
    );
    assert_eq!(
        colour_at(bounds, cursor, Modifiers::ALT, pressed_at),
        (135.0, 60.0)
    );
    assert_eq!(
        colour_at(bounds, cursor, Modifiers::default(), pressed_at),
        (269.0, 90.0)
    );
}

/// Letting a modifier go part way through a drag restores plain
/// behaviour on the very next pointer move, with nothing to unwind.
///
/// The point of the test is the *order*: the same cursor is asked for
/// three times with the modifiers changing underneath it, and the third
/// answer is the first one. That is what says the mapping keeps no
/// record of having been constrained - the failure it guards against is
/// a lock that latches, which reads to a user as the control having
/// stopped responding with no way to tell why, and which is the hidden
/// in-gesture classifier `docs/adr/0001-widget-interaction-model.md`
/// exists to keep out.
#[test]
fn releasing_a_modifier_mid_drag_restores_plain_behaviour_at_once() {
    let bounds = field();
    let pressed_at = Point::new(bounds.x + 43.0, bounds.y + 43.0);
    let cursor = Point::new(bounds.x + 129.0, bounds.y + 77.4);

    let plain = colour_at(bounds, cursor, Modifiers::default(), pressed_at);

    assert_eq!(plain, (269.0, 90.0), "where the pointer actually is");

    for held in [
        Modifiers::SHIFT,
        Modifiers::ALT,
        Modifiers::SHIFT | Modifiers::ALT,
    ] {
        assert_ne!(
            colour_at(bounds, cursor, held, pressed_at),
            plain,
            "{held:?} did nothing, so letting it go proves nothing"
        );
        assert_eq!(
            colour_at(bounds, cursor, Modifiers::default(), pressed_at),
            plain,
            "the frame after {held:?} was let go"
        );
    }
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

/// What one wheel event made the widget say.
#[derive(Debug, Clone, PartialEq)]
enum Said {
    Colour(f32, f32),
    Released,
}

/// Deliver `events` to a colour field showing `colour`, and collect what
/// it published.
///
/// A widget can be driven without a window: the tree is two public
/// fields the widget itself fills in, the layout is one node placed
/// where `field()` says, and `()` is a renderer and `clipboard::Null` a
/// clipboard. That is the whole harness, which is why this one question
/// is worth asking here rather than being left to a human's hands.
///
/// `()` is a renderer only under `debug_assertions`, which is where
/// iced's null implementation of the trait lives, so this compiles under
/// `cargo test` and not under `cargo test --release`. CI runs the
/// former.
fn published(colour: Option<(f32, f32)>, events: &[iced::Event], cursor: Point) -> Vec<Said> {
    let bounds = field();
    let mut widget = colour_field(colour, bounds.height, style(), Said::Colour, Said::Released);

    let mut tree = widget::Tree {
        tag: <_ as Widget<Said, (), ()>>::tag(&widget),
        state: <_ as Widget<Said, (), ()>>::state(&widget),
        children: Vec::new(),
    };
    let node = layout::Node::new(bounds.size()).move_to(bounds.position());
    let mut messages = Vec::new();

    for event in events {
        let mut shell = iced::advanced::Shell::new(&mut messages);

        <_ as Widget<Said, (), ()>>::update(
            &mut widget,
            &mut tree,
            event,
            Layout::new(&node),
            mouse::Cursor::Available(cursor),
            &(),
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &bounds,
        );
    }

    messages
}

/// Any style at all: nothing under test here draws.
fn style() -> Style {
    Style {
        fill: Color::BLACK,
        marker: Color::WHITE,
        marker_shadow: Color::BLACK,
        opacity: 1.0,
    }
}

/// The one thing about the wheel the pure functions cannot answer: that
/// the event reaches the widget, over the field and not elsewhere, and
/// comes back out as a change the application can act on.
///
/// The release is published with it because a nudge is a whole
/// interaction rather than the middle of one. `app::pending` throttles
/// what a gesture puts on the wire and flushes it on release; a change
/// with no release would leave the last nudge to be swallowed by the
/// throttle and the pending value with no settle deadline, which is a
/// widget showing a colour the house never received and nothing left to
/// correct it.
#[test]
fn a_wheel_over_the_field_reaches_the_control() {
    let bounds = field();
    let over = Point::new(bounds.x + 86.0, bounds.y + 43.0);
    let wheel = |x, y| {
        iced::Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x, y },
        })
    };

    assert_eq!(
        published(Some((100.0, 50.0)), &[wheel(0.0, 1.0)], over),
        vec![Said::Colour(101.0, 50.0), Said::Released],
    );

    // Off the field the wheel belongs to whatever is behind it. A colour
    // surface that answered a scroll anywhere in the window would change
    // the light while the user was scrolling something else.
    assert_eq!(
        published(
            Some((100.0, 50.0)),
            &[wheel(0.0, 1.0)],
            Point::new(bounds.x - 10.0, bounds.y - 10.0)
        ),
        vec![],
    );

    // Shift reaches it too, which is the only place the widget's own
    // record of the modifiers is exercised.
    assert_eq!(
        published(
            Some((100.0, 50.0)),
            &[
                iced::Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::SHIFT)),
                wheel(0.0, 1.0)
            ],
            over
        ),
        vec![Said::Colour(100.0, 51.0), Said::Released],
    );

    // A field with no colour has nothing to nudge. Pressing is how an
    // absent axis is given a value, because a press names one outright;
    // a wheel can only offer a number to add to one that is not there.
    assert_eq!(published(None, &[wheel(0.0, 1.0)], over), vec![]);
}

/// A precise device reports the distance the fingers moved, not
/// detents, so its events accumulate until they are worth a step
/// instead of each being one.
///
/// Without this a trackpad would run away: it emits at the refresh rate,
/// so a flick over the field would be a hundred steps and a hundred
/// service calls.
#[test]
fn a_trackpad_s_pixels_accumulate_into_steps() {
    let bounds = field();
    let over = Point::new(bounds.x + 86.0, bounds.y + 43.0);
    let creep = iced::Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels {
            x: 0.0,
            y: PIXELS_PER_NOTCH / 4.0,
        },
    });

    assert_eq!(
        published(Some((100.0, 50.0)), std::slice::from_ref(&creep), over),
        vec![],
        "a quarter of a notch is not a step yet"
    );

    let four = [creep.clone(), creep.clone(), creep.clone(), creep];

    assert_eq!(
        published(Some((100.0, 50.0)), &four, over),
        vec![Said::Colour(101.0, 50.0), Said::Released],
        "four quarters are one step, and one step only"
    );
}
