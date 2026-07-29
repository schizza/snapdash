//! The expanded widget's controls, driven end to end (seam S2).
//!
//! See `tests/harness/mod.rs` for what this seam is and why it exists.

mod harness;

use serde_json::json;
use snapdash::app::Message;
use snapdash::ha::AxisKind;
use snapdash::ui::entity_window::shortcut_hint;

use harness::{
    Harness, dimmable_light_attributes, light_off_attributes, light_on_attributes,
    rgb_light_attributes,
};

const ENTITY: &str = "light.living_room";
/// A second light, whose only colour mode is `rgb`. Colour reaches this
/// one through an 8-bit conversion in both directions, which is the
/// round trip the hue tolerance has to survive.
const RGB_ENTITY: &str = "light.desk_lamp";
/// A third light that can only be dimmed, and so has no colour surface
/// and none of the gestures that go with one.
const DIMMABLE_ENTITY: &str = "light.hallway";

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
    // Both axes of the colour surface, which is what stops the marker
    // being drawn at all.
    assert_eq!(controls[2].value(0), None, "hue");
    assert_eq!(controls[2].value(1), None, "saturation");
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
    // light sits in `color_temp` mode, so the colour surface has a
    // reading even though nobody has set a colour: it is the colour the
    // white the bulb is showing corresponds to.
    assert_eq!(controls[2].value(0), Some(28.391), "hue");
    assert_eq!(controls[2].value(1), Some(65.659), "saturation");
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
/// drag across the colour field from one point to another, and the bulb
/// takes the colour under the finger.
///
/// One `light.turn_on` per position and never one per axis. Hue and
/// saturation reach Home Assistant as the two elements of a single
/// `hs_color`, so a gesture that sent them separately would be sending
/// two colours, the first of them one the user never pointed at.
///
/// Every body is asserted whole rather than for the presence of
/// `hs_color`, because "and no other parameter" is the substance of it.
/// A `light.turn_on` carrying brightness alongside the colour would set
/// both, and a colour surface that quietly also sets brightness is one
/// that fights the brightness slider above it. Leaving the key out is
/// what preserves it: `light.turn_on` without `brightness` keeps
/// whatever the light already had.
#[tokio::test]
async fn dragging_across_the_colour_field_posts_hs_color_and_nothing_else() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([255, 170, 0], [40.0, 100.0], [0.555, 0.422]),
        )
        .await;

    harness
        .send(Message::ColorChanged {
            entity_id: RGB_ENTITY.to_owned(),
            hue: 100.0,
            saturation: 50.0,
        })
        .await;
    harness
        .send(Message::ColorChanged {
            entity_id: RGB_ENTITY.to_owned(),
            hue: 212.0,
            saturation: 85.0,
        })
        .await;
    harness
        .send(Message::ColorReleased {
            entity_id: RGB_ENTITY.to_owned(),
        })
        .await;

    let calls = harness.service_calls().await;

    assert!(!calls.is_empty(), "the drag reached Home Assistant");
    for (path, body) in &calls {
        assert_eq!(path, "/api/services/light/turn_on");
        assert_eq!(
            body.as_object().map(serde_json::Map::len),
            Some(2),
            "the entity and its colour, and nothing else: {body}"
        );
    }

    assert_eq!(
        calls.first().map(|(_, body)| body),
        Some(&json!({ "entity_id": RGB_ENTITY, "hs_color": [100, 50] })),
        "where the drag started"
    );
    assert_eq!(
        calls.last().map(|(_, body)| body),
        Some(&json!({ "entity_id": RGB_ENTITY, "hs_color": [212, 85] })),
        "where it ended, flushed by the release however the throttle fell"
    );
}

/// The other half of the round trip: Home Assistant answers, and the
/// control goes back to showing the house rather than the finger.
///
/// The echo is not the pair Snapdash sent. This light stores 8-bit RGB,
/// so `(212, 85)` is written as `rgb(38, 139, 255)` and read back as
/// `(212.074, 85.098)` - and that is what the control must end up
/// showing, because it is what the bulb is actually doing.
///
/// Nothing here advances a clock. The reconciliation happens on the
/// strength of the echo alone, which is the difference between a control
/// that hands back in one round trip and one that sits on a local value
/// for the two seconds of the settle window on every single drag.
#[tokio::test]
async fn the_echo_from_an_rgb_light_hands_the_colour_straight_back() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([255, 170, 0], [40.0, 100.0], [0.555, 0.422]),
        )
        .await;

    harness
        .send(Message::ColorChanged {
            entity_id: RGB_ENTITY.to_owned(),
            hue: 212.0,
            saturation: 85.0,
        })
        .await;
    harness
        .send(Message::ColorReleased {
            entity_id: RGB_ENTITY.to_owned(),
        })
        .await;
    assert_eq!(
        axis_value(&harness, RGB_ENTITY, AxisKind::Hue),
        Some(212.0),
        "held locally until the echo arrives"
    );

    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([38, 139, 255], [212.074, 85.098], [0.149, 0.156]),
        )
        .await;

    assert!(
        harness.app.pending.is_empty(),
        "one round trip, no settle timeout"
    );
    assert_eq!(
        axis_value(&harness, RGB_ENTITY, AxisKind::Hue),
        Some(212.074),
        "Home Assistant's number, not the one we sent"
    );
    assert_eq!(
        axis_value(&harness, RGB_ENTITY, AxisKind::Saturation),
        Some(85.098)
    );
}

/// The case a numeric hue tolerance cannot survive, and the reason the
/// comparison moved into the control and into RGB.
///
/// At saturation 1 an eight-bit colour barely determines a hue at all:
/// `(212, 1)` is stored as `rgb(252, 254, 255)` and read straight back
/// as hue **200**, twelve degrees from what was sent. As a colour those
/// two are the same three bytes, so comparing colours confirms it in one
/// round trip where comparing degrees could only ever time out.
#[tokio::test]
async fn an_echo_at_the_lowest_saturation_still_confirms() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([255, 170, 0], [40.0, 100.0], [0.555, 0.422]),
        )
        .await;

    harness
        .send(Message::ColorChanged {
            entity_id: RGB_ENTITY.to_owned(),
            hue: 212.0,
            saturation: 1.0,
        })
        .await;
    harness
        .send(Message::ColorReleased {
            entity_id: RGB_ENTITY.to_owned(),
        })
        .await;

    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([252, 254, 255], [200.0, 1.176], [0.32, 0.328]),
        )
        .await;

    assert!(
        harness.app.pending.is_empty(),
        "twelve degrees out and still the colour that was sent"
    );
    assert_eq!(
        axis_value(&harness, RGB_ENTITY, AxisKind::Hue),
        Some(200.0),
        "Home Assistant's number, not the one we sent"
    );
}

/// A colour the user did not pick must not end the interaction. Here
/// the bulb answers with a colour ten degrees away at full saturation,
/// which is four eight-bit levels of blue - far outside anything
/// quantisation can account for.
#[tokio::test]
async fn a_genuinely_different_colour_does_not_confirm() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([255, 170, 0], [40.0, 100.0], [0.555, 0.422]),
        )
        .await;

    harness
        .send(Message::ColorChanged {
            entity_id: RGB_ENTITY.to_owned(),
            hue: 200.0,
            saturation: 100.0,
        })
        .await;
    harness
        .send(Message::ColorReleased {
            entity_id: RGB_ENTITY.to_owned(),
        })
        .await;

    harness
        .state_changed(
            RGB_ENTITY,
            "on",
            rgb_light_attributes([0, 128, 255], [210.0, 100.0], [0.156, 0.163]),
        )
        .await;

    assert!(
        !harness.app.pending.is_empty(),
        "the house is showing a colour nobody asked for"
    );
    assert_eq!(
        axis_value(&harness, RGB_ENTITY, AxisKind::Hue),
        Some(200.0),
        "still the user's colour, until the settle window says otherwise"
    );
}

/// A colour light that is off reports `hs_color` as null, so its colour
/// surface renders as absent (#94) - and is still the way to give the
/// light a colour. `light.turn_on` carrying `hs_color` is also what
/// turns the light on, so there is no "on first, then colour" step to
/// get wrong.
#[tokio::test]
async fn dragging_the_colour_field_on_a_light_that_is_off_turns_it_on() {
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
        .send(Message::ColorChanged {
            entity_id: ENTITY.to_owned(),
            hue: 275.0,
            saturation: 60.0,
        })
        .await;
    harness
        .send(Message::ColorReleased {
            entity_id: ENTITY.to_owned(),
        })
        .await;

    let calls = harness.service_calls().await;

    assert!(!calls.is_empty(), "an absent axis is still operable");
    for (path, body) in &calls {
        assert_eq!(path, "/api/services/light/turn_on");
        assert_eq!(body, &json!({ "entity_id": ENTITY, "hs_color": [275, 60] }));
    }
}

/// A bulb with no colour mode has no colour surface to offer, whatever
/// else it can do. Discovery keys on `supported_color_modes`, so a
/// dimmable white light gets a brightness slider and nothing else.
#[tokio::test]
async fn a_light_with_no_colour_mode_offers_no_colour_surface() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(DIMMABLE_ENTITY, "on", dimmable_light_attributes())
        .await;

    let controls = harness.app.control_views(DIMMABLE_ENTITY);

    assert_eq!(controls.len(), 1, "brightness alone");
    assert_eq!(
        axis_value(&harness, DIMMABLE_ENTITY, AxisKind::Hue),
        None,
        "no hue axis exists to have a value"
    );
    assert_eq!(
        axis_value(&harness, DIMMABLE_ENTITY, AxisKind::Saturation),
        None
    );
}

/// Shift, Alt and the wheel are invisible unless something says they
/// exist, which is what the help affordance in the colour control's
/// label row is for (#100).
///
/// It is asked for here rather than looked at, because a rendered
/// `Element` cannot be read back: [`shortcut_hint`] is the one place
/// deciding which controls get one, and the widget builds its label row
/// from that answer.
///
/// The negative half is the half with teeth. A brightness slider is
/// `iced::widget::slider`, which reads the pointer's horizontal offset
/// and nothing else, so an icon there would promise help that holding
/// Shift cannot give.
#[tokio::test]
async fn only_a_colour_control_offers_the_shortcut_help() {
    let mut harness = Harness::new().await;
    harness
        .state_changed(ENTITY, "on", light_on_attributes())
        .await;
    harness
        .state_changed(DIMMABLE_ENTITY, "on", dimmable_light_attributes())
        .await;

    let colour_bulb = harness.app.control_views(ENTITY);
    let helped: Vec<_> = colour_bulb
        .iter()
        .filter(|view| shortcut_hint(&view.control).is_some())
        .collect();

    assert_eq!(
        helped.len(),
        1,
        "one help affordance on a bulb offering brightness, white and colour"
    );
    assert!(
        helped[0]
            .control
            .axes()
            .any(|axis| axis.kind == AxisKind::Hue),
        "and it is the colour surface that carries it"
    );

    let hint = shortcut_hint(&helped[0].control).expect("the colour control is the helped one");
    for shortcut in ["Shift", "Alt", "Wheel"] {
        assert!(
            hint.contains(shortcut),
            "the hint names {shortcut}, and says: {hint}"
        );
    }

    let dimmable = harness.app.control_views(DIMMABLE_ENTITY);

    assert_eq!(dimmable.len(), 1, "brightness alone");
    assert!(
        dimmable
            .iter()
            .all(|view| shortcut_hint(&view.control).is_none()),
        "a light with no colour surface is promised no shortcuts"
    );
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
