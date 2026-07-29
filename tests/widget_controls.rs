//! The expanded widget's controls, driven end to end (seam S2).
//!
//! See `tests/harness/mod.rs` for what this seam is and why it exists.

mod harness;

use serde_json::json;
use snapdash::app::Message;
use snapdash::ha::AxisKind;

use harness::{Harness, light_off_attributes, light_on_attributes, rgb_light_attributes};

const ENTITY: &str = "light.living_room";
/// A second light, whose only colour mode is `rgb`. Colour reaches this
/// one through an 8-bit conversion in both directions, which is the
/// round trip the hue tolerance has to survive.
const RGB_ENTITY: &str = "light.desk_lamp";

/// What the control driving `kind` currently shows, found by axis rather
/// than by position so a test never encodes the order the entity happens
/// to offer its controls in.
fn axis_value(harness: &Harness, entity_id: &str, kind: AxisKind) -> Option<f32> {
    harness
        .app
        .control_views(entity_id)
        .iter()
        .find_map(|view| {
            let index = view.control.axes().position(|axis| axis.kind == kind)?;
            view.value(index)
        })
}

/// Home Assistant nulls every colour attribute of a light that is off.
/// Reading that as zero is a different claim from reading it as absent,
/// and only one of them is true: the bulb is not sitting at 0%
/// brightness, it has no brightness at all right now.
#[tokio::test]
async fn an_axis_home_assistant_reports_as_null_offers_no_value() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(ENTITY, "off", light_off_attributes())
        .await;

    let controls = harness.app.control_views(ENTITY);

    // The bulb is still dimmable and still supports both colour temperature
    // and colour - those are static capabilities - so every control is
    // offered.
    assert_eq!(controls.len(), 3, "an off light still offers its controls");
    assert_eq!(controls[0].value(0), None, "brightness");
    assert_eq!(controls[1].value(0), None, "colour temperature");
    assert_eq!(controls[2].value(0), None, "hue");
}

/// The absent value is a reading of the present, not a gap in the
/// record. The moment Home Assistant reports a number again, that is
/// what the control shows.
#[tokio::test]
async fn the_value_comes_back_once_home_assistant_reports_one_again() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(ENTITY, "off", light_off_attributes())
        .await;
    harness
        .state_changed(ENTITY, "on", light_on_attributes())
        .await;

    let controls = harness.app.control_views(ENTITY);

    assert_eq!(controls[0].value(0), Some(172.0), "brightness");
    assert_eq!(controls[1].value(0), Some(2703.0), "colour temperature");
    // Home Assistant derives `hs_color` from the kelvin value while the
    // light sits in `color_temp` mode, so the hue control has a reading
    // even though nobody has set a colour: it is the colour the white
    // the bulb is showing corresponds to.
    assert_eq!(controls[2].value(0), Some(28.391), "hue");
}

/// Rendering an axis as absent must not make it inert. Grabbing a
/// control with no value is how the user gives it one, and for a light
/// that is off the `light.turn_on` it produces is also what turns the
/// light on - there is no separate "on first, then set" step to get
/// wrong.
#[tokio::test]
async fn dragging_an_axis_with_no_value_still_sets_it_and_turns_the_light_on() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(ENTITY, "off", light_off_attributes())
        .await;
    assert_eq!(
        harness.app.control_views(ENTITY)[0].value(0),
        None,
        "the axis under test has no value to start from"
    );

    harness
        .send(Message::ControlValueChanged {
            entity_id: ENTITY.to_owned(),
            axis: AxisKind::Brightness,
            value: 128.0,
        })
        .await;
    harness
        .send(Message::ControlReleased {
            entity_id: ENTITY.to_owned(),
            axis: AxisKind::Brightness,
        })
        .await;

    let calls = harness.service_calls().await;

    assert!(!calls.is_empty(), "a valueless axis is still operable");
    for (path, body) in &calls {
        assert_eq!(path, "/api/services/light/turn_on");
        assert_eq!(body, &json!({ "entity_id": ENTITY, "brightness": 128 }));
    }
}

/// The whole path, stated the way a user would: expand a colour light,
/// drag the hue control, and the bulb changes colour.
///
/// The body is asserted exactly rather than for the presence of
/// `hs_color`, because "and no other parameter" is the substance of it.
/// A `light.turn_on` carrying brightness alongside the colour would set
/// both, and a hue control that quietly also sets brightness is a hue
/// control that fights the brightness slider above it. Leaving the key
/// out is what preserves it: `light.turn_on` without `brightness` keeps
/// whatever the light already had.
#[tokio::test]
async fn dragging_hue_posts_hs_color_and_nothing_else() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([255, 170, 0], [40.0, 100.0], [0.555, 0.422]),
        )
        .await;

    harness
        .send(Message::ControlValueChanged {
            entity_id: RGB_ENTITY.to_owned(),
            axis: AxisKind::Hue,
            value: 200.0,
        })
        .await;
    harness
        .send(Message::ControlReleased {
            entity_id: RGB_ENTITY.to_owned(),
            axis: AxisKind::Hue,
        })
        .await;

    let calls = harness.service_calls().await;

    assert!(!calls.is_empty(), "the drag reached Home Assistant");
    for (path, body) in &calls {
        assert_eq!(path, "/api/services/light/turn_on");
        assert_eq!(
            body,
            &json!({ "entity_id": RGB_ENTITY, "hs_color": [200, 100] })
        );
    }
}

/// The other half of the round trip: Home Assistant answers, and the
/// control goes back to showing the house rather than the finger.
///
/// The echo is not the number Snapdash sent. This light stores 8-bit
/// RGB, so hue 200 is written as `rgb(0, 169, 255)` and read back as
/// 200.235 - and that is the value the control must end up showing,
/// because it is what the bulb is actually doing.
///
/// Nothing here advances a clock. The reconciliation happens on the
/// strength of the echo alone, which is the difference between a control
/// that hands back in one round trip and one that sits on a local value
/// for the two seconds of the settle window on every single drag.
#[tokio::test]
async fn the_echo_from_an_rgb_light_hands_the_hue_straight_back() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([255, 170, 0], [40.0, 100.0], [0.555, 0.422]),
        )
        .await;

    harness
        .send(Message::ControlValueChanged {
            entity_id: RGB_ENTITY.to_owned(),
            axis: AxisKind::Hue,
            value: 200.0,
        })
        .await;
    harness
        .send(Message::ControlReleased {
            entity_id: RGB_ENTITY.to_owned(),
            axis: AxisKind::Hue,
        })
        .await;
    assert_eq!(
        axis_value(&harness, RGB_ENTITY, AxisKind::Hue),
        Some(200.0),
        "held locally until the echo arrives"
    );

    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([0, 169, 255], [200.235, 100.0], [0.144, 0.202]),
        )
        .await;

    assert!(
        harness.app.pending.is_empty(),
        "one round trip, no settle timeout"
    );
    assert_eq!(
        axis_value(&harness, RGB_ENTITY, AxisKind::Hue),
        Some(200.235),
        "Home Assistant's number, not the one we sent"
    );
}

/// A colour light that is off reports `hs_color` as null, so its hue
/// control renders as absent (#94) - and is still the way to give the
/// light a colour. `light.turn_on` carrying `hs_color` is also what
/// turns the light on, so there is no "on first, then colour" step to
/// get wrong.
#[tokio::test]
async fn dragging_hue_on_a_light_that_is_off_turns_it_on() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(ENTITY, "off", light_off_attributes())
        .await;
    assert_eq!(
        axis_value(&harness, ENTITY, AxisKind::Hue),
        None,
        "an off light has no colour to show"
    );

    harness
        .send(Message::ControlValueChanged {
            entity_id: ENTITY.to_owned(),
            axis: AxisKind::Hue,
            value: 275.0,
        })
        .await;
    harness
        .send(Message::ControlReleased {
            entity_id: ENTITY.to_owned(),
            axis: AxisKind::Hue,
        })
        .await;

    let calls = harness.service_calls().await;

    assert!(!calls.is_empty(), "an absent axis is still operable");
    for (path, body) in &calls {
        assert_eq!(path, "/api/services/light/turn_on");
        assert_eq!(
            body,
            &json!({ "entity_id": ENTITY, "hs_color": [275, 100] })
        );
    }
}

/// While the user drives it, the control shows the local value even
/// though Home Assistant is still reporting null for that axis: the
/// echo lags the finger, and an axis being dragged is by definition
/// being driven.
#[tokio::test]
async fn a_dragged_axis_shows_the_value_the_user_is_setting() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(ENTITY, "off", light_off_attributes())
        .await;

    harness
        .send(Message::ControlValueChanged {
            entity_id: ENTITY.to_owned(),
            axis: AxisKind::Brightness,
            value: 200.0,
        })
        .await;

    assert_eq!(harness.app.control_views(ENTITY)[0].value(0), Some(200.0));
}
