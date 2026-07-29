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

use iced::advanced::image;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget, tree};
use iced::keyboard::{self, Modifiers};
use iced::{Color, Element, Length, Point, Radians, Rectangle, Size, border, mouse, touch};

use crate::ui::colour_texture::{self, MAX_HUE, MAX_SATURATION};

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
/// `modifiers` and `pressed_at` are not read yet. They are in the
/// signature because #07 constrains a drag to one axis while a modifier
/// is held, which needs both the modifier and the point the drag started
/// from, and because a pure function is where that can be tested. The
/// widget already tracks both, so #07 changes this function and nothing
/// around it.
pub fn colour_at(
    bounds: Rectangle,
    cursor: Point,
    _modifiers: Modifiers,
    _pressed_at: Point,
) -> (f32, f32) {
    let across = fraction(cursor.x - bounds.x, bounds.width);
    let down = fraction(cursor.y - bounds.y, bounds.height);

    ((across * MAX_HUE).round(), (down * MAX_SATURATION).round())
}

/// Where a colour sits in the field, which is [`colour_at`] read
/// backwards and how the marker finds its place.
fn position_of(bounds: Rectangle, hue: f32, saturation: f32) -> Point {
    Point::new(
        bounds.x + fraction(hue, MAX_HUE) * bounds.width,
        bounds.y + fraction(saturation, MAX_SATURATION) * bounds.height,
    )
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
#[derive(Debug, Default)]
struct State {
    pressed_at: Option<Point>,
    modifiers: Modifiers,
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
        let mut change = |colour: &mut Option<(f32, f32)>, next: (f32, f32)| {
            if *colour != Some(next) {
                *colour = Some(next);
                shell.publish((self.on_change)(next.0, next.1));
            }
        };

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
