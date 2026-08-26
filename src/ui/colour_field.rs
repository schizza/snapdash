//! The colour surface: one drag sets hue and saturation together.
//!
//! ## Why a widget of its own
//!
//! `iced::widget::slider` is one-dimensional by construction - its
//! value is a scalar and its event handler reads only the horizontal
//! offset of the cursor - so two of them side by side would be two
//! gestures and two service calls for one colour. And `mouse_area`
//! reports that something was pressed but not *where*: its callbacks
//! carry no cursor position at all, let alone one relative to the
//! widget's bounds, which is the only thing a colour field needs to
//! know.
//!
//! ## What lives here and what does not
//!
//! The mapping from a pointer to a colour is [`colour_at`], a free
//! function of geometry that the event handler calls. Inside the handler
//! it could not be tested at all: reaching it means an event loop, a
//! window and a renderer, and the places it is most likely to be wrong -
//! the corners of the field and the exact extents of the two axes - are
//! the cheapest possible assertions once it is out.
//!
//! The pixels are `ui::colour_texture`'s, and the only thing this file
//! knows about them is that they are laid out the way [`colour_at`] maps
//! a pointer: hue across, saturation down, both inclusive of their ends.
//! The two are one statement of the same mapping made twice, so they
//! share the extents rather than each spelling them out
//! (`docs/adr/0005-the-colour-field-is-a-texture.md`).
//!
//! ## Precision
//!
//! At Small the field is 132 points across 359 degrees, which is 2.72
//! degrees under every point of movement. Four things answer that, and
//! all four are functions of the current frame rather than modes:
//! [`SATURATION_MAGNET`] pulls the last few points at each end of the
//! saturation axis onto exactly 0 and exactly 100, Shift holds whichever
//! axis has moved less since the press, Alt keeps [`FINE_SCALE`] of the
//! movement, and the wheel nudges a step at a time.
//!
//! What is deliberately absent is an automatic directional lock. One
//! that guesses wrong presents as the control having stopped responding,
//! which is indistinguishable from a defect, and escaping it means
//! releasing and pressing again with nothing anywhere to say so. It is
//! exactly the hidden in-gesture classifier
//! `docs/adr/0001-widget-interaction-model.md` exists to keep out. A held
//! key cannot get stuck and cannot surprise, because letting go of it is
//! the whole of undoing it.

use iced::advanced::image;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget, tree};
use iced::keyboard::{self, Modifiers};
use iced::{Color, Element, Length, Point, Radians, Rectangle, Size, border, mouse, touch};

use crate::ui::colour_texture::{self, MAX_HUE, MAX_SATURATION};

/// The precision shortcuts, in the words the field is advertised by.
///
/// Shift, Alt and the wheel are invisible unless something says they
/// exist, so `ui::entity_window` hangs this off a help icon in the
/// colour control's own label row (#100). It lives here, beside
/// [`constrained`] and [`nudged`], because it is a statement about what
/// they do: a wording that drifts from them is a lie the user is told at
/// the exact moment they went looking for the truth.
///
/// Four short lines rather than four sentences. The tooltip is bounded
/// by the widget's own window, which is 160 points wide at Small and
/// leaves about 126 for text, and every line here fits inside that at
/// the size `components::tooltip_message` draws them. The precise
/// statements - *which* axis Shift holds, how much finer Alt is - are
/// the README's job, because a tooltip is read in the second before a
/// gesture and not studied.
///
/// The saturation magnets are deliberately absent. They need no holding
/// down and no discovering: they happen on their own, at the two ends of
/// an axis the user was already dragging towards.
pub const SHORTCUTS: &str = "Shift holds an axis\n\
                             Alt drags finer\n\
                             Wheel nudges hue\n\
                             With Shift, saturation";

/// The colour under a pointer, given where the field is and where the
/// gesture started.
///
/// Hue runs left to right from 0 to [`MAX_HUE`] and saturation top to
/// bottom from 0 to [`MAX_SATURATION`], which is the arrangement
/// `ui::colour_texture` computes its pixels in: white along the top
/// edge, fully saturated along the bottom, red at both ends because the
/// two ends of a wheel are the same colour.
///
/// Both come back on the whole-number steps the axes carry, so what the
/// readout says, what the pending value holds and what goes on the wire
/// are one number rather than three roundings of one.
///
/// A cursor outside `bounds` clamps rather than being rejected, and that
/// is the whole of "the drag keeps tracking when the pointer leaves the
/// field": there is no outside to handle, only an edge to sit on.
///
/// `modifiers` and `pressed_at` are read by [`constrained`], which is
/// where Shift and Alt live. Both are arguments rather than remembered
/// state, and that is the whole of "no state persists across a gesture":
/// every frame is answered from the modifiers of that frame, so a
/// modifier released mid-drag stops applying on the next pointer move
/// with nothing to reset and nothing that can get stuck.
pub fn colour_at(
    bounds: Rectangle,
    cursor: Point,
    modifiers: Modifiers,
    pressed_at: Point,
) -> (f32, f32) {
    let point = constrained(cursor, modifiers, pressed_at);

    let across = fraction(point.x - bounds.x, bounds.width);
    let down = magnetised(point.y - bounds.y, bounds.height);

    ((across * MAX_HUE).round(), (down * MAX_SATURATION).round())
}

/// The point the held modifiers say the gesture is really at.
///
/// Both modifiers are answered by moving the point rather than by
/// branching on the colour it produces, and that is what makes them
/// compose: Alt shortens the displacement from the press, Shift throws
/// one component of that displacement away, and holding both does both
/// because the two edits are edits of the same thing. A third branch for
/// "Shift and Alt" would be a third behaviour to get wrong.
fn constrained(cursor: Point, modifiers: Modifiers, pressed_at: Point) -> Point {
    let moved = cursor - pressed_at;

    // Alt keeps a quarter of the movement since the press rather than a
    // quarter of the position, so the fine gesture carries on from where
    // the coarse one had got to instead of teleporting to a quarter of
    // the way across the field the moment the key goes down.
    let mut point = pressed_at + moved * if modifiers.alt() { FINE_SCALE } else { 1.0 };

    if modifiers.shift() {
        // "Moved less" is measured in points of pointer movement, not in
        // degrees and percent. The two are different questions here,
        // because the field is twice as wide as it is tall while hue
        // spans 359 against saturation's 100: a point of sideways
        // movement is worth 2.1 degrees and a point of downward movement
        // 1.2 percent, so measuring in units would call almost every
        // diagonal drag a hue drag and lock saturation nearly always.
        // Degrees and percent are not commensurable anyway - there is no
        // honest sense in which 20 degrees is more or less than 12
        // percent - whereas the hand really did move some number of
        // points one way and some number the other. The lock is a
        // reading of the gesture, so it is measured in what the gesture
        // is made of.
        //
        // A tie locks saturation, so that the one drag the lock is
        // mostly wanted for - holding a saturation while sweeping the
        // hue - is the one a dead heat resolves towards.
        if moved.x.abs() >= moved.y.abs() {
            point.y = pressed_at.y;
        } else {
            point.x = pressed_at.x;
        }
    }

    point
}

/// The colour `steps` of the wheel move `colour` to: hue, or saturation
/// while Shift is held.
///
/// Up the wheel is up the axis on both, rather than up the wheel being
/// up the *field*. The wheel nudges a number the readout is showing, not
/// the marker, and "scroll up for more" is what every other control that
/// answers a wheel does. Reading it spatially would make Shift and the
/// wheel upwards desaturate, which is backwards for a control whose
/// whole readout is "hue degrees, saturation percent".
///
/// The result is on whole steps whichever axis moved, because the two
/// leave together as one `hs_color` and neither the readout nor the
/// service call has anywhere to put a fraction.
fn nudged(colour: (f32, f32), steps: f32, modifiers: Modifiers) -> (f32, f32) {
    let (hue, saturation) = (colour.0.round(), colour.1.round());

    if modifiers.shift() {
        (hue, (saturation + steps).clamp(0.0, MAX_SATURATION))
    } else {
        ((hue + steps).clamp(0.0, MAX_HUE), saturation)
    }
}

/// How many wheel detents a scroll event is worth, which may well be a
/// fraction of one.
///
/// The vertical component when there is one and the horizontal one
/// otherwise, because the field has exactly one meaning for a scroll and
/// the modifier rather than the direction of the swipe chooses the axis
/// it lands on. Falling back on the horizontal component is not
/// tidiness: macOS moves a shift-held wheel onto it, so on that platform
/// it carries the entire "Shift and wheel nudges saturation" gesture.
/// `iced::widget::scrollable` compensates for the same thing by swapping
/// the two, which is the same fact stated for a widget that has two
/// directions to tell apart.
///
/// Preferring the vertical component rather than adding the two is what
/// keeps the sideways wobble of a two-finger swipe from cancelling the
/// swipe out.
fn notches(delta: mouse::ScrollDelta) -> f32 {
    let (x, y, per_notch) = match delta {
        mouse::ScrollDelta::Lines { x, y } => (x, y, 1.0),
        mouse::ScrollDelta::Pixels { x, y } => (x, y, PIXELS_PER_NOTCH),
    };

    (if y == 0.0 { x } else { y }) / per_notch
}

/// Pixels of a precise scroll that make one detent.
///
/// A trackpad has no detents to report, so it reports the distance the
/// fingers travelled and leaves the quantising to whoever wants it. Sixty
/// is what iced's own scrollable calls a line, so one step of the colour
/// field costs the same swipe as one line of scrolling anything else -
/// the hand already knows how far that is.
///
/// Erring coarse is deliberate. A trackpad emits scroll events at the
/// refresh rate, and every step this widget hands out is a `light.turn_on`
/// that the throttle in `app::pending` does not cover, because a nudge
/// is released as soon as it is made. A generous notch is what keeps a
/// flick over the field from being a hundred service calls.
const PIXELS_PER_NOTCH: f32 = 60.0;

/// Where a colour sits in the field, which is [`colour_at`] read
/// backwards and how the marker finds its place.
fn position_of(bounds: Rectangle, hue: f32, saturation: f32) -> Point {
    Point::new(
        bounds.x + fraction(hue, MAX_HUE) * bounds.width,
        bounds.y + fraction(saturation, MAX_SATURATION) * bounds.height,
    )
}

/// How far down the saturation axis an offset sits, as 0..=1, with the
/// two ends of the axis pulling the last [`SATURATION_MAGNET`] points
/// onto themselves.
///
/// Only saturation has magnets. Its ends are two colours a user asks for
/// by name - white and fully saturated - and both are one pixel wide
/// without help. Hue's ends are not: 0 and 359 are the same red, so
/// there is nothing at either end worth snapping to, and a magnet there
/// would only make the reds harder to tell apart.
fn magnetised(offset: f32, extent: f32) -> f32 {
    // Asked before either edge, because a field with no extent has no
    // two edges to be near: without this the bottom test would answer 1
    // for any offset at all and a degenerate layout would report full
    // saturation.
    if extent <= 0.0 {
        return 0.0;
    }

    if offset <= SATURATION_MAGNET {
        return 0.0;
    }

    if offset >= extent - SATURATION_MAGNET {
        return 1.0;
    }

    fraction(offset, extent)
}

/// How far along an extent an offset sits, as 0..=1.
///
/// A zero extent answers 0 rather than dividing by it. That is a
/// degenerate layout rather than a real one, and the alternative is a
/// NaN travelling all the way to a service call.
fn fraction(offset: f32, extent: f32) -> f32 {
    if extent <= 0.0 {
        return 0.0;
    }

    (offset / extent).clamp(0.0, 1.0)
}

/// How much of the pointer's movement Alt keeps.
///
/// A quarter, which is what the field needs to be precise enough at
/// Small: 132 points across 359 degrees is 2.72 degrees per point, and a
/// quarter of that is 0.68 - finer than the eye can tell two hues apart,
/// so the axis stops being the limit. A HiDPI display has already halved
/// the coarse figure for free, because the cursor arrives in logical
/// coordinates before anything is cast down to a step.
const FINE_SCALE: f32 = 0.25;

/// How near an end of the saturation axis a pointer has to be for the
/// axis to snap onto it, in logical points of the field.
///
/// Points and not a fraction of the height, deliberately. The field is
/// 66 points tall at Small and 106 at Large, so a fractional magnet
/// would be a full half again as deep on the large one - the same
/// gesture would ask for a different steadiness at each preset, for no
/// reason the hand can account for. Points make the magnet the same
/// physical size everywhere, which is where the steadiness actually
/// lives.
///
/// Three points is about the slop in a deliberate mouse movement, and it
/// is cheap: at Small it is 4.5 percent, so the axis gives up the four
/// steps inside each end and keeps the other ninety-two. Alt does not
/// win those eight back, because it scales the movement and not the
/// magnet - the wheel does, a step at a time, which is what the wheel is
/// for.
const SATURATION_MAGNET: f32 = 3.0;

/// Radius of the ring drawn where the colour sits.
const MARKER_RADIUS: f32 = 6.0;

/// Thickness of the ring's bright inner stroke.
const MARKER_STROKE: f32 = 2.0;

/// Corner radius of the field itself, following the rounded language of
/// the card it sits in without competing with it.
const FIELD_RADIUS: f32 = 8.0;

/// How a colour field is painted.
///
/// Passed in rather than read from a palette here so the widget stays a
/// widget: it knows about pointers and pixels and nothing about themes.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// What sits under the spectrum.
    ///
    /// The texture is opaque, so on a settled frame this shows only
    /// through the rounded corners. It matters on the first frame a
    /// field is drawn: a renderer is entitled to upload an image handle
    /// in the background and draw nothing until it is resident, and a
    /// hole the shape of the field flashing into the card is worse than
    /// a plate of the card's own colour doing so.
    pub fill: Color,
    /// The ring drawn at the current colour.
    pub marker: Color,
    /// A darker ring just outside the bright one, so the marker stays
    /// visible over a pale part of the field.
    pub marker_shadow: Color,
    /// How much of the field's presence to keep, as 0..=1.
    ///
    /// Below 1 the spectrum recedes towards the card behind it, which is
    /// how an absent colour reads: the same statement a slider makes by
    /// fading its rail, and made here by fading the surface, because the
    /// surface is this control's rail (#94).
    ///
    /// It is the caller's number rather than something inferred from
    /// `colour` being `None`, so that the field, the label above it and
    /// every slider in the same card dim by one shared constant instead
    /// of by two mechanisms that can drift apart.
    pub opacity: f32,
}

/// A two-dimensional colour surface: hue across, saturation down.
pub struct ColorField<'a, Message> {
    /// The colour to mark, or `None` when Home Assistant is reporting
    /// none - the light is off, or sitting in a white mode. The field is
    /// still drawn and still operable; only the mark that would claim a
    /// colour goes away, exactly as a null axis renders without a knob
    /// (`docs/adr/0006-a-null-axis-renders-as-absent.md`).
    colour: Option<(f32, f32)>,
    height: f32,
    style: Style,
    on_change: Box<dyn Fn(f32, f32) -> Message + 'a>,
    on_release: Message,
}

/// What the widget has to remember between events.
///
/// `pressed_at` is both the "is a drag in progress" flag and the origin
/// #07 constrains against, which is why it is a point and not a bool.
///
/// `modifiers` is here because `Widget::update` is not given them: they
/// arrive as their own keyboard event, and a widget that wants to know
/// whether shift is down while the mouse moves has to have been
/// listening.
///
/// What is deliberately *not* here is any record of what a modifier did
/// last frame - no locked axis, no origin the lock was taken against
/// beyond the press itself. [`colour_at`] is handed the modifiers of the
/// frame it is answering, so a lock cannot outlive the key that asked
/// for it, and there is no state to be found in the wrong position by a
/// user who released Shift while the pointer was still moving.
///
/// `scrolled` is the one exception and is not a decision: it is the
/// remainder of a detent that a precise scrolling device has not yet
/// finished paying for.
#[derive(Debug, Default)]
struct State {
    pressed_at: Option<Point>,
    modifiers: Modifiers,
    scrolled: f32,
}

/// A colour field bound to `colour`, `height` tall and as wide as it is
/// given.
///
/// The width is filled rather than fixed so the field is exactly as wide
/// as the slider tracks above it, whatever the card's padding is;
/// `WidgetSize::colour_field_size` states the pair the window grows by.
pub fn colour_field<'a, Message>(
    colour: Option<(f32, f32)>,
    height: f32,
    style: Style,
    on_change: impl Fn(f32, f32) -> Message + 'a,
    on_release: Message,
) -> ColorField<'a, Message> {
    ColorField {
        colour,
        height,
        style,
        on_change: Box::new(on_change),
        on_release,
    }
}

/// The renderer a colour field needs: one that can draw a raster image,
/// which is a strictly stronger requirement than the plain
/// `renderer::Renderer` the marker and the fill would have been happy
/// with. Every backend iced ships satisfies it once the crate asks for
/// the image feature, `tiny-skia` included, which is the whole reason
/// the field is a texture and not a shader.
impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for ColorField<'_, Message>
where
    Message: Clone,
    Renderer: image::Renderer<Handle = image::Handle>,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fixed(self.height),
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fixed(self.height))
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        // Publishing only on a change keeps a stationary pointer from
        // restating the same colour every frame. The widget's own idea of
        // the colour moves with it, because the application's answer
        // comes back a frame later and would otherwise re-arm every one
        // of those messages.
        //
        // The verdict comes back for the wheel's sake: a nudge that ran
        // into the end of an axis has nothing to release.
        let mut change = |colour: &mut Option<(f32, f32)>, next: (f32, f32)| {
            if *colour == Some(next) {
                return false;
            }

            *colour = Some(next);
            shell.publish((self.on_change)(next.0, next.1));
            true
        };

        // A touch is read through `cursor` rather than through the
        // position the touch event carries, which looks like it would
        // leave a touchscreen with no mouse unable to press the field at
        // all. It does not: iced's winit shell sets its cursor position
        // from `WindowEvent::Touch` as well as from `CursorMoved`, so on
        // a finger event the cursor *is* the finger, and it is already
        // in logical coordinates while `touch::Event` is not.
        //
        // What this does give up is multi-touch. The cursor is wherever
        // the most recent finger went, so a second finger on the field
        // moves the same gesture rather than starting its own. Snapdash
        // has one colour surface open at a time and one marker to put
        // somewhere, so there is nothing a second finger could mean.
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | iced::Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(position) = cursor.position_over(bounds) {
                    state.pressed_at = Some(position);
                    change(
                        &mut self.colour,
                        colour_at(bounds, position, state.modifiers, position),
                    );

                    // Captured so the press does not also reach the card
                    // behind, which would start dragging the window
                    // (`docs/adr/0001-widget-interaction-model.md`).
                    shell.capture_event();
                }
            }

            iced::Event::Mouse(mouse::Event::CursorMoved { .. })
            | iced::Event::Touch(touch::Event::FingerMoved { .. }) => {
                // `land()` because a pointer that has left the widget's
                // layer still belongs to this drag: the gesture is owned
                // by the press, not by what happens to be under the
                // cursor now.
                if let (Some(pressed_at), Some(position)) =
                    (state.pressed_at, cursor.land().position())
                {
                    change(
                        &mut self.colour,
                        colour_at(bounds, position, state.modifiers, pressed_at),
                    );
                    shell.capture_event();
                }
            }

            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | iced::Event::Touch(touch::Event::FingerLifted { .. })
            | iced::Event::Touch(touch::Event::FingerLost { .. }) => {
                if state.pressed_at.take().is_some() {
                    shell.publish(self.on_release.clone());
                }
            }

            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return;
                }

                // A detent at a time, with the remainder kept: a precise
                // device reports the distance the fingers travelled
                // rather than detents, and emits at the refresh rate, so
                // spending every event as a step would make one flick
                // over the field a hundred service calls.
                state.scrolled += notches(*delta);
                let steps = state.scrolled.trunc();
                state.scrolled -= steps;

                // A field with no colour has nothing to nudge. That is
                // not the same as being inert: a press names a value
                // outright and is how an absent axis is given one
                // (`docs/adr/0006-a-null-axis-renders-as-absent.md`),
                // whereas a wheel can only offer a number to add to one
                // that is not there.
                //
                // The release goes out with the change because a nudge is
                // a whole interaction and not the middle of one.
                // `app::pending` throttles what a gesture puts on the
                // wire and flushes it on release, so a change with no
                // release would leave the last nudge to be swallowed by
                // the throttle and the pending value with no settle
                // deadline - a widget showing a colour the house never
                // received, with nothing left to correct it.
                if let Some(colour) = self.colour
                    && steps != 0.0
                    && change(&mut self.colour, nudged(colour, steps, state.modifiers))
                {
                    shell.publish(self.on_release.clone());
                }

                // Captured whether or not this event turned into a step:
                // the field owns the wheel over itself, and a scroll it
                // has only half consumed still must not also reach
                // whatever is behind it.
                shell.capture_event();
            }

            iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }

            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        if state.pressed_at.is_some() || cursor.is_over(layout.bounds()) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: border::rounded(FIELD_RADIUS),
                ..renderer::Quad::default()
            },
            self.style.fill,
        );

        // The spectrum, drawn into exactly the bounds the marker is
        // placed in. `WidgetSize::colour_field_size` is a slider track
        // wide and half as tall at every preset, and the texture is
        // computed at 2:1, so this is a uniform scale down in both axes
        // and a column of the texture stays a column on screen.
        //
        // `clip_bounds` is the same rectangle, because the border radius
        // is applied to the clip rather than to the image: the corners
        // are rounded by clipping the surface, not by rounding it.
        //
        // Only `wgpu` honours that radius. `iced_tiny_skia` carries the
        // field through its layer and then never reads it, clipping the
        // raster to a plain rectangle, so on the software fallback the
        // spectrum has square corners sitting on a rounded plate. It is
        // set anyway, because it is right where it is read and costs
        // nothing where it is not, and the divergence is four corners of
        // a cosmetic radius rather than anything the widget claims about
        // the light. Opacity, which is the part that carries meaning, is
        // honoured by both.
        //
        // Not snapped to the pixel grid. Snapping would move the surface
        // by up to half a physical pixel relative to the marker, whose
        // position comes from these unsnapped bounds, and the marker
        // agreeing with the colour under it is the entire job here.
        // Nothing is lost by leaving it off, because a smooth gradient
        // has no hard edge for the pixel grid to shimmer against.
        renderer.draw_image(
            image::Image {
                handle: colour_texture::handle(),
                filter_method: image::FilterMethod::Linear,
                rotation: Radians(0.0),
                border_radius: FIELD_RADIUS.into(),
                opacity: self.style.opacity,
                snap: false,
            },
            bounds,
            bounds,
        );

        let Some((hue, saturation)) = self.colour else {
            return;
        };

        let centre = position_of(bounds, hue, saturation);

        // Clipped to the field rather than inset into it, so the ring is
        // centred on the colour it names right up to the edges. Inset, a
        // marker at saturation 0 would sit a whole radius below the top
        // row and name a colour the field is not showing there - and it
        // would draw over the label above, which is outside these bounds.
        renderer.with_layer(bounds, |renderer| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: square_at(centre, MARKER_RADIUS + 1.0),
                    border: border::rounded(MARKER_RADIUS + 1.0)
                        .color(self.style.marker_shadow)
                        .width(1.0),
                    ..renderer::Quad::default()
                },
                Color::TRANSPARENT,
            );

            renderer.fill_quad(
                renderer::Quad {
                    bounds: square_at(centre, MARKER_RADIUS),
                    border: border::rounded(MARKER_RADIUS)
                        .color(self.style.marker)
                        .width(MARKER_STROKE),
                    ..renderer::Quad::default()
                },
                Color::TRANSPARENT,
            );
        });
    }
}

/// The box a circle of `radius` centred on `centre` occupies.
fn square_at(centre: Point, radius: f32) -> Rectangle {
    Rectangle::new(
        Point::new(centre.x - radius, centre.y - radius),
        Size::new(radius * 2.0, radius * 2.0),
    )
}

impl<'a, Message, Theme, Renderer> From<ColorField<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: image::Renderer<Handle = image::Handle> + 'a,
{
    fn from(field: ColorField<'a, Message>) -> Self {
        Self::new(field)
    }
}

#[cfg(test)]
mod tests;
