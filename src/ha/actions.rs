//! Outbound service calls to Home Assistant.
//!
//! Actionable widgets (issue #81) let the user act on a widget without
//! opening Home Assistant. This module owns two separate concerns that
//! phase 1 was able to conflate and phase 2 cannot:
//!
//! - **Discovery** - [`Capabilities`], "what can this entity do?".
//!   Answered from the entity's *attributes*, because the domain alone
//!   does not know it: not every `light` is dimmable and not every
//!   `cover` can be positioned.
//! - **Invocation** - [`ActionKind`], "what do I send?". A concrete
//!   service call, optionally carrying a value.
//!
//! ## Why REST and not the existing WebSocket?
//!
//! Recorded in `docs/adr/0003-service-calls-stay-on-rest.md`. In short:
//! sends are throttled, so the peak is ~5 calls/sec and only during a
//! drag, and the per-call cost that motivated the idea was a fresh
//! `reqwest::Client` per call rather than HTTP itself. The migration
//! would put outbound sender plumbing, request-id correlation and
//! reconnect re-attach into the live state feed, which is the one
//! subsystem that must not break.

use serde_json::Value;

use crate::ha::types::{EntityState, HaError};
use crate::ui::format::domain;

/// Domains that may be pinned as a widget.
///
/// Candidacy is deliberately domain-based, not capability-based: an
/// entity is a valid widget if its state is worth *displaying*, whether
/// or not it can also be acted on. A non-dimmable light and a
/// read-only sensor are both perfectly good widgets.
const WIDGET_DOMAINS: &[&str] = &[
    "sensor",
    "binary_sensor",
    "switch",
    "light",
    "scene",
    "script",
    "input_boolean",
    "climate",
    "cover",
];

/// One process-wide HTTP client, so service calls reuse a pooled
/// keep-alive connection.
///
/// Previously every call constructed `reqwest::Client::new()`, which
/// builds a fresh connection pool and therefore paid for a new TCP and
/// TLS handshake per tap. That per-call cost - not HTTP itself - was
/// what made a WebSocket migration look necessary for phase 2. `Client`
/// is internally `Arc`'d and explicitly designed to be reused.
static HTTP: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn http() -> &'static reqwest::Client {
    HTTP.get_or_init(reqwest::Client::new)
}

/// A concrete Home Assistant service call.
///
/// Phase 1 variants are zero-argument: HA decides the resulting state.
/// Phase 2 variants carry the value being set. The enum stays `Copy`
/// because every payload is a scalar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionKind {
    /// `switch.toggle` - HA flips it based on current state.
    ToggleSwitch,
    /// `light.toggle` - on/off only, no brightness or color.
    ToggleLight,
    /// `scene.turn_on` - a scene is stateless: "activate this scene".
    TriggerScene,
    /// `script.turn_on` - starts the script from the top.
    TriggerScript,
    /// `input_boolean.toggle` - HA's user-defined boolean helper.
    ToggleInputBoolean,
    /// `light.turn_on` with `brightness` (0-255).
    SetBrightness(u8),
    /// `light.turn_on` with `color_temp_kelvin`.
    SetColorTemp(u32),
    /// `climate.set_temperature` with `temperature`.
    SetTemperature(f32),
    /// `cover.set_cover_position` with `position` (0-100).
    SetPosition(u8),
}

/// The complete wire mapping for one action: the endpoint plus any
/// parameters beyond `entity_id`.
///
/// Kept as a single value so the whole mapping lives in one auditable
/// place - `wire_mapping_is_stable` asserts against it, and any change
/// here changes what Snapdash puts on the wire.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceCall {
    pub domain: &'static str,
    pub service: &'static str,
    /// Extra body fields, merged alongside `entity_id`.
    pub params: Vec<(&'static str, Value)>,
}

impl ActionKind {
    /// The `(domain, service)` endpoint and parameters to POST.
    pub fn service_call(self) -> ServiceCall {
        let (domain, service, params) = match self {
            Self::ToggleSwitch => ("switch", "toggle", vec![]),
            Self::ToggleLight => ("light", "toggle", vec![]),
            Self::TriggerScene => ("scene", "turn_on", vec![]),
            Self::TriggerScript => ("script", "turn_on", vec![]),
            Self::ToggleInputBoolean => ("input_boolean", "toggle", vec![]),
            Self::SetBrightness(v) => ("light", "turn_on", vec![("brightness", Value::from(v))]),
            Self::SetColorTemp(v) => (
                "light",
                "turn_on",
                vec![("color_temp_kelvin", Value::from(v))],
            ),
            Self::SetTemperature(v) => (
                "climate",
                "set_temperature",
                vec![("temperature", Value::from(v))],
            ),
            Self::SetPosition(v) => (
                "cover",
                "set_cover_position",
                vec![("position", Value::from(v))],
            ),
        };

        ServiceCall {
            domain,
            service,
            params,
        }
    }

    /// The zero-argument action a widget fires from its header icon, or
    /// `None` for entities that only display.
    ///
    /// Derivable from the entity id alone, unlike [`ContinuousControl`],
    /// because every primary action is a plain domain-level toggle or
    /// trigger.
    pub fn primary_for_entity(entity_id: &str) -> Option<Self> {
        match domain(entity_id) {
            "switch" => Some(Self::ToggleSwitch),
            "light" => Some(Self::ToggleLight),
            "scene" => Some(Self::TriggerScene),
            "script" => Some(Self::TriggerScript),
            "input_boolean" => Some(Self::ToggleInputBoolean),
            _ => None,
        }
    }
}

/// Which dimension a [`ContinuousControl`] adjusts. Determines the
/// action built on release and how the value is presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContinuousKind {
    Brightness,
    /// White colour temperature of a light, in kelvin.
    ColorTemp,
    Temperature,
    Position,
}

impl ContinuousKind {
    /// How far an echo may sit from the value we sent and still count as
    /// confirmation of it.
    ///
    /// Not one constant, because the round trip through Home Assistant
    /// is lossy by different amounts per axis. Brightness, position and
    /// a setpoint are stored in the units we send, so anything past
    /// float representation noise is a genuinely different value.
    ///
    /// Colour temperature is not: most integrations store mireds and
    /// round-trip kelvin through `round(1_000_000 / kelvin)` and back,
    /// which does not return what we sent. Over the 50 K grid Snapdash
    /// actually sends, the worst case is 19 K (6350 K comes back as
    /// 6369 K). 25 K clears that comfortably while staying at half the
    /// step, so it can never confirm a neighbouring stop.
    pub fn echo_tolerance(self) -> f32 {
        match self {
            // A setpoint round-trips through HA as a float and may come
            // back with a different representation than we sent, which
            // is all this needs to absorb.
            Self::Brightness | Self::Temperature | Self::Position => 0.01,
            Self::ColorTemp => 25.0,
        }
    }
}

/// A numeric dimension of an entity that is *set* rather than toggled,
/// together with the range needed to render a control for it.
///
/// Ranges come from the entity's attributes rather than being hardcoded,
/// because HA reports per-device limits (a thermostat's `min_temp` and
/// `max_temp` vary by device and by unit system).
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousControl {
    pub kind: ContinuousKind,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    /// Current value as reported by HA, when it reports one.
    pub current: Option<f32>,
}

impl ContinuousControl {
    /// Build the action that sets this dimension to `value`.
    pub fn action(&self, value: f32) -> ActionKind {
        let clamped = value.clamp(self.min, self.max);
        match self.kind {
            ContinuousKind::Brightness => ActionKind::SetBrightness(clamped as u8),
            ContinuousKind::ColorTemp => ActionKind::SetColorTemp(clamped as u32),
            ContinuousKind::Temperature => ActionKind::SetTemperature(clamped),
            ContinuousKind::Position => ActionKind::SetPosition(clamped as u8),
        }
    }
}

/// What an entity supports, discovered from its current state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Capabilities {
    /// Fired by the widget's header icon.
    pub primary: Option<ActionKind>,
    /// Every axis the entity exposes, adjusted from the expanded widget.
    /// Empty for an entity with nothing to set.
    pub continuous: Vec<ContinuousControl>,
}

impl Capabilities {
    pub fn from_state(state: &EntityState) -> Self {
        Self {
            primary: ActionKind::primary_for_entity(&state.entity_id),
            continuous: continuous_controls(state),
        }
    }

    /// The axis of a given kind, when the entity has one.
    pub fn axis(&self, kind: ContinuousKind) -> Option<&ContinuousControl> {
        self.continuous.iter().find(|c| c.kind == kind)
    }

    /// `true` when the entity can only be displayed, never acted on.
    pub fn is_display_only(&self) -> bool {
        self.primary.is_none() && self.continuous.is_empty()
    }
}

fn attr_f32(state: &EntityState, key: &str) -> Option<f32> {
    match state.attributes.get(key)? {
        Value::Number(n) => n.as_f64().map(|v| v as f32),
        Value::String(s) => s.parse::<f32>().ok(),
        _ => None,
    }
}

fn supported_features(state: &EntityState) -> u64 {
    state
        .attributes
        .get("supported_features")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

/// A light is dimmable when it advertises any color mode other than
/// plain on/off. `onoff` and `unknown` are the two modes that carry no
/// brightness channel; everything else (`brightness`, `color_temp`,
/// `hs`, `xy`, `rgb`, `rgbw`, `rgbww`, `white`) does.
fn light_is_dimmable(state: &EntityState) -> bool {
    let Some(Value::Array(modes)) = state.attributes.get("supported_color_modes") else {
        return false;
    };

    modes
        .iter()
        .filter_map(Value::as_str)
        .any(|m| !matches!(m, "onoff" | "unknown"))
}

/// A light supports white colour temperature when it advertises the
/// `color_temp` colour mode.
///
/// `color_temp_kelvin` is `null` whenever the light is currently in some
/// other mode, so the axis is offered on the strength of what the device
/// *supports*, and the slider simply starts at its minimum until the
/// light is put into that mode.
fn light_has_color_temp(state: &EntityState) -> bool {
    let Some(Value::Array(modes)) = state.attributes.get("supported_color_modes") else {
        return false;
    };

    modes
        .iter()
        .filter_map(Value::as_str)
        .any(|m| m == "color_temp")
}

/// Every axis an entity exposes, in the order they should be shown.
///
/// A light can carry more than one, which is why this returns a list.
/// Each axis reconciles independently, but they share one send throttle
/// per entity, so the peak call rate does not grow with the axis count
/// (`docs/adr/0003-service-calls-stay-on-rest.md`).
fn continuous_controls(state: &EntityState) -> Vec<ContinuousControl> {
    let mut controls = Vec::new();

    match domain(&state.entity_id) {
        "light" => {
            if light_is_dimmable(state) {
                controls.push(ContinuousControl {
                    kind: ContinuousKind::Brightness,
                    min: 0.0,
                    max: 255.0,
                    step: 1.0,
                    current: attr_f32(state, "brightness"),
                });
            }

            if light_has_color_temp(state) {
                // Ranges are per-device. The fallbacks are HA's own
                // defaults for a light that reports the mode but not its
                // limits, which some integrations do.
                controls.push(ContinuousControl {
                    kind: ContinuousKind::ColorTemp,
                    min: attr_f32(state, "min_color_temp_kelvin").unwrap_or(2000.0),
                    max: attr_f32(state, "max_color_temp_kelvin").unwrap_or(6535.0),
                    // Kelvin spans thousands, so a 1 K step would be a
                    // slider with several thousand stops and no visible
                    // difference between neighbours.
                    step: 50.0,
                    current: attr_f32(state, "color_temp_kelvin"),
                });
            }
        }

        // ClimateEntityFeature.TARGET_TEMPERATURE == 1. Without it the
        // entity has no single setpoint to drag (it may be a range-only
        // thermostat, which is not covered).
        "climate" if supported_features(state) & 1 != 0 => {
            controls.push(ContinuousControl {
                kind: ContinuousKind::Temperature,
                min: attr_f32(state, "min_temp").unwrap_or(7.0),
                max: attr_f32(state, "max_temp").unwrap_or(35.0),
                step: attr_f32(state, "target_temp_step").unwrap_or(0.5),
                current: attr_f32(state, "temperature"),
            });
        }

        // CoverEntityFeature.SET_POSITION == 4. Open/close-only covers
        // report 1|2 but not 4 and get no slider.
        "cover" if supported_features(state) & 4 != 0 => {
            controls.push(ContinuousControl {
                kind: ContinuousKind::Position,
                min: 0.0,
                max: 100.0,
                step: 1.0,
                current: attr_f32(state, "current_position"),
            });
        }

        _ => {}
    }

    controls
}

/// Whether an entity is eligible to appear in the Settings widget
/// picker. See [`WIDGET_DOMAINS`] for why this is domain-based.
pub fn is_widget_candidate(entity_id: &str) -> bool {
    WIDGET_DOMAINS.contains(&domain(entity_id))
}

/// POST `/api/services/{domain}/{service}` with `{"entity_id": …}` plus
/// any action parameters.
///
/// Returns `Ok(())` when HA acknowledges with a 2xx. Any transport or
/// server-side failure lands in `HaError::ServiceCall` with enough
/// context for the status bar. The widget UI is refreshed via the
/// normal `state_changed` WS event that follows a successful call,
/// so this function is fire-and-forget from the caller's perspective.
pub async fn call_service(
    ha_url: &str,
    token: &str,
    action: ActionKind,
    entity_id: &str,
) -> Result<(), HaError> {
    let call = action.service_call();
    let base = ha_url.trim_end_matches('/');
    let url = format!("{base}/api/services/{}/{}", call.domain, call.service);

    let mut body = serde_json::Map::new();
    body.insert("entity_id".into(), Value::from(entity_id));
    for (key, value) in call.params {
        body.insert(key.into(), value);
    }

    let resp = http()
        .post(&url)
        .bearer_auth(token)
        .json(&Value::Object(body))
        .send()
        .await
        .map_err(|e| HaError::ServiceCall {
            entity_id: entity_id.to_owned(),
            status: None,
            message: e.to_string(),
        })?;

    if !resp.status().is_success() {
        return Err(HaError::ServiceCall {
            entity_id: entity_id.to_owned(),
            status: Some(resp.status().as_u16()),
            message: resp
                .status()
                .canonical_reason()
                .unwrap_or("error")
                .to_owned(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state(entity_id: &str, attrs: Value) -> EntityState {
        EntityState {
            entity_id: entity_id.to_owned(),
            state: "on".into(),
            attributes: serde_json::from_value(attrs).unwrap(),
            last_changed: None,
            last_updated: None,
        }
    }

    #[test]
    fn maps_supported_domains_to_primary_actions() {
        assert_eq!(
            ActionKind::primary_for_entity("switch.kitchen"),
            Some(ActionKind::ToggleSwitch)
        );
        assert_eq!(
            ActionKind::primary_for_entity("light.living_room"),
            Some(ActionKind::ToggleLight)
        );
        assert_eq!(
            ActionKind::primary_for_entity("scene.evening"),
            Some(ActionKind::TriggerScene)
        );
        assert_eq!(
            ActionKind::primary_for_entity("script.morning_routine"),
            Some(ActionKind::TriggerScript)
        );
        assert_eq!(
            ActionKind::primary_for_entity("input_boolean.guest_mode"),
            Some(ActionKind::ToggleInputBoolean)
        );
    }

    #[test]
    fn rejects_unsupported_domains() {
        assert_eq!(ActionKind::primary_for_entity("sensor.outdoor_temp"), None);
        assert_eq!(ActionKind::primary_for_entity("climate.thermostat"), None);
        assert_eq!(
            ActionKind::primary_for_entity("media_player.living_tv"),
            None
        );
        assert_eq!(ActionKind::primary_for_entity("cover.blinds"), None);
        assert_eq!(ActionKind::primary_for_entity("lock.front_door"), None);
    }

    #[test]
    fn rejects_malformed_entity_ids() {
        // Missing domain (no dot).
        assert_eq!(ActionKind::primary_for_entity("just_a_name"), None);
        assert_eq!(ActionKind::primary_for_entity(""), None);
        // Wrong-case domain - HA IDs are always lowercase, we don't try to normalize.
        assert_eq!(ActionKind::primary_for_entity("Switch.kitchen"), None);
    }

    #[test]
    fn widget_candidates_span_display_and_actions() {
        // Read-only display candidates.
        assert!(is_widget_candidate("sensor.outdoor_temp"));
        assert!(is_widget_candidate("binary_sensor.front_door"));
        // Actionable ones.
        assert!(is_widget_candidate("switch.kitchen"));
        assert!(is_widget_candidate("light.living_room"));
        assert!(is_widget_candidate("scene.evening"));
        assert!(is_widget_candidate("script.morning"));
        assert!(is_widget_candidate("input_boolean.guest_mode"));
        // Continuous-control domains (#87). Candidates regardless of
        // whether the individual device supports a setpoint or position -
        // their state is worth displaying either way.
        assert!(is_widget_candidate("climate.thermostat"));
        assert!(is_widget_candidate("cover.blinds"));
        // Still out of scope - no phase 2 control surface for media.
        assert!(!is_widget_candidate("media_player.living_tv"));
        // Malformed.
        assert!(!is_widget_candidate("no_dot_here"));
        assert!(!is_widget_candidate(""));
    }

    #[test]
    fn wire_mapping_is_stable() {
        // Any change here means the on-wire call to HA changes shape -
        // bump deliberately.
        let call = |a: ActionKind| {
            let c = a.service_call();
            (c.domain, c.service, c.params)
        };

        assert_eq!(call(ActionKind::ToggleSwitch), ("switch", "toggle", vec![]));
        assert_eq!(call(ActionKind::ToggleLight), ("light", "toggle", vec![]));
        assert_eq!(call(ActionKind::TriggerScene), ("scene", "turn_on", vec![]));
        assert_eq!(
            call(ActionKind::TriggerScript),
            ("script", "turn_on", vec![])
        );
        assert_eq!(
            call(ActionKind::ToggleInputBoolean),
            ("input_boolean", "toggle", vec![])
        );
        assert_eq!(
            call(ActionKind::SetBrightness(128)),
            ("light", "turn_on", vec![("brightness", json!(128))])
        );
        assert_eq!(
            call(ActionKind::SetTemperature(21.5)),
            (
                "climate",
                "set_temperature",
                vec![("temperature", json!(21.5))]
            )
        );
        assert_eq!(
            call(ActionKind::SetPosition(60)),
            ("cover", "set_cover_position", vec![("position", json!(60))])
        );
        // Kelvin, not mireds. HA accepts both but `color_temp` (mireds)
        // is deprecated, and the two are inverse scales, so sending the
        // wrong key would drive the slider backwards.
        assert_eq!(
            call(ActionKind::SetColorTemp(3000)),
            ("light", "turn_on", vec![("color_temp_kelvin", json!(3000))])
        );
    }

    #[test]
    fn dimmable_light_reports_brightness_control() {
        let s = state(
            "light.living_room",
            json!({ "supported_color_modes": ["brightness"], "brightness": 128 }),
        );
        let caps = Capabilities::from_state(&s);

        assert_eq!(caps.primary, Some(ActionKind::ToggleLight));
        let c = caps
            .axis(ContinuousKind::Brightness)
            .expect("dimmable light has a brightness axis");
        assert_eq!((c.min, c.max), (0.0, 255.0));
        assert_eq!(c.current, Some(128.0));
        // Brightness alone: this bulb advertises no colour temperature.
        assert_eq!(caps.continuous.len(), 1);
    }

    /// A light can expose more than one axis, which is the whole reason
    /// capabilities are a list rather than a single control.
    #[test]
    fn a_light_can_report_brightness_and_colour_temperature() {
        let s = state(
            "light.kitchen",
            json!({
                "supported_color_modes": ["color_temp"],
                "brightness": 200,
                "color_temp_kelvin": 3000,
                "min_color_temp_kelvin": 2202,
                "max_color_temp_kelvin": 6535
            }),
        );
        let caps = Capabilities::from_state(&s);

        assert_eq!(caps.continuous.len(), 2);
        // Brightness first: it is the axis people reach for.
        assert_eq!(caps.continuous[0].kind, ContinuousKind::Brightness);

        let temp = caps
            .axis(ContinuousKind::ColorTemp)
            .expect("color_temp mode means a white axis");
        assert_eq!((temp.min, temp.max), (2202.0, 6535.0));
        assert_eq!(temp.current, Some(3000.0));
    }

    /// `color_temp_kelvin` is null whenever the light sits in another
    /// colour mode. The axis is still offered, because the device
    /// supports it, and the slider simply starts at its minimum.
    #[test]
    fn colour_temperature_survives_the_light_being_in_another_mode() {
        let s = state(
            "light.kitchen",
            json!({
                "supported_color_modes": ["color_temp", "hs"],
                "color_mode": "hs",
                "color_temp_kelvin": null
            }),
        );
        let temp = Capabilities::from_state(&s)
            .axis(ContinuousKind::ColorTemp)
            .cloned()
            .expect("still supported, just not active");

        assert_eq!(temp.current, None);
    }

    #[test]
    fn onoff_only_light_has_no_continuous_axes() {
        let s = state("light.porch", json!({ "supported_color_modes": ["onoff"] }));
        let caps = Capabilities::from_state(&s);

        // Still tappable, just not adjustable. This is exactly the case
        // an entity-id-only check gets wrong.
        assert_eq!(caps.primary, Some(ActionKind::ToggleLight));
        assert!(caps.continuous.is_empty());
    }

    #[test]
    fn light_without_color_modes_has_no_continuous_axes() {
        let s = state("light.mystery", json!({}));
        assert!(Capabilities::from_state(&s).continuous.is_empty());
    }

    #[test]
    fn climate_setpoint_uses_reported_range() {
        let s = state(
            "climate.hall",
            json!({
                "supported_features": 1,
                "min_temp": 10.0,
                "max_temp": 28.0,
                "target_temp_step": 0.5,
                "temperature": 21.0
            }),
        );
        let caps = Capabilities::from_state(&s);
        let c = caps
            .axis(ContinuousKind::Temperature)
            .expect("target temperature supported");

        assert_eq!((c.min, c.max, c.step), (10.0, 28.0, 0.5));
        assert_eq!(c.current, Some(21.0));
    }

    #[test]
    fn climate_without_target_temperature_feature_has_no_control() {
        let s = state("climate.hall", json!({ "supported_features": 0 }));
        assert!(Capabilities::from_state(&s).continuous.is_empty());
    }

    #[test]
    fn cover_needs_set_position_feature() {
        // OPEN|CLOSE|STOP but no SET_POSITION (4).
        let positionless = state("cover.garage", json!({ "supported_features": 11 }));
        assert!(
            Capabilities::from_state(&positionless)
                .continuous
                .is_empty()
        );

        let positionable = state(
            "cover.blinds",
            json!({ "supported_features": 15, "current_position": 40 }),
        );
        let caps = Capabilities::from_state(&positionable);
        let c = caps
            .axis(ContinuousKind::Position)
            .expect("SET_POSITION means a position axis");
        assert_eq!(c.current, Some(40.0));
    }

    #[test]
    fn sensors_are_display_only() {
        let s = state(
            "sensor.outdoor_temp",
            json!({ "unit_of_measurement": "°C" }),
        );
        assert!(Capabilities::from_state(&s).is_display_only());
    }

    #[test]
    fn continuous_control_clamps_before_building_an_action() {
        let c = ContinuousControl {
            kind: ContinuousKind::Brightness,
            min: 0.0,
            max: 255.0,
            step: 1.0,
            current: None,
        };

        assert_eq!(c.action(300.0), ActionKind::SetBrightness(255));
        assert_eq!(c.action(-20.0), ActionKind::SetBrightness(0));
        assert_eq!(c.action(128.0), ActionKind::SetBrightness(128));
    }
}
