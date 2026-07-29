# A Control may drive several Axes, and owns drawing, sending and echo recognition

An **Axis** is one numeric dimension of an entity: it owns a range, the value Home Assistant reports for it, and one scalar pending value.
A **Control** is the thing the user grabs.
It owns three things: how it draws, what it puts on the wire, and how it recognises its own echo.

A Control may drive more than one Axis.
An Axis need not have a Control of its own.

## Why they were one thing, and why they stop being one

Until the colour surface arrived, every control drove exactly one axis.
Brightness, white colour temperature, a thermostat setpoint and a cover position are all a label, a range and a slider, so a single type carrying all of it was not wrong, it was just not yet distinguishable from the alternative.

A colour surface is one control that sets hue and saturation with one gesture and one `light.turn_on` call.
Hue is a numeric dimension and saturation is another, exactly as `CONTEXT.md` defines an Axis, so the answer is not "an axis that carries two numbers".
The answer is that the control and the axis were never the same concept, and the code had simply never had to tell them apart.

## Why an enum rather than a grouping marker

`Control` is an enum whose variants name their shape, rather than a flat list of axes with a marker saying which of them belong together.

A marker permits a hue with no saturation beside it, or an orphan hue rendered in a slider of its own, and nothing in the type system would catch either.
`AxisKind::Hue` and `AxisKind::Saturation` exist only inside `Control::Color`, which makes the invalid arrangements unrepresentable rather than merely undocumented.

## Why the Control owns echo recognition

`0002-pending-values-and-settle-window.md` records that echoes are matched by value, because `state_changed` carries no correlation id.
How much slack that comparison needs is not a property of the machinery doing the comparing.
It is a property of what the value passes through on the way back.

Colour temperature round-trips through mireds and comes back up to 19 K away from what was sent.
A colour round-trips through 8-bit RGB and comes back up to 19.9 degrees of hue away at low saturation, because at saturation 1 hue is barely determined in 8 bits at all.
Brightness and cover position are stored in the units we send and come back untouched.

So the colour control does not compare hue and saturation numerically at all.
It converts both the sent and the echoed colour to 8-bit RGB and compares those, which is comparing what the user actually sees rather than the coordinates the colour happens to be written in.
That dissolves the low-saturation ambiguity for free, and it removes any need for hue wraparound handling, since hue 0 and hue 360 produce identical RGB.

The general rule is what matters here rather than the particular numbers: the tolerance belongs to the Control, alongside sending and drawing.

## What comparing in RGB does not fix

A light whose native mode is `xy`, which includes Philips Hue bulbs, does not lose colour to quantisation.
It loses it to a gamut conversion, and that is a different kind of loss.
Measured over the whole wheel at full saturation, roughly a third of it comes back more than half a degree out, worst case 2.7 degrees at hue 242.
At full saturation 2.7 degrees of hue is about eleven levels of an 8-bit channel, so an RGB comparison tight enough to reject a colour the user did not pick will not confirm it either.

Those lights therefore reconcile on the settle window rather than on the echo, which is the path `0002-pending-values-and-settle-window.md` describes as the failure path.
That is a known and accepted limitation rather than an oversight: the alternative is a tolerance wide enough to confirm a visibly different colour, which is worse, because it would leave the widget claiming a colour the house is not showing.
Closing it properly means comparing against what the device reports it is actually doing rather than against what it was told to do, and that is a larger change than a tolerance.

## Consequences

The expanded widget renders by iterating controls, and its height is the sum of per-control heights rather than one row height multiplied by a count.
Controls are not all the same height: a colour surface is a field, not a slider.

`PendingValues` takes a batch of axes per gesture rather than one axis per call.
A gesture that moves two axes must consume the entity's throttle window once, not twice; the second call would otherwise find the window spent, record what is displayed, and silently never record what was sent, leaving that axis unable to ever recognise its echo.
That is recorded in the module documentation of `src/app/pending.rs`, and the batch is the only way in so that a scalar path and a batch path cannot drift apart.

The throttle stays keyed per entity at 200 ms.
Adding axes must not move the peak send rate, which is the invariant the arithmetic in `0003-service-calls-stay-on-rest.md` rests on.

Asking a Control to recognise its echo means asking about three states per axis and not two.
A per-entity throttle plus more than one Control per entity makes "held by the user with nothing yet on the wire" reachable: grabbing a second control inside the window the first one spent records what is shown and nothing sent.
That axis has no last-sent value, exactly like an axis nobody is touching, and the two need opposite verdicts.
An untouched axis is vacuously confirmed, because there is nothing for the echo to disagree with; a held one must never be, because releasing it would drop a gesture in progress and the value the user finally chose would never be sent at all.
So `Control::reconciles` is asked for `Outstanding`, whose three variants make the distinction one the type carries rather than one every caller has to remember.
