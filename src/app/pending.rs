//! Pending values: the local override that wins over Home Assistant
//! truth while the user is driving a continuous control.
//!
//! Sends are throttled during a drag, so HA keeps broadcasting
//! `state_changed` with values that lag the user's finger. Binding a
//! slider straight to those echoes makes it visibly fight the drag. So
//! from first touch the value shown is local, and HA only takes over
//! again once the interaction has reconciled.
//!
//! Reconciliation matches on **value**, not on an id: `state_changed` is
//! a broadcast carrying no reference to the service call that caused it,
//! so there is no correlation id to match on even over the WebSocket.
//! The settle timeout is the safety net for the case value-matching
//! cannot terminate on its own: HA clamping the value, rejecting the
//! call, or silently dropping it. Without it a pending value would stay
//! authoritative forever and the widget would quietly lie about the
//! state of the house.
//!
//! State is kept **per axis**, so brightness and colour temperature
//! reconcile independently and neither overwrites the other. Throttling
//! is kept **per entity**, so a light with two sliders still peaks at
//! the same call rate as one with a single slider. That is what keeps
//! the arithmetic in `docs/adr/0003-service-calls-stay-on-rest.md` valid
//! as axes are added.
//!
//! Recorded in `docs/adr/0002-pending-values-and-settle-window.md`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::ha::ContinuousKind;

/// Minimum gap between service calls for one entity while it is being
/// driven. Caps the send rate at ~5/sec, which is what keeps REST viable
/// as the transport (`docs/adr/0003-service-calls-stay-on-rest.md`).
pub const SEND_INTERVAL: Duration = Duration::from_millis(200);

/// How long a released value stays authoritative while waiting for a
/// matching echo before HA truth is allowed to win again.
pub const SETTLE_TIMEOUT: Duration = Duration::from_secs(2);

/// Echoes are compared to the last sent value with a tolerance, because
/// a setpoint round-trips through HA as a float and may come back with
/// a different representation than we sent.
const EPSILON: f32 = 0.01;

/// One axis's in-flight interaction.
#[derive(Debug, Clone)]
struct Pending {
    /// What the widget and the control render right now.
    shown: f32,
    /// The most recent value actually put on the wire, awaiting an echo.
    last_sent: Option<f32>,
    /// `None` while the user still holds the control. Set on release,
    /// after which the pending value has a bounded life.
    settle_deadline: Option<Instant>,
}

impl Pending {
    fn new(value: f32) -> Self {
        Self {
            shown: value,
            last_sent: None,
            settle_deadline: None,
        }
    }

    /// The user moved the control. Re-arms the interaction, so a value
    /// changed after release is treated as a fresh drag rather than
    /// inheriting the old deadline.
    fn update(&mut self, value: f32) {
        self.shown = value;
        self.settle_deadline = None;
    }

    /// `true` when this echo is the one we were waiting for, meaning the
    /// axis can go back to being driven by HA.
    fn reconciles(&self, echoed: f32) -> bool {
        self.last_sent
            .is_some_and(|sent| (sent - echoed).abs() <= EPSILON)
    }

    /// `true` once the settle window has run out. Never true while the
    /// user still holds the control.
    fn expired(&self, now: Instant) -> bool {
        self.settle_deadline.is_some_and(|deadline| now >= deadline)
    }
}

type AxisKey = (String, ContinuousKind);

/// All in-flight interactions.
#[derive(Debug, Default)]
pub struct PendingValues {
    axes: HashMap<AxisKey, Pending>,
    /// Last time anything was sent for an entity. Shared by all of that
    /// entity's axes, which is what keeps the peak call rate flat as
    /// axes are added.
    last_send_at: HashMap<String, Instant>,
}

impl PendingValues {
    /// The locally-held value for one axis, if it currently has one.
    /// Callers render this in preference to the HA state.
    pub fn shown(&self, entity_id: &str, kind: ContinuousKind) -> Option<f32> {
        self.axes
            .get(&(entity_id.to_owned(), kind))
            .map(|pending| pending.shown)
    }

    pub fn is_empty(&self) -> bool {
        self.axes.is_empty()
    }

    /// Whether the throttle window for this entity has elapsed.
    fn send_due(&self, entity_id: &str, now: Instant) -> bool {
        match self.last_send_at.get(entity_id) {
            None => true,
            Some(at) => now.duration_since(*at) >= SEND_INTERVAL,
        }
    }

    /// Record a new value for one axis. Returns `Some(value)` when a send
    /// is due now, `None` when the entity's throttle window swallows it.
    ///
    /// A swallowed value is not lost: it stays in `shown`, and
    /// [`Self::release`] always flushes the final one.
    pub fn set(
        &mut self,
        entity_id: &str,
        kind: ContinuousKind,
        value: f32,
        now: Instant,
    ) -> Option<f32> {
        let due = self.send_due(entity_id, now);

        let pending = self
            .axes
            .entry((entity_id.to_owned(), kind))
            .or_insert_with(|| Pending::new(value));
        pending.update(value);

        if !due {
            return None;
        }

        pending.last_sent = Some(value);
        self.last_send_at.insert(entity_id.to_owned(), now);
        Some(value)
    }

    /// The user released the control. Always returns the final value to
    /// send, so an interaction never ends on a throttled-away
    /// intermediate, and starts the settle window.
    pub fn release(&mut self, entity_id: &str, kind: ContinuousKind, now: Instant) -> Option<f32> {
        let pending = self.axes.get_mut(&(entity_id.to_owned(), kind))?;
        let value = pending.shown;

        pending.last_sent = Some(value);
        pending.settle_deadline = Some(now + SETTLE_TIMEOUT);
        self.last_send_at.insert(entity_id.to_owned(), now);

        Some(value)
    }

    /// Feed in the values an entity's `state_changed` carries, one per
    /// axis it reports.
    ///
    /// Returns `true` when the entity is HA-driven again, meaning no axis
    /// of it is still waiting. `false` means the caller must keep
    /// rendering the pending values and ignore this echo, because
    /// applying it would drag a slider back under the user's finger.
    pub fn reconcile(&mut self, entity_id: &str, echoed: &[(ContinuousKind, Option<f32>)]) -> bool {
        for (kind, value) in echoed {
            let key = (entity_id.to_owned(), *kind);
            let Some(pending) = self.axes.get(&key) else {
                continue;
            };

            if value.is_some_and(|value| pending.reconciles(value)) {
                self.axes.remove(&key);
            }
        }

        let still_waiting = self.axes.keys().any(|(id, _)| id == entity_id);
        if !still_waiting {
            self.last_send_at.remove(entity_id);
        }

        !still_waiting
    }

    /// Drop every pending value whose settle window has run out, so HA
    /// truth wins again after a command it clamped or dropped.
    ///
    /// Returns the entities that lost at least one axis, without
    /// duplicates. The caller needs them individually: a widget that just
    /// lost a pending value is still displaying that local number, and
    /// nothing else will correct it. In exactly the case the timeout
    /// exists for, the next `state_changed` may never come.
    pub fn expire(&mut self, now: Instant) -> Vec<String> {
        let retired: Vec<AxisKey> = self
            .axes
            .iter()
            .filter(|(_, pending)| pending.expired(now))
            .map(|(key, _)| key.clone())
            .collect();

        let mut entities: Vec<String> = Vec::new();
        for key in retired {
            if !entities.contains(&key.0) {
                entities.push(key.0.clone());
            }
            self.axes.remove(&key);
        }

        for entity_id in &entities {
            if !self.axes.keys().any(|(id, _)| id == entity_id) {
                self.last_send_at.remove(entity_id);
            }
        }

        entities
    }

    /// Forget an entity entirely, e.g. when its widget closes or its
    /// controls are collapsed.
    pub fn clear(&mut self, entity_id: &str) {
        self.axes.retain(|(id, _), _| id != entity_id);
        self.last_send_at.remove(entity_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRIGHTNESS: ContinuousKind = ContinuousKind::Brightness;
    const TEMP: ContinuousKind = ContinuousKind::ColorTemp;

    fn t0() -> Instant {
        Instant::now()
    }

    #[test]
    fn first_value_sends_immediately() {
        let mut p = PendingValues::default();
        assert_eq!(p.set("light.a", BRIGHTNESS, 100.0, t0()), Some(100.0));
    }

    #[test]
    fn throttle_swallows_sends_inside_the_window() {
        let now = t0();
        let mut p = PendingValues::default();

        assert_eq!(p.set("light.a", BRIGHTNESS, 100.0, now), Some(100.0));
        // Still inside SEND_INTERVAL, so no wire traffic...
        assert_eq!(
            p.set(
                "light.a",
                BRIGHTNESS,
                120.0,
                now + Duration::from_millis(50)
            ),
            None
        );
        assert_eq!(
            p.set(
                "light.a",
                BRIGHTNESS,
                140.0,
                now + Duration::from_millis(100)
            ),
            None
        );
        // ...but the value is not lost, the control still shows it.
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(140.0));

        // Past the window, the next move goes out.
        assert_eq!(
            p.set("light.a", BRIGHTNESS, 160.0, now + SEND_INTERVAL),
            Some(160.0)
        );
    }

    /// The throttle is per entity, not per axis. Two sliders on one light
    /// must not double its call rate, or the ~5/sec ceiling that keeps
    /// REST viable (ADR-0003) stops holding as axes are added.
    #[test]
    fn axes_of_one_entity_share_the_send_throttle() {
        let now = t0();
        let mut p = PendingValues::default();

        assert_eq!(p.set("light.a", BRIGHTNESS, 100.0, now), Some(100.0));
        // A different axis of the same light, immediately after.
        assert_eq!(
            p.set("light.a", TEMP, 3000.0, now + Duration::from_millis(10)),
            None,
            "shares the entity's window"
        );
        // A different entity is unaffected.
        assert_eq!(
            p.set("light.b", BRIGHTNESS, 50.0, now + Duration::from_millis(10)),
            Some(50.0)
        );
    }

    #[test]
    fn release_always_flushes_the_final_value() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        // Throttled away.
        assert_eq!(
            p.set(
                "light.a",
                BRIGHTNESS,
                250.0,
                now + Duration::from_millis(10)
            ),
            None
        );
        // Release must still put 250 on the wire, or the light ends up
        // sitting at a value the user scrubbed past.
        assert_eq!(
            p.release("light.a", BRIGHTNESS, now + Duration::from_millis(20)),
            Some(250.0)
        );
    }

    #[test]
    fn matching_echo_hands_control_back_to_ha() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        assert!(
            !p.reconcile("light.a", &[(BRIGHTNESS, Some(40.0))]),
            "stale echo ignored"
        );
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(100.0));

        assert!(
            p.reconcile("light.a", &[(BRIGHTNESS, Some(100.0))]),
            "matching echo resolves"
        );
        assert_eq!(p.shown("light.a", BRIGHTNESS), None);
    }

    /// One echo carries every axis. Resolving brightness while colour
    /// temperature is still in flight must not hand the whole widget
    /// back to HA, or the temperature slider jumps under the finger.
    #[test]
    fn an_entity_stays_held_while_any_axis_is_still_waiting() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        p.set("light.a", TEMP, 3000.0, now + SEND_INTERVAL);

        let resolved = p.reconcile(
            "light.a",
            &[(BRIGHTNESS, Some(100.0)), (TEMP, Some(2500.0))],
        );

        assert!(!resolved, "colour temperature has not come back yet");
        assert_eq!(p.shown("light.a", BRIGHTNESS), None, "brightness resolved");
        assert_eq!(p.shown("light.a", TEMP), Some(3000.0), "still held");

        assert!(p.reconcile("light.a", &[(TEMP, Some(3000.0))]));
        assert!(p.is_empty());
    }

    #[test]
    fn echo_matching_tolerates_float_round_trip() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("climate.a", ContinuousKind::Temperature, 21.5, now);
        assert!(p.reconcile("climate.a", &[(ContinuousKind::Temperature, Some(21.502))]));
    }

    #[test]
    fn entities_without_a_pending_value_are_always_ha_driven() {
        let mut p = PendingValues::default();
        assert!(p.reconcile("sensor.temp", &[]));
        assert!(p.reconcile("sensor.temp", &[(BRIGHTNESS, Some(12.0))]));
    }

    #[test]
    fn missing_echo_value_does_not_resolve() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        // A state_changed with no readable value for the axis must not be
        // mistaken for confirmation.
        assert!(!p.reconcile("light.a", &[(BRIGHTNESS, None)]));
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(100.0));
    }

    #[test]
    fn held_control_never_expires() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        // Long past the settle timeout, but the user has not let go.
        assert!(p.expire(now + SETTLE_TIMEOUT * 10).is_empty());
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(100.0));
    }

    /// One entity expiring must be reported even while others are still
    /// pending: the caller refreshes exactly the retired widgets, and a
    /// widget that isn't named keeps showing a value HA never confirmed.
    #[test]
    fn expiry_reports_each_entity_independently() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        p.release("light.a", BRIGHTNESS, now);
        // Second entity grabbed later, so its window has not elapsed.
        p.set("light.b", BRIGHTNESS, 50.0, now + SETTLE_TIMEOUT);
        p.release("light.b", BRIGHTNESS, now + SETTLE_TIMEOUT);

        let retired = p.expire(now + SETTLE_TIMEOUT);
        assert_eq!(retired, vec!["light.a".to_owned()]);
        assert_eq!(p.shown("light.a", BRIGHTNESS), None);
        assert_eq!(
            p.shown("light.b", BRIGHTNESS),
            Some(50.0),
            "still inside its window"
        );
        assert!(!p.is_empty());
    }

    /// Two axes of one entity expiring together name that entity once.
    /// The caller uses the list to push HA truth back onto each widget,
    /// and doing it twice would be wasted work.
    #[test]
    fn expiry_names_an_entity_once_however_many_axes_it_lost() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        p.release("light.a", BRIGHTNESS, now);
        p.set("light.a", TEMP, 3000.0, now);
        p.release("light.a", TEMP, now);

        assert_eq!(p.expire(now + SETTLE_TIMEOUT), vec!["light.a".to_owned()]);
        assert!(p.is_empty());
    }

    #[test]
    fn released_value_expires_when_no_echo_matches() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        p.release("light.a", BRIGHTNESS, now);

        assert!(
            p.expire(now + SETTLE_TIMEOUT - Duration::from_millis(1))
                .is_empty()
        );
        assert_eq!(
            p.shown("light.a", BRIGHTNESS),
            Some(100.0),
            "still inside the window"
        );

        // HA clamped or dropped the command and will never echo 100.
        assert_eq!(p.expire(now + SETTLE_TIMEOUT), vec!["light.a".to_owned()]);
        assert_eq!(p.shown("light.a", BRIGHTNESS), None, "HA truth wins again");
    }

    #[test]
    fn moving_again_after_release_re_arms_the_interaction() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        p.release("light.a", BRIGHTNESS, now);
        // User grabs the slider again before the window elapses.
        p.set("light.a", BRIGHTNESS, 200.0, now + SEND_INTERVAL);

        // The old deadline must not still be running.
        assert!(p.expire(now + SETTLE_TIMEOUT).is_empty());
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(200.0));
    }

    #[test]
    fn clear_forgets_every_axis_of_the_entity() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", BRIGHTNESS, 100.0, now);
        p.set("light.a", TEMP, 3000.0, now);
        p.set("light.b", BRIGHTNESS, 50.0, now);

        p.clear("light.a");

        assert_eq!(p.shown("light.a", BRIGHTNESS), None);
        assert_eq!(p.shown("light.a", TEMP), None);
        assert_eq!(p.shown("light.b", BRIGHTNESS), Some(50.0), "untouched");
    }
}
