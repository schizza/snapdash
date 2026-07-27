# The whole card is the drag handle, the action lives in a header icon

Issue #86 asked for the whole widget to be tappable, with drag-gesture detection deciding between firing the action and moving the window.
We built a hit-region split towards that end (the title row as drag handle, the value area as tap target) and then reverted it.
The card is one surface with one job: it drags.
The primary action stays in an icon in the header row, next to the chevron that expands the continuous controls.

## Why not gesture detection

Gesture detection requires deferring the call to `winit::Window::drag_window` until after the pointer has moved past a threshold.
winit explicitly does not support this:

> Moves the window with the left mouse button until the button is released.
> There's no guarantee that this will work unless the left mouse button was pressed immediately before this function is called.
>
> **macOS:** May prevent the button release event to be triggered.

So the deferred handoff is outside the documented contract on every backend, and the macOS note means a classifier that waits for the release to decide "that was a tap" can hang.

## Why not the hit-region split either

Splitting the card into a drag region and a tap region avoids the winit problem, because neither region ever has to decide what a press meant.
We shipped it far enough to use it, and it traded one problem for three.

The drag handle shrank to the title row, which is about 20px tall on the 160x110 Small preset.
Moving a widget stopped being a grab and became a precision task.

The persistent icon that said "this widget does something", and named which action through its tooltip, disappeared.
A bare card body advertises nothing, so the widget lost the only signal that distinguished an actionable widget from a read-only one.

Tapping the card body fired the action by accident, because the body is most of the card and there was no longer anywhere safe to click.

Each of those is individually fixable and collectively not.
Making the drag region bigger makes the accidental-fire region bigger.
Making the tap region smaller turns it back into a button, which is where we started.

## The decision

The card body drags, exactly as it did before any of this.
The action is a header icon with a tooltip naming what it will do, which is both the affordance and the signal.
Continuous controls do not live on the card surface at all: the widget expands to reveal them, which is recorded separately.

## Consequences

There is no gesture classifier, no movement threshold, no deferred handoff and no timer anywhere in the interaction, so the widget behaves identically on macOS, Windows, X11 and Wayland.

The header row now carries the title, the action icon, the expand chevron and, when relevant, the update alert.
At the Small preset that leaves roughly 76px for the title, so titles truncate sooner than they used to.
That is the cost of keeping both affordances visible at rest rather than hiding them behind hover.

Issue #86 is closed as `wontfix`.
Reopening it is only worth it after the drag handoff is hardened in the `schizza/iced` fork, because every route to literal tap-anywhere runs through the winit constraint above.
