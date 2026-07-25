//! Outbound service calls to Home Assistant.
//!
//! Actionable widgets (issue #81) let the user tap a widget to trigger a
//! Home Assistant action — toggle a switch, activate a scene, run a
//! script. This module owns the "send a service call" side of that
//! interaction.
//!
//! ## Why REST and not the existing WebSocket?
//!
//! HA accepts `call_service` over both transports and the auth is the
//! same bearer token. WS would save one HTTP handshake per tap (~30–100 ms
//! on a LAN, imperceptible for a user-driven click) but would require:
//!
//! - handing an outbound `mpsc::Sender` back from the iced subscription
//!   (`Subscription::run_with` in `src/app/lifecycle.rs` is read-only),
//! - tracking monotonic request IDs and correlating them with
//!   `ServerMsg::Result`,
//! - re-attaching the sender across reconnects.
//!
//! REST drops in next to the existing `rest::fetch_all_states` with none
//! of that. Phase 2 (brightness sliders, climate setpoints — anything
//! that produces many calls/sec from a drag gesture) is a good candidate
//! to migrate to WS when it lands.

use crate::ha::types::HaError;
use crate::ui::format::domain;

/// A widget action that maps to a single HA service call with no extra
/// parameters. Phase 1 scope — one-tap toggles and one-shot triggers.
///
/// Phases 2 and 3 (issue #81) will add value-carrying variants
/// (`SetBrightness(u8)`, `SetTemperature(f32)`, `Play`, `Pause`, …) that
/// need per-domain payloads and probably a different UI affordance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    /// `switch.toggle` — HA flips it based on current state.
    ToggleSwitch,
    /// `light.toggle` — on/off only, no brightness or color.
    ToggleLight,
    /// `scene.turn_on` — a scene is stateless: "activate this scene".
    TriggerScene,
    /// `script.turn_on` — starts the script from the top.
    TriggerScript,
    /// `input_boolean.toggle` — HA's user-defined boolean helper.
    ToggleInputBoolean,
}

impl ActionKind {
    /// Returns the `(domain, service)` pair to POST to. Reused by
    /// `call_service` and by tests to assert the wire mapping.
    pub const fn service(self) -> (&'static str, &'static str) {
        match self {
            Self::ToggleSwitch => ("switch", "toggle"),
            Self::ToggleLight => ("light", "toggle"),
            Self::TriggerScene => ("scene", "turn_on"),
            Self::TriggerScript => ("script", "turn_on"),
            Self::ToggleInputBoolean => ("input_boolean", "toggle"),
        }
    }

    /// `Some(action)` when the entity's domain is supported in Phase 1,
    /// `None` for anything else (sensors, climate, media_player, …). The
    /// widget UI uses this to decide whether to render the action button
    /// at all.
    pub fn from_entity_id(entity_id: &str) -> Option<Self> {
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

/// Whether an entity is eligible to appear in the Settings widget picker.
///
/// - `sensor.*` and `binary_sensor.*` are the classic read-only display
///   candidates (temperature, humidity, motion, door open/closed, …).
/// - The five actionable domains (`switch`, `light`, `scene`, `script`,
///   `input_boolean`) come along for the ride — see [`ActionKind`].
///
/// Later phases will extend this (`climate`, `cover`, `media_player`,
/// `fan`, `lock`, `vacuum`); adding a domain here + a matching
/// `ActionKind` variant is enough to expose it in the picker.
pub fn is_widget_candidate(entity_id: &str) -> bool {
    matches!(domain(entity_id), "sensor" | "binary_sensor")
        || ActionKind::from_entity_id(entity_id).is_some()
}

/// POST `/api/services/{domain}/{service}` with `{"entity_id": …}`.
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
    let (domain, service) = action.service();
    let base = ha_url.trim_end_matches('/');
    let url = format!("{base}/api/services/{domain}/{service}");
    let body = serde_json::json!({ "entity_id": entity_id });

    let resp = reqwest::Client::new()
        .post(&url)
        .bearer_auth(token)
        .json(&body)
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

    #[test]
    fn maps_supported_domains_to_service_calls() {
        assert_eq!(
            ActionKind::from_entity_id("switch.kitchen"),
            Some(ActionKind::ToggleSwitch)
        );
        assert_eq!(
            ActionKind::from_entity_id("light.living_room"),
            Some(ActionKind::ToggleLight)
        );
        assert_eq!(
            ActionKind::from_entity_id("scene.evening"),
            Some(ActionKind::TriggerScene)
        );
        assert_eq!(
            ActionKind::from_entity_id("script.morning_routine"),
            Some(ActionKind::TriggerScript)
        );
        assert_eq!(
            ActionKind::from_entity_id("input_boolean.guest_mode"),
            Some(ActionKind::ToggleInputBoolean)
        );
    }

    #[test]
    fn rejects_unsupported_domains() {
        assert_eq!(ActionKind::from_entity_id("sensor.outdoor_temp"), None);
        assert_eq!(ActionKind::from_entity_id("climate.thermostat"), None);
        assert_eq!(ActionKind::from_entity_id("media_player.living_tv"), None);
        assert_eq!(ActionKind::from_entity_id("cover.blinds"), None);
        assert_eq!(ActionKind::from_entity_id("lock.front_door"), None);
    }

    #[test]
    fn rejects_malformed_entity_ids() {
        // Missing domain (no dot).
        assert_eq!(ActionKind::from_entity_id("just_a_name"), None);
        assert_eq!(ActionKind::from_entity_id(""), None);
        // Wrong-case domain — HA IDs are always lowercase, we don't try to normalize.
        assert_eq!(ActionKind::from_entity_id("Switch.kitchen"), None);
    }

    #[test]
    fn widget_candidates_span_display_and_actions() {
        // Read-only display candidates.
        assert!(is_widget_candidate("sensor.outdoor_temp"));
        assert!(is_widget_candidate("binary_sensor.front_door"));
        // Actionable ones travel with ActionKind.
        assert!(is_widget_candidate("switch.kitchen"));
        assert!(is_widget_candidate("light.living_room"));
        assert!(is_widget_candidate("scene.evening"));
        assert!(is_widget_candidate("script.morning"));
        assert!(is_widget_candidate("input_boolean.guest_mode"));
        // Not in Phase 1 scope — no picker exposure yet.
        assert!(!is_widget_candidate("climate.thermostat"));
        assert!(!is_widget_candidate("media_player.living_tv"));
        assert!(!is_widget_candidate("cover.blinds"));
        // Malformed.
        assert!(!is_widget_candidate("no_dot_here"));
        assert!(!is_widget_candidate(""));
    }

    #[test]
    fn wire_mapping_is_stable() {
        // Any change here means the on-wire call to HA changes shape —
        // bump deliberately.
        assert_eq!(ActionKind::ToggleSwitch.service(), ("switch", "toggle"));
        assert_eq!(ActionKind::ToggleLight.service(), ("light", "toggle"));
        assert_eq!(ActionKind::TriggerScene.service(), ("scene", "turn_on"));
        assert_eq!(ActionKind::TriggerScript.service(), ("script", "turn_on"));
        assert_eq!(
            ActionKind::ToggleInputBoolean.service(),
            ("input_boolean", "toggle")
        );
    }
}
