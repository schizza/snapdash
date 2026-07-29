//! Pending values: the local override that wins over Home Assistant
//! truth while the user is driving a control.
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
//! Echoes are compared to the last sent value with a tolerance, and
//! that tolerance belongs to the **axis** rather than being one global
//! constant. An axis whose value passes through a unit conversion on the
//! way back does not return the number we sent, and how much it loses is
//! a fact about that conversion, not about floats. See
//! [`AxisKind::echo_tolerance`].
//!
//! State is kept **per axis**, so brightness and colour temperature
//! reconcile independently and neither overwrites the other. Throttling
//! is kept **per entity**, so a light with two sliders still peaks at
//! the same call rate as one with a single slider. That is what keeps
//! the arithmetic in `docs/adr/0003-service-calls-stay-on-rest.md` valid
//! as axes are added.
//!
//! Because of that split, the unit both [`PendingValues::set`] and
//! [`PendingValues::release`] take is the **gesture**: a batch of axes,
//! never a single one. One gesture spends one throttle window, so the
//! window has to be consulted once for everything it moved. A per-axis
//! entry point would have let a caller spend it on the first axis and
//! silently record nothing sent for the rest, which is a bug the type
//! system could not catch and only shows up as an axis that never
//! reconciles. There is one way in so the two cannot drift apart.
//!
//! Recorded in `docs/adr/0002-pending-values-and-settle-window.md`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::ha::AxisKind;

/// Minimum gap between service calls for one entity while it is being
/// driven. Caps the send rate at ~5/sec, which is what keeps REST viable
/// as the transport (`docs/adr/0003-service-calls-stay-on-rest.md`).
pub const SEND_INTERVAL: Duration = Duration::from_millis(200);

/// How long a released value stays authoritative while waiting for a
/// matching echo before HA truth is allowed to win again.
pub const SETTLE_TIMEOUT: Duration = Duration::from_secs(2);

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
    ///
    /// `kind` decides how much slack the comparison gets, because the
    /// round trip through Home Assistant is lossy by different amounts
    /// for different axes.
    fn reconciles(&self, kind: AxisKind, echoed: f32) -> bool {
        self.last_sent
            .is_some_and(|sent| (sent - echoed).abs() <= kind.echo_tolerance())
    }

    /// `true` once the settle window has run out. Never true while the
    /// user still holds the control.
    fn expired(&self, now: Instant) -> bool {
        self.settle_deadline.is_some_and(|deadline| now >= deadline)
    }
}

type AxisKey = (String, AxisKind);

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
    pub fn shown(&self, entity_id: &str, kind: AxisKind) -> Option<f32> {
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

    /// Record the new values one gesture produced, and say whether they
    /// are due to go out now. `false` means the entity's throttle window
    /// swallowed this batch.
    ///
    /// Nothing is returned but the verdict, because the caller is holding
    /// the values it just passed in.
    ///
    /// A swallowed batch is not lost: every value stays in `shown`, and
    /// [`Self::release`] flushes the final ones.
    pub fn set(&mut self, entity_id: &str, updates: &[(AxisKind, f32)], now: Instant) -> bool {
        if updates.is_empty() {
            return false;
        }

        // Asked once, for the batch. The throttle window is spent by the
        // gesture and not by the axis, so asking per axis would let the
        // first one spend it and leave every other one recording nothing
        // but `shown`. Those axes would have no `last_sent` to recognise
        // their echo against, would run to the settle timeout on every
        // interaction, and - since an entity only goes back to being
        // HA-driven when none of its axes are pending - would hold their
        // siblings there too.
        let due = self.send_due(entity_id, now);

        for (kind, value) in updates {
            let pending = self
                .axes
                .entry((entity_id.to_owned(), *kind))
                .or_insert_with(|| Pending::new(*value));
            pending.update(*value);

            if due {
                pending.last_sent = Some(*value);
            }
        }

        if due {
            self.last_send_at.insert(entity_id.to_owned(), now);
        }

        due
    }

    /// The user let go of the control. Always returns the final value of
    /// every axis it was driving, so an interaction never ends on a
    /// throttled-away intermediate, and starts the settle window on each.
    ///
    /// The values have to come back, unlike in [`Self::set`]: the last
    /// one the caller saw may well have been swallowed by the throttle,
    /// so what the control is showing is not the caller's to know.
    ///
    /// `None` when this entity has nothing pending on any of these axes,
    /// which is the case for a release that follows a reconciled echo.
    pub fn release(
        &mut self,
        entity_id: &str,
        axes: &[AxisKind],
        now: Instant,
    ) -> Option<Vec<(AxisKind, f32)>> {
        let mut flushed = Vec::with_capacity(axes.len());

        for kind in axes {
            let Some(pending) = self.axes.get_mut(&(entity_id.to_owned(), *kind)) else {
                continue;
            };
            let value = pending.shown;

            pending.last_sent = Some(value);
            pending.settle_deadline = Some(now + SETTLE_TIMEOUT);
            flushed.push((*kind, value));
        }

        if flushed.is_empty() {
            return None;
        }

        self.last_send_at.insert(entity_id.to_owned(), now);
        Some(flushed)
    }

    /// Feed in the values an entity's `state_changed` carries, one per
    /// axis it reports.
    ///
    /// Returns `true` when the entity is HA-driven again, meaning no axis
    /// of it is still waiting. `false` means the caller must keep
    /// rendering the pending values and ignore this echo, because
    /// applying it would drag a slider back under the user's finger.
    pub fn reconcile(&mut self, entity_id: &str, echoed: &[(AxisKind, Option<f32>)]) -> bool {
        for (kind, value) in echoed {
            let key = (entity_id.to_owned(), *kind);
            let Some(pending) = self.axes.get(&key) else {
                continue;
            };

            if value.is_some_and(|value| pending.reconciles(*kind, value)) {
                self.axes.remove(&key);
            }
        }

        // The throttle timestamp deliberately outlives the interaction it
        // was set by. On a LAN the echo comes back well inside
        // `SEND_INTERVAL`, so clearing it here would let every echo reopen
        // the window mid-drag and the call rate would track the UI event
        // rate instead of the ~5/sec ADR-0003 is costed against. It is
        // dropped once the interaction is truly over, in [`Self::expire`]
        // and [`Self::clear`], and a stale one can only ever throttle, it
        // can never let an extra call through.
        !self.axes.keys().any(|(id, _)| id == entity_id)
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

    const BRIGHTNESS: AxisKind = AxisKind::Brightness;
    const TEMP: AxisKind = AxisKind::ColorTemp;

    fn t0() -> Instant {
        Instant::now()
    }

    #[test]
    fn first_value_sends_immediately() {
        let mut p = PendingValues::default();
        assert!(p.set("light.a", &[(BRIGHTNESS, 100.0)], t0()));
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(100.0));
    }

    #[test]
    fn throttle_swallows_sends_inside_the_window() {
        let now = t0();
        let mut p = PendingValues::default();

        assert!(p.set("light.a", &[(BRIGHTNESS, 100.0)], now));
        // Still inside SEND_INTERVAL, so no wire traffic...
        assert!(!p.set(
            "light.a",
            &[(BRIGHTNESS, 120.0)],
            now + Duration::from_millis(50)
        ));
        assert!(!p.set(
            "light.a",
            &[(BRIGHTNESS, 140.0)],
            now + Duration::from_millis(100)
        ));
        // ...but the value is not lost, the control still shows it.
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(140.0));

        // Past the window, the next move goes out.
        assert!(p.set("light.a", &[(BRIGHTNESS, 160.0)], now + SEND_INTERVAL));
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(160.0));
    }

    /// The throttle is per entity, not per axis. Two sliders on one light
    /// must not double its call rate, or the ~5/sec ceiling that keeps
    /// REST viable (ADR-0003) stops holding as axes are added.
    #[test]
    fn axes_of_one_entity_share_the_send_throttle() {
        let now = t0();
        let mut p = PendingValues::default();

        assert!(p.set("light.a", &[(BRIGHTNESS, 100.0)], now));
        // A different axis of the same light, immediately after.
        assert!(
            !p.set(
                "light.a",
                &[(TEMP, 3000.0)],
                now + Duration::from_millis(10)
            ),
            "shares the entity's window"
        );
        // A different entity is unaffected.
        assert!(p.set(
            "light.b",
            &[(BRIGHTNESS, 50.0)],
            now + Duration::from_millis(10)
        ));
    }

    /// The bug this batch API exists to make unrepresentable. One gesture
    /// spends one throttle window, so every axis it moved has to come out
    /// of it able to recognise its own echo. Set once per axis instead and
    /// the second axis records only what it shows: it never reconciles,
    /// runs to the settle timeout every time, and holds the whole entity
    /// - the first axis included - there with it.
    #[test]
    fn a_batch_that_goes_out_records_the_sent_value_on_every_axis() {
        let now = t0();
        let mut p = PendingValues::default();

        assert!(p.set("light.a", &[(BRIGHTNESS, 100.0), (TEMP, 3000.0)], now));

        assert!(
            p.reconcile(
                "light.a",
                &[(BRIGHTNESS, Some(100.0)), (TEMP, Some(3000.0))]
            ),
            "both axes must recognise the echo of what the batch sent"
        );
        assert!(p.is_empty());
    }

    /// A batch the window swallows is still what the controls show, all
    /// of it: the axes of one gesture cannot be allowed to disagree about
    /// where the user's finger is.
    #[test]
    fn a_throttled_batch_still_records_every_axis_as_shown() {
        let now = t0();
        let mut p = PendingValues::default();

        assert!(p.set("light.a", &[(BRIGHTNESS, 100.0), (TEMP, 3000.0)], now));
        assert!(!p.set(
            "light.a",
            &[(BRIGHTNESS, 120.0), (TEMP, 4000.0)],
            now + Duration::from_millis(50)
        ));

        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(120.0));
        assert_eq!(p.shown("light.a", TEMP), Some(4000.0));
    }

    /// The window is consumed by the gesture, not by the axis. A batch of
    /// two must leave the entity exactly as throttled as a batch of one,
    /// or the peak call rate would grow with every axis a control gains
    /// and ADR-0003's arithmetic would stop holding.
    #[test]
    fn a_batch_consults_the_throttle_once_however_many_axes_it_moves() {
        let now = t0();
        let mut p = PendingValues::default();

        assert!(p.set("light.a", &[(BRIGHTNESS, 100.0), (TEMP, 3000.0)], now));

        assert!(
            !p.send_due("light.a", now + SEND_INTERVAL - Duration::from_millis(1)),
            "one batch, one window"
        );
        assert!(p.send_due("light.a", now + SEND_INTERVAL));
    }

    #[test]
    fn release_always_flushes_the_final_value() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        // Throttled away.
        assert!(!p.set(
            "light.a",
            &[(BRIGHTNESS, 250.0)],
            now + Duration::from_millis(10)
        ));
        // Release must still put 250 on the wire, or the light ends up
        // sitting at a value the user scrubbed past.
        assert_eq!(
            p.release("light.a", &[BRIGHTNESS], now + Duration::from_millis(20)),
            Some(vec![(BRIGHTNESS, 250.0)])
        );
    }

    /// Release answers for the whole gesture, in the order it was asked,
    /// so the caller can put every axis of it on the wire.
    #[test]
    fn release_flushes_every_axis_of_the_gesture() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(BRIGHTNESS, 100.0), (TEMP, 3000.0)], now);

        assert_eq!(
            p.release("light.a", &[BRIGHTNESS, TEMP], now),
            Some(vec![(BRIGHTNESS, 100.0), (TEMP, 3000.0)])
        );
    }

    /// An axis with nothing pending has nothing to flush, and must not
    /// invent a value for one that has.
    #[test]
    fn release_skips_axes_that_are_not_pending() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);

        assert_eq!(
            p.release("light.a", &[BRIGHTNESS, TEMP], now),
            Some(vec![(BRIGHTNESS, 100.0)])
        );
        assert_eq!(p.release("light.a", &[TEMP], now), None);
    }

    #[test]
    fn matching_echo_hands_control_back_to_ha() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
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

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        p.set("light.a", &[(TEMP, 3000.0)], now + SEND_INTERVAL);

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

    /// Home Assistant echoes in tens of milliseconds on a LAN, well
    /// inside `SEND_INTERVAL`. If reconciling reopened the entity's
    /// throttle window, every echo would let the next slider event
    /// straight through and the call rate would track the user's finger
    /// rather than the ~5/sec ADR-0003 is costed against.
    #[test]
    fn an_echo_mid_drag_does_not_reopen_the_throttle() {
        let now = t0();
        let mut p = PendingValues::default();

        assert!(p.set("light.a", &[(BRIGHTNESS, 100.0)], now));

        // HA confirms almost immediately, so the axis is no longer waiting.
        assert!(p.reconcile("light.a", &[(BRIGHTNESS, Some(100.0))]));
        assert!(p.is_empty());

        // The user has not let go, and the window has not elapsed.
        assert!(
            !p.set(
                "light.a",
                &[(BRIGHTNESS, 120.0)],
                now + Duration::from_millis(30)
            ),
            "the echo must not have reopened the window"
        );

        assert!(
            p.set("light.a", &[(BRIGHTNESS, 140.0)], now + SEND_INTERVAL),
            "and it still opens on time"
        );
    }

    /// Once the interaction is genuinely over the timestamp goes, so a
    /// widget picked up much later is not throttled by a stale one. Both
    /// exits have to do it: the settle timeout and an explicit clear.
    #[test]
    fn a_finished_interaction_drops_its_throttle() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        p.release("light.a", &[BRIGHTNESS], now);
        assert_eq!(p.expire(now + SETTLE_TIMEOUT), vec!["light.a".to_owned()]);
        assert!(p.send_due("light.a", now + SETTLE_TIMEOUT));

        p.set("light.b", &[(BRIGHTNESS, 50.0)], now);
        p.clear("light.b");
        assert!(p.send_due("light.b", now));
    }

    #[test]
    fn echo_matching_tolerates_float_round_trip() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("climate.a", &[(AxisKind::Temperature, 21.5)], now);
        assert!(p.reconcile("climate.a", &[(AxisKind::Temperature, Some(21.502))]));
    }

    /// A light that stores mireds internally round-trips kelvin through
    /// `round(1_000_000 / kelvin)` and back, so the echo is never the
    /// number we sent. 6350 K comes back as 6369 K, the worst case over
    /// the 50 K grid Snapdash sends. Against the old global 0.01 this
    /// could not match at all, and the axis ran to the settle timeout on
    /// every single interaction.
    #[test]
    fn colour_temperature_reconciles_across_the_mired_round_trip() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(TEMP, 6350.0)], now);
        assert!(p.reconcile("light.a", &[(TEMP, Some(6369.0))]));
    }

    /// The slack must stay under the 50 K step, or a stop could be
    /// confirmed by its neighbour and the slider would silently accept
    /// a value the user did not ask for.
    #[test]
    fn colour_temperature_does_not_reconcile_against_a_neighbouring_stop() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(TEMP, 3000.0)], now);
        assert!(!p.reconcile("light.a", &[(TEMP, Some(3050.0))]));
        assert_eq!(p.shown("light.a", TEMP), Some(3000.0));
    }

    /// The slack is colour temperature's, not everybody's. Brightness
    /// makes the round trip untouched, so 100 is not 110.
    #[test]
    fn brightness_keeps_the_tight_tolerance() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        assert!(!p.reconcile("light.a", &[(BRIGHTNESS, Some(110.0))]));
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

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        // A state_changed with no readable value for the axis must not be
        // mistaken for confirmation.
        assert!(!p.reconcile("light.a", &[(BRIGHTNESS, None)]));
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(100.0));
    }

    #[test]
    fn held_control_never_expires() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
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

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        p.release("light.a", &[BRIGHTNESS], now);
        // Second entity grabbed later, so its window has not elapsed.
        p.set("light.b", &[(BRIGHTNESS, 50.0)], now + SETTLE_TIMEOUT);
        p.release("light.b", &[BRIGHTNESS], now + SETTLE_TIMEOUT);

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

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        p.release("light.a", &[BRIGHTNESS], now);
        p.set("light.a", &[(TEMP, 3000.0)], now);
        p.release("light.a", &[TEMP], now);

        assert_eq!(p.expire(now + SETTLE_TIMEOUT), vec!["light.a".to_owned()]);
        assert!(p.is_empty());
    }

    #[test]
    fn released_value_expires_when_no_echo_matches() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        p.release("light.a", &[BRIGHTNESS], now);

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

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        p.release("light.a", &[BRIGHTNESS], now);
        // User grabs the slider again before the window elapses.
        p.set("light.a", &[(BRIGHTNESS, 200.0)], now + SEND_INTERVAL);

        // The old deadline must not still be running.
        assert!(p.expire(now + SETTLE_TIMEOUT).is_empty());
        assert_eq!(p.shown("light.a", BRIGHTNESS), Some(200.0));
    }

    #[test]
    fn clear_forgets_every_axis_of_the_entity() {
        let now = t0();
        let mut p = PendingValues::default();

        p.set("light.a", &[(BRIGHTNESS, 100.0)], now);
        p.set("light.a", &[(TEMP, 3000.0)], now);
        p.set("light.b", &[(BRIGHTNESS, 50.0)], now);

        p.clear("light.a");

        assert_eq!(p.shown("light.a", BRIGHTNESS), None);
        assert_eq!(p.shown("light.a", TEMP), None);
        assert_eq!(p.shown("light.b", BRIGHTNESS), Some(50.0), "untouched");
    }
}
