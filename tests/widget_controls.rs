//! The expanded widget's controls, driven end to end (seam S2).
//!
//! See `tests/harness/mod.rs` for what this seam is and why it exists.

mod harness;

use serde_json::json;
use snapdash::app::Message;
use snapdash::ha::AxisKind;

use harness::{Harness, light_off_attributes, light_on_attributes};

const ENTITY: &str = "light.living_room";

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

    // The bulb is still dimmable and still supports colour temperature -
    // those are static capabilities - so both controls are offered.
    assert_eq!(controls.len(), 2, "an off light still offers its controls");
    assert_eq!(controls[0].value(0), None, "brightness");
    assert_eq!(controls[1].value(0), None, "colour temperature");
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
