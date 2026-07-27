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
//! cannot terminate on its own — HA clamping the value, rejecting the
//! call, or silently dropping it. Without it a pending value would stay
//! authoritative forever and the widget would quietly lie about the
//! state of the house.
//!
//! Recorded in `docs/adr/0002-pending-values-and-settle-window.md`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Minimum gap between service calls while a control is being dragged.
/// Caps the send rate at ~5/sec, which is what keeps REST viable as the
/// transport (`docs/adr/0003-service-calls-stay-on-rest.md`).
pub const SEND_INTERVAL: Duration = Duration::from_millis(200);

/// How long a released value stays authoritative while waiting for a
/// matching echo before HA truth is allowed to win again.
pub const SETTLE_TIMEOUT: Duration = Duration::from_secs(2);

/// Echoes are compared to the last sent value with a tolerance, because
/// a setpoint round-trips through HA as a float and may come back with
/// a different representation than we sent.
const EPSILON: f32 = 0.01;

/// One entity's in-flight interaction.
#[derive(Debug, Clone)]
pub struct Pending {
    /// What the widget and the control render right now.
    shown: f32,
    /// The most recent value actually put on the wire, awaiting an echo.
    last_sent: Option<f32>,
    last_send_at: Option<Instant>,
    /// `None` while the user still holds the control. Set on release,
    /// after which the pending value has a bounded life.
    settle_deadline: Option<Instant>,
}

impl Pending {
    fn new(value: f32) -> Self {
        Self {
            shown: value,
            last_sent: None,
            last_send_at: None,
            settle_deadline: None,
        }
    }

    pub fn shown(&self) -> f32 {
        self.shown
    }

    /// The user moved the control. Re-arms the interaction, so a value
    /// changed after release is treated as a fresh drag rather than
    /// inheriting the old deadline.
    fn update(&mut self, value: f32) {
        self.shown = value;
        self.settle_deadline = None;
    }

    /// Whether the throttle window has elapsed and a send is due.
    fn send_due(&self, now: Instant) -> bool {
        match self.last_send_at {
            None => true,
            Some(at) => now.duration_since(at) >= SEND_INTERVAL,
        }
    }

    fn record_send(&mut self, now: Instant) {
        self.last_sent = Some(self.shown);
        self.last_send_at = Some(now);
    }

    /// The user let go. From here the pending value is on a clock.
    fn release(&mut self, now: Instant) {
        self.settle_deadline = Some(now + SETTLE_TIMEOUT);
    }

    /// `true` when this echo is the one we were waiting for, meaning the
    /// entity can go back to being driven by HA.
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

/// All in-flight interactions, keyed by entity id.
#[derive(Debug, Default)]
pub struct PendingValues(HashMap<String, Pending>);

impl PendingValues {
    /// The locally-held value for an entity, if it currently has one.
    /// Callers render this in preference to the HA state.
    pub fn shown(&self, entity_id: &str) -> Option<f32> {
        self.0.get(entity_id).map(Pending::shown)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Record a new value from the control. Returns `Some(value)` when a
    /// send is due now, `None` when the throttle window swallows it.
    ///
    /// A swallowed value is not lost: it stays in `shown`, and
    /// [`Self::release`] always flushes the final one.
    pub fn set(&mut self, entity_id: &str, value: f32, now: Instant) -> Option<f32> {
        let pending = self
            .0
            .entry(entity_id.to_owned())
            .or_insert_with(|| Pending::new(value));
        pending.update(value);

        if pending.send_due(now) {
            pending.record_send(now);
            Some(value)
        } else {
            None
        }
    }

    /// The user released the control. Always returns the final value to
    /// send, so an interaction never ends on a throttled-away
    /// intermediate, and starts the settle window.
    pub fn release(&mut self, entity_id: &str, now: Instant) -> Option<f32> {
        let pending = self.0.get_mut(entity_id)?;
        let value = pending.shown;
        pending.record_send(now);
        pending.release(now);
        Some(value)
    }

    /// Feed in an echoed value from `state_changed`.
    ///
    /// Returns `true` when the entity is now HA-driven again — either
    /// because the echo matched what we sent, or because it had no
    /// pending value to begin with. `false` means the caller must keep
    /// rendering the pending value and ignore this echo.
    pub fn reconcile(&mut self, entity_id: &str, echoed: Option<f32>) -> bool {
        let Some(pending) = self.0.get(entity_id) else {
            return true;
        };

        match echoed {
            Some(value) if pending.reconciles(value) => {
                self.0.remove(entity_id);
                true
            }
            _ => false,
        }
    }

    /// Drop every pending value whose settle window has run out, so HA
    /// truth wins again after a command it clamped or dropped.
    ///
    /// Returns the entities that were retired. The caller needs them
    /// individually: a widget that just lost its pending value is still
    /// displaying that local number, and nothing else will correct it
    /// until the next `state_changed` — which, in exactly the case the
    /// timeout exists for, may never come.
    pub fn expire(&mut self, now: Instant) -> Vec<String> {
        let retired: Vec<String> = self
            .0
            .iter()
            .filter(|(_, pending)| pending.expired(now))
            .map(|(entity_id, _)| entity_id.clone())
            .collect();

        for entity_id in &retired {
            self.0.remove(entity_id);
        }

        retired
    }

    /// Forget an entity entirely, e.g. when its widget or popover closes.
    pub fn clear(&mut self, entity_id: &str) {
        self.0.remove(entity_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t0() -> Instant {
        Instant::now()
    }

    #[test]
    fn first_value_sends_immediately() {
        let mut p = PendingValues::default();
        assert_eq!(p.set("light.a", 100.0, t0()), Some(100.0));
    }

    #[test]
    fn throttle_swallows_sends_inside_the_window() {
        let now = t0();
        let mut p = PendingValues::default();

        assert_eq!(p.set("light.a", 100.0, now), Some(100.0));
        // Still inside SEND_INTERVAL — no wire traffic...
        assert_eq!(
            p.set("light.a", 120.0, now + Duration::from_millis(50)),
            None
        );
        assert_eq!(
            p.set("light.a", 140.0, now + Duration::from_millis(100)),
            None
        );
        // ...but the value is not lost, the control still shows it.
        assert_eq!(p.shown("light.a"), Some(140.0));

        // Past the window, the next move goes out.
        assert_eq!(p.set("light.a", 160.0, now + SEND_INTERVAL), Some(160.0));
    }

    #[test]
    fn release_always_flushes_the_final_value() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", 100.0, now);
        // Throttled away.
        assert_eq!(
            p.set("light.a", 250.0, now + Duration::from_millis(10)),
            None
        );
        // Release must still put 250 on the wire, or the light ends up
        // sitting at a value the user scrubbed past.
        assert_eq!(
            p.release("light.a", now + Duration::from_millis(20)),
            Some(250.0)
        );
    }

    #[test]
    fn matching_echo_hands_control_back_to_ha() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", 100.0, now);
        assert!(!p.reconcile("light.a", Some(40.0)), "stale echo ignored");
        assert_eq!(p.shown("light.a"), Some(100.0));

        assert!(
            p.reconcile("light.a", Some(100.0)),
            "matching echo resolves"
        );
        assert_eq!(p.shown("light.a"), None);
    }

    #[test]
    fn echo_matching_tolerates_float_round_trip() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("climate.a", 21.5, now);
        assert!(p.reconcile("climate.a", Some(21.502)));
    }

    #[test]
    fn entities_without_a_pending_value_are_always_ha_driven() {
        let mut p = PendingValues::default();
        assert!(p.reconcile("sensor.temp", Some(12.0)));
        assert!(p.reconcile("sensor.temp", None));
    }

    #[test]
    fn missing_echo_value_does_not_resolve() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", 100.0, now);
        // A state_changed with no readable numeric value must not be
        // mistaken for confirmation.
        assert!(!p.reconcile("light.a", None));
        assert_eq!(p.shown("light.a"), Some(100.0));
    }

    #[test]
    fn held_control_never_expires() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", 100.0, now);
        // Long past the settle timeout, but the user has not let go.
        assert!(p.expire(now + SETTLE_TIMEOUT * 10).is_empty());
        assert_eq!(p.shown("light.a"), Some(100.0));
    }

    /// One entity expiring must be reported even while others are still
    /// pending — the caller refreshes exactly the retired widgets, and a
    /// widget that isn't named keeps showing a value HA never confirmed.
    #[test]
    fn expiry_reports_each_entity_independently() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", 100.0, now);
        p.release("light.a", now);
        // Second entity grabbed later, so its window has not elapsed.
        p.set("light.b", 50.0, now + SETTLE_TIMEOUT);
        p.release("light.b", now + SETTLE_TIMEOUT);

        let retired = p.expire(now + SETTLE_TIMEOUT);
        assert_eq!(retired, vec!["light.a".to_owned()]);
        assert_eq!(p.shown("light.a"), None);
        assert_eq!(p.shown("light.b"), Some(50.0), "still inside its window");
        assert!(!p.is_empty());
    }

    #[test]
    fn released_value_expires_when_no_echo_matches() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", 100.0, now);
        p.release("light.a", now);

        assert!(
            p.expire(now + SETTLE_TIMEOUT - Duration::from_millis(1))
                .is_empty()
        );
        assert_eq!(p.shown("light.a"), Some(100.0), "still inside the window");

        // HA clamped or dropped the command and will never echo 100.
        assert_eq!(p.expire(now + SETTLE_TIMEOUT), vec!["light.a".to_owned()]);
        assert_eq!(p.shown("light.a"), None, "HA truth wins again");
    }

    #[test]
    fn moving_again_after_release_re_arms_the_interaction() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", 100.0, now);
        p.release("light.a", now);
        // User grabs the slider again before the window elapses.
        p.set("light.a", 200.0, now + SEND_INTERVAL);

        // The old deadline must not still be running.
        assert!(p.expire(now + SETTLE_TIMEOUT).is_empty());
        assert_eq!(p.shown("light.a"), Some(200.0));
    }

    #[test]
    fn clear_forgets_the_entity() {
        let mut p = PendingValues::default();
        p.set("light.a", 100.0, t0());
        p.clear("light.a");
        assert!(p.is_empty());
    }
}
