# Service calls stay on REST

Phase 1 sent Home Assistant service calls over REST and left a note in `src/ha/actions.rs` predicting a migration to the WebSocket once brightness sliders and climate setpoints started producing many calls per second.
We have decided against that migration.
Service calls stay on REST, with a single shared `reqwest::Client` replacing the per-call one.

## Why the prediction did not hold

Two things changed the arithmetic.

Sends during a drag are throttled to roughly one per 200 ms, so the peak is about five calls per second and only while a control is actively being dragged.
That is not the "many calls/sec" the original note was worried about.

Crucially, the throttle is per **entity**, not per axis.
A light with both a brightness and a colour temperature slider still peaks at about five calls per second, because both axes share one send window.
Adding axes therefore does not move this number, which is what makes the decision hold up as the control surface grows.

The real per-call cost was self-inflicted rather than inherent to HTTP.
`call_service` constructed a fresh `reqwest::Client` on every call, so nothing was ever pooled or kept alive and each tap paid for a new TCP and TLS handshake.
Hoisting one shared client removes that cost without touching the transport.

## What the migration would have cost

The note itself enumerates the work: handing an outbound `mpsc::Sender` back out of the iced subscription, tracking monotonic request ids and correlating them with `ServerMsg::Result`, and re-attaching the sender across reconnects.
All three land in the reconnect path of the live state feed, which is the one subsystem that must not break.
Paying that to serve five calls per second was not a trade we wanted.

## Consequences

Service calls and the state feed use different transports, which is a real asymmetry worth knowing about when debugging.

Revisit if measurement shows the REST path is actually a problem, or if the send throttle ever stops being per entity.
Note that the WebSocket would not have helped with echo correlation either, for the reason recorded in `0002-pending-values-and-settle-window.md`.
