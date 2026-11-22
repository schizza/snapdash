# An axis Home Assistant reports as null renders as absent, not as minimum

Home Assistant nulls an axis the device is not currently driving.
A light that is off nulls `brightness`, `color_temp_kelvin` and every colour attribute at once; a light in some other colour mode nulls `color_temp_kelvin` on its own.

When that happens the control renders as *absent*: the block drops to around 40% opacity, the readout shows no number, and the slider knob is not drawn at all.
The control stays fully operable, and touching it is how the axis gets a value again.

## Why not fall back to the axis minimum

That is what shipped first, and it is a lie with no tell.
A knob parked hard left over a "0%" readout is the same picture a light genuinely dimmed to zero produces, so the user has nothing to tell "off" from "on but dark" with.
"There is no brightness" and "the brightness is zero" are different claims, and only one of them is true.

## Why not remember the last non-null value

It reintroduces the same problem one step removed.
Showing 4000 K on a light currently glowing blue is a statement about the past presented as a statement about the present, with nothing in the widget marking it as history.
A dashboard's whole job is to be the state of the house right now.

## Why a missing knob rather than a differently-styled one

The knob is the mark that asserts a position on the rail.
Any knob that is drawn is somewhere, and wherever it is drawn is a value the user will read off it.
Removing it is the only rendering that makes no claim at all.

`iced::widget::slider` can hide its knob by giving the handle a transparent background, so the slider keeps its geometry and its hit area, and only the mark goes away.
No custom widget is needed.

## Consequences

Value resolution answers `Option<f32>` rather than a number, and every control surface has to decide what absent looks like for it.
A colour surface inherits the same rule: its marker is not drawn either.

The pending value still wins while the user drives the axis, so an axis with no Home Assistant value shows what the user is setting the moment they grab it, and goes back to absent if the send never lands.
