# The colour field is a texture, not a gradient

The colour field a user drags a marker across is painted from an RGBA buffer computed in Rust, once per process, at a fixed 256x128.
It is drawn scaled to whatever the widget's size preset gives it.
It is not two crossed gradients, and it is not a `wgpu` shader.

The obvious implementation is the gradient one, so this records why it was rejected before anyone reaches for it again.

## Why not a gradient

iced's gradients interpolate between stops with `smoothstep`, not linearly, in both the quad and the triangle shader.
`smoothstep(t) = 3t^2 - 2t^3` departs from `t` by up to 9.6% of the segment it spans, at t = 0.211 and its mirror at t = 0.789.

On the saturation axis that is a ten-point error.
At t = 0.211 the shader emits 0.115, so the field shows 11.5% saturation exactly where the marker claims 21%.

On the hue axis it is worse in character if not in magnitude.
The natural stops for a hue ramp are the six vertices of the colour wheel plus the end of the range, which is seven stops and six segments of about 60 degrees, and 9.6% of 60 degrees is about 5.8 degrees.
The error is not spread evenly either: it lands as wide plateaus of pure red, yellow and green sitting on the stops, with the transitions between them compressed.
Subdividing is not a way out, because `iced::gradient::Linear` holds at most eight stops in total.

A brightness slider whose fill is a few percent off is merely ugly.
A colour picker whose marker sits on a different colour from the one it names is wrong, because naming the colour under the marker is the whole job of the widget.
That is the difference between a gradient degrading this control and a gradient disqualifying itself from it.

We did consider approximating our way out by inverting `smoothstep` at each stop so the pre-distorted offsets come back straight.
It amounts to fighting the shader with stop placement, it still has only eight stops to work with, and it leaves the field's accuracy resting on a shader detail we do not control and iced does not promise.

## Why not a wgpu shader

A custom shader primitive would be exact, and it would be exact only on `wgpu`.

iced's default features include the `tiny-skia` software renderer, and that is what runs when there is no usable adapter: a remote desktop session, a virtual machine, an old GPU, a driver that fails to initialise.
On those machines a shader primitive draws nothing at all.
Snapdash is made entirely of these widgets, so "the colour picker is invisible on some machines" is not a hole we can leave open in exchange for avoiding 128 KiB of pixels.

## What the texture is

Hue runs across, from 0 at the left edge to 359 at the right.
It stops one degree short of 360 rather than wrapping, because 360 is the same red the left edge already shows and the axis is easier to reason about as a plain range than as a circle.

Saturation runs down, from 0 at the top row to 100 at the bottom, so the top edge is white and the bottom edge is the pure hues.

Value is pinned to 100 across the whole field.
Brightness is a separate axis with its own control, and a `light.turn_on` that omits `brightness` leaves the lamp where the user put it.
A field that faded to black down one edge would be advertising an axis it does not set.

256x128 is 128 KiB of RGBA, computed once and shared by every widget and every size preset through one image handle.
The renderer keys its upload on that handle, so pinning twenty lights costs one texture, not twenty.

## Consequences

The crate gains one iced feature, `image-without-codecs`, which enables the image widget path and pulls in the `image` crate with `default-features = false`.
No format decoders are compiled in, because nothing is ever decoded: we hand iced bytes we computed ourselves.
If anyone later needs to display an actual PNG, that is the moment to weigh the decoders, not now.

The field is quantised to 256 hues and 128 saturations rather than being continuous.
The residue is bounded and small: half a column is 0.70 degrees of hue, which inside a 60-degree sector is worth about three of 255 levels.
That is two orders of magnitude better than the gradient it replaces, and it is a number that can be improved by making the texture bigger if it ever matters.

The marker's position and the texture's geometry are now two statements of the same mapping, and they have to agree.
Anything that changes the axes has to change both, which is why the extents live as named constants in `ui::colour_texture` rather than as literals at the drawing site.

Scaling is the renderer's, so the field is interpolated up from 256x128 to whatever the widget is.
That is fine for a colour field, where neighbouring pixels are near-identical by construction, and it is the reason the texture can be small enough to keep.
