# Pending values reconcile against echoes by value, with a settle window

While the user drags a continuous control, the widget renders a local pending value rather than the last state Home Assistant reported.
The pending value stays authoritative until an echo arrives whose value equals the last value we sent, or until a settle window expires.
Only then does Home Assistant truth take over again.

## Why not just render Home Assistant truth

Sends are throttled during a drag, so Home Assistant keeps broadcasting `state_changed` with values that lag the user's finger.
Binding the slider to those events makes it visibly fight the drag, which is the difference between a usable dimmer and an unusable one.

## Why match on value rather than an id

`state_changed` is a broadcast.
It carries no reference back to the service call that caused it, so there is no correlation id to match on even over the WebSocket transport.
Comparing the echoed value to the last value we sent is the only correlation actually available.

Echoes are compared with a small tolerance, because a setpoint round-trips through Home Assistant as a float and may come back in a different representation than we sent.

## Why there is a timeout at all

Value matching alone never terminates when Home Assistant clamps the value, rejects the call, or silently drops it.
Without the settle window the pending value would stay authoritative forever and the widget would quietly lie about the state of the house, which is the one failure mode a dashboard must not have.

The timeout only starts on release.
While the user still holds the control, the pending value cannot expire under their finger.

## Consequences

Every entity being interacted with carries a small state machine, and `apply_entity_state` must consult it before letting a `state_changed` reach the card.
The entity store is still updated either way; only the display is held back, so there is always real state to fall back to the moment the pending value resolves or expires.

Expiry has to be reported per entity rather than as a single flag.
A widget that just lost its pending value is still displaying that local number, and nothing else will correct it: in exactly the case the timeout exists for, Home Assistant has gone quiet and no further echo is coming.
The caller pushes the last known truth back onto each retired widget by hand.

The settle duration is a tunable constant, not a derived value.

## Note for when a second axis lands

The state machine is keyed per axis, so brightness and colour temperature reconcile independently and neither overwrites the other's pending value.
The send throttle is deliberately **not** per axis: it stays per entity, so the peak send rate for one entity is the same with two axes as with one.
That is what keeps the arithmetic in `0003-service-calls-stay-on-rest.md` valid.
