use iced::widget::{column, mouse_area, row, space, text};
use iced::{Alignment, Element, Length};

use super::components;
use crate::app::{EntityWindowState, Message};
use crate::ha::{ActionKind, Axis, AxisKind, Control};
use crate::theme::{Palette, metric};
use crate::ui::colour_field;
use crate::ui::components::IconVisual;
use crate::ui::format::format_entity_value;
use crate::ui::icon::Icon;
use crate::widget_size::{Priority, WidgetSize};

fn format_main_value(
    state: &EntityWindowState,
) -> (Option<String>, Option<String>, Option<String>) {
    match &state.last {
        Some(s) => {
            let f = format_entity_value(s);
            (f.title, Some(f.main), f.detail)
        }
        None => (None, Some("-".into()), None),
    }
}

fn status_line(p: Palette, connected: bool) -> Element<'static, Message> {
    let dot = components::status_dot(p, connected);

    row![dot]
        .spacing(8)
        .align_y(Alignment::End)
        .height(Length::Fill)
        .into()
}

/// Card body for a widget awaiting confirmation (#85).
///
/// Replaces the value rather than floating above it, so the question
/// stays welded to the thing being acted on instead of drifting into a
/// detached dialog. At the Small preset there is room for a short prompt
/// and two icons and nothing else, which is why this is terse.
///
/// The affirmative is red because the gate only exists for actions the
/// user called risky; the way out is unemphasised so it does not compete.
///
/// `gap` is the card's own title gap rather than the page-level
/// `metric::GAP`: at 12px between the question and the buttons, the
/// prompt is taller than the body of a Small card, and a column that
/// overflows its limits does not spill in iced, it hands its last child
/// whatever is left. That was zero, so the two icons laid out inside a
/// zero-height box and their glyphs were clipped away entirely.
fn confirm_prompt<'a>(entity_id: &str, font: f32, gap: f32, p: Palette) -> Element<'a, Message> {
    let confirm = |confirmed: bool| Message::WidgetActionConfirmed {
        entity_id: entity_id.to_owned(),
        confirmed,
    };

    let buttons = row![
        components::icon_button(
            Icon::Play,
            components::tooltip_message("Confirm", crate::ui::theme::MessageType::Warning, p),
            Some(p.danger),
            Some(font),
            confirm(true),
            IconVisual::danger(p),
            p,
        ),
        components::icon_button(
            Icon::Close,
            components::tooltip_message("Cancel", crate::ui::theme::MessageType::Info, p),
            Some(p.text_secondary),
            Some(font),
            confirm(false),
            IconVisual::neutral(p),
            p,
        ),
    ]
    .spacing(metric::GAP)
    .align_y(Alignment::Center);

    let prompt =
        text("Confirm?")
            .size(font)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.text_secondary),
            });

    iced::widget::container(
        column![prompt, buttons]
            .spacing(gap)
            .align_x(Alignment::Center),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

/// Human-facing value for the current slider position.
///
/// Brightness is stored 0-255 on the wire but means nothing to a user in
/// those units, so it reads as a percentage. Position is already a
/// percentage. A setpoint keeps its own scale, and its step decides
/// whether a decimal is worth showing.
fn readout(control: &Axis, value: f32) -> String {
    match control.kind {
        AxisKind::Brightness => {
            let pct = (value / control.max * 100.0).round();
            format!("{pct:.0}%")
        }
        // Kelvin is the unit users actually see on a bulb's box, so it
        // is shown as-is rather than rescaled to a percentage.
        AxisKind::ColorTemp => format!("{:.0}K", value.round()),
        // Degrees around the wheel and percent of colour, which are the
        // units these values are in. Neither is read by a block of its
        // own: the colour surface states both at once, in
        // [`colour_block`], because a colour is one value with two
        // components rather than two values shown together.
        AxisKind::Hue => format!("{:.0}°", value.round()),
        AxisKind::Saturation => format!("{:.0}%", value.round()),
        AxisKind::Position => format!("{:.0}%", value.round()),
        AxisKind::Temperature => {
            if control.step < 1.0 {
                format!("{value:.1}°")
            } else {
                format!("{value:.0}°")
            }
        }
    }
}

fn axis_label(kind: AxisKind) -> &'static str {
    match kind {
        AxisKind::Brightness => "Brightness",
        AxisKind::ColorTemp => "White",
        // Neither of these heads a block of its own: they are the two
        // axes of the colour surface, which is labelled once, as
        // "Colour". The names are here because an axis is entitled to
        // one, not because anything currently draws them.
        AxisKind::Hue => "Hue",
        AxisKind::Saturation => "Saturation",
        AxisKind::Temperature => "Target",
        AxisKind::Position => "Position",
    }
}

/// One control inside an expanded widget.
///
/// The dispatch that turns a [`Control`] into its own layout. Each
/// variant draws itself; this is the only place that decides which.
fn control_block<'a>(
    entity_id: &str,
    view: &ControlView,
    size: WidgetSize,
    p: Palette,
) -> Element<'a, Message> {
    let font = size.detail_font();
    let help = shortcut_hint(&view.control);

    match &view.control {
        Control::Value(axis) => control_row(entity_id, axis, view.value(0), font, help, p),
        Control::Color { .. } => {
            colour_block(entity_id, view.value(0), view.value(1), help, size, p)
        }
    }
}

/// What a control's precision shortcuts are, or `None` for a control
/// that has none.
///
/// The one place that decides where the help affordance appears, rather
/// than each block deciding for itself (#100). Only the colour surface
/// implements the shortcuts: `iced::widget::slider` reads the pointer's
/// horizontal offset and nothing else, so the same icon over a
/// brightness rail would promise help that pressing Shift cannot give.
///
/// Answering with the words instead of a `bool` is what keeps the two
/// facts together. A control that grows shortcuts of its own says so
/// here and describes them in its own module, and the label row picks
/// them up without being touched.
pub fn shortcut_hint(control: &Control) -> Option<&'static str> {
    match control {
        Control::Value(_) => None,
        Control::Color { .. } => Some(colour_field::SHORTCUTS),
    }
}

/// How much of its normal presence a control keeps while nothing is
/// driving the axis it belongs to.
///
/// Faint enough to read as "not in play" without a second look, and
/// solid enough that the label stays legible: the control is still fully
/// operable, and grabbing it is precisely how the axis gets a value.
const ABSENT_OPACITY: f32 = 0.4;

/// The same colour, scaled towards transparent.
///
/// Scaling the existing alpha rather than replacing it keeps a palette's
/// own translucency intact, so a theme that already ships a soft rail
/// does not come back opaque.
fn fade(color: iced::Color, opacity: f32) -> iced::Color {
    iced::Color {
        a: color.a * opacity,
        ..color
    }
}

fn fade_background(background: iced::Background, opacity: f32) -> iced::Background {
    match background {
        iced::Background::Color(color) => fade(color, opacity).into(),
        other => other,
    }
}

/// One axis inside an expanded widget: a label with its readout, and the
/// slider beneath.
///
/// `value` is what [`ControlView::value`] resolved for this axis, and
/// `None` there means Home Assistant is reporting the axis as null. The
/// row then renders as *absent* rather than as sitting at its minimum:
/// the whole block dims and the knob is not drawn at all. A rail with no
/// knob says "this axis has no value right now" in a way that cannot be
/// misread as "the value is at the minimum", which is exactly what a
/// knob parked hard left over a "0%" readout does say (#94).
///
/// Nothing about the row's *behaviour* changes. Grabbing it sends the
/// same service call, and for a light that is off `light.turn_on` is
/// also what turns the light on.
fn control_row<'a>(
    entity_id: &str,
    control: &Axis,
    value: Option<f32>,
    font: f32,
    help: Option<&'static str>,
    p: Palette,
) -> Element<'a, Message> {
    let absent = value.is_none();
    let opacity = if absent { ABSENT_OPACITY } else { 1.0 };

    // The slider still needs a position in range to lay itself out. It
    // is the minimum, but with the knob hidden nothing renders there.
    let position = value.unwrap_or(control.min);

    let header = control_header(
        axis_label(control.kind),
        value.map(|value| readout(control, value)),
        font,
        opacity,
        help,
        p,
    );

    let axis = control.kind;
    let bar = iced::widget::slider(control.min..=control.max, position, {
        let entity_id = entity_id.to_owned();
        move |value: f32| Message::ControlValueChanged {
            entity_id: entity_id.clone(),
            axis,
            value,
        }
    })
    .step(control.step)
    .on_release(Message::ControlReleased {
        entity_id: entity_id.to_owned(),
        axis,
    })
    .style(move |theme: &iced::Theme, status| {
        let mut style = iced::widget::slider::default(theme, status);

        if absent {
            style.rail.backgrounds = (
                fade_background(style.rail.backgrounds.0, ABSENT_OPACITY),
                fade_background(style.rail.backgrounds.1, ABSENT_OPACITY),
            );
            // A transparent handle is how the knob is removed: the
            // slider keeps its geometry and its hit area, so the row
            // stays draggable, and only the mark that would claim a
            // value goes away.
            style.handle.background = iced::Color::TRANSPARENT.into();
            style.handle.border_color = iced::Color::TRANSPARENT;
        }

        style
    });

    column![header, bar].spacing(4).width(Length::Fill).into()
}

/// The label-and-readout line every control is headed by.
///
/// Shared so a colour surface and a slider line their labels up, and so
/// "the value, or a dash when there is none" is stated once. The dash is
/// what an absent axis reads as: a readout is a statement about the
/// device, and an axis the device is not driving has nothing to state.
///
/// `help` is the control's precision shortcuts, from [`shortcut_hint`],
/// and where there are any the line ends with an icon that names them on
/// hover (#100).
fn control_header<'a>(
    label: &'static str,
    value: Option<String>,
    font: f32,
    opacity: f32,
    help: Option<&'static str>,
    p: Palette,
) -> Element<'a, Message> {
    let label_color = fade(p.text_dim, opacity);
    let value_color = fade(p.text_secondary, opacity);

    let label = text(label)
        .size(font)
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(label_color),
        });

    let value = text(value.unwrap_or_else(|| "-".to_owned()))
        .size(font)
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(value_color),
        });

    let mut line = row![label, space().width(Length::Fill), value].align_y(Alignment::Center);

    if let Some(hint) = help {
        line = line
            .push(space().width(HELP_GAP))
            .push(help_icon(hint, font, opacity, p));
    }

    line.into()
}

/// How far the help icon stands off the readout.
///
/// Enough that "85%" and the circle do not read as one glyph, and no
/// more: at Small the line is 132 points wide and the label, the widest
/// readout and the icon already claim about 100 of them.
const HELP_GAP: f32 = 4.0;

/// The affordance that says the precision shortcuts exist.
///
/// Hover and nothing else. It fires no message, so a press over it falls
/// through to the card underneath and still drags the widget, which is
/// what keeps it from becoming a hole in the drag surface
/// (`docs/adr/0001-widget-interaction-model.md`).
///
/// That ADR also keeps hover chrome off an expanded card, because every
/// pixel the card grew by is a control and an overlay pinned to its
/// edges swallows the press underneath. This is not that: it sits inside
/// the control's own layout rather than over a track, it takes its space
/// from the line it is part of rather than from anything draggable, and
/// the panel it opens is only up while the pointer is on the icon - so
/// it is never between the user and a gesture.
///
/// Drawn at the label's own size. The glyph fills its em box where the
/// text only fills its cap height, so at equal nominal sizes the icon
/// already reads a little larger than the words beside it, and asking
/// for more would make the help louder than the readout it follows.
fn help_icon<'a>(hint: &'static str, font: f32, opacity: f32, p: Palette) -> Element<'a, Message> {
    iced::widget::tooltip(
        Icon::Help
            .text(p)
            .size(font)
            .color(fade(p.text_dim, opacity)),
        components::tooltip_message(hint, crate::ui::theme::MessageType::Info, p),
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}

/// The colour surface inside an expanded widget: a label with its
/// readout, and the two-dimensional field beneath (#97).
///
/// The readout names both axes, in the units they are in - degrees
/// around the wheel and percent of colour - because a colour is one
/// value with two components rather than two values shown together.
///
/// `hue` and `saturation` are what [`ControlView::value`] resolved, and
/// `None` in either means Home Assistant is reporting no colour: the
/// light is off, or sitting in a white mode. The block then renders as
/// *absent* - dimmed, with no marker at all - for the reason a slider
/// then renders without a knob. A marker parked in the top-left corner
/// over a "0°, 0%" readout would be claiming the bulb is showing white,
/// which is a claim about the house and a false one (#94).
///
/// Nothing about its behaviour changes. Dragging it sends the same
/// service call, and for a light that is off `light.turn_on` carrying
/// `hs_color` is also what turns the light on.
fn colour_block<'a>(
    entity_id: &str,
    hue: Option<f32>,
    saturation: Option<f32>,
    help: Option<&'static str>,
    size: WidgetSize,
    p: Palette,
) -> Element<'a, Message> {
    let colour = hue.zip(saturation);
    let opacity = if colour.is_none() {
        ABSENT_OPACITY
    } else {
        1.0
    };

    let readout = colour.map(|(hue, saturation)| format!("{hue:.0}°, {saturation:.0}%"));

    let field = colour_field::colour_field(
        colour,
        size.colour_field_size().height,
        colour_field::Style {
            // The plate the spectrum is painted onto, which shows only
            // through the rounded corners and on the frame before the
            // texture is resident. It fades with the field so an absent
            // colour recedes into the card rather than onto a plate.
            fill: fade(p.card_2, opacity),
            marker: fade(iced::Color::WHITE, opacity),
            marker_shadow: fade(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5), opacity),
            // The one number a slider's rail is faded by, applied to the
            // surface that stands where a slider's rail would. Colours
            // are faded by scaling their alpha and a texture cannot be,
            // so it travels as its own field and the widget hands it to
            // the renderer.
            opacity,
        },
        {
            let entity_id = entity_id.to_owned();
            move |hue: f32, saturation: f32| Message::ColorChanged {
                entity_id: entity_id.clone(),
                hue,
                saturation,
            }
        },
        Message::ColorReleased {
            entity_id: entity_id.to_owned(),
        },
    );

    column![
        control_header("Colour", readout, size.detail_font(), opacity, help, p),
        field,
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

/// Ring pulse color for the card border.
///
/// Normal/Low widgets rest faint and flash to accent on a state update
/// (dimmed → accent → dimmed). High-priority widgets rest on a steady
/// accent ring and *invert* the pulse — they dip to dimmed and return
/// (accent → dimmed → accent) — so the update feedback survives the
/// always-on emphasis ring.
fn pulse_border(p: Palette, pulse: f32, priority: Priority) -> iced::Color {
    let t = pulse.clamp(0.0, 1.0);

    let a = match priority {
        Priority::Low | Priority::Normal => 0.15 + 0.55 * t,
        Priority::High => 0.65 - 0.50 * t,
    };

    iced::Color { a, ..p.accent }
}

/// Everything the widget card needs to render itself.
///
/// Bundled rather than passed as a parameter list because the card grew
/// past what a positional signature can carry legibly once the expand
/// chevron and its controls arrived, and every future axis adds more.
pub struct WidgetView<'a> {
    pub state: &'a EntityWindowState,
    pub palette: Palette,
    pub connected: bool,
    pub update_available: bool,
    pub settings: crate::config::WidgetSettings,
    pub priority: Priority,
    pub title: String,
    /// Every control this entity offers, in display order. A non-empty
    /// list is what earns the widget its expand chevron.
    pub controls: Vec<ControlView>,
}

/// One control together with the locally-held value of each axis it
/// drives, in the same order as [`Control::axes`].
///
/// A pending value wins over whatever HA last reported, for as long as
/// the user is driving that axis (`crate::app::pending`).
pub struct ControlView {
    pub control: Control,
    pub pending: Vec<Option<f32>>,
}

impl ControlView {
    /// The pending value of the `n`th axis this control drives.
    fn pending(&self, index: usize) -> Option<f32> {
        self.pending.get(index).copied().flatten()
    }

    /// The value the `n`th axis renders at, or `None` when nothing is
    /// currently driving it.
    ///
    /// A pending value wins while the user is driving that axis, and
    /// Home Assistant takes over again once the interaction reconciles.
    ///
    /// `None` is a reading rather than a gap in the record. Home
    /// Assistant nulls an axis the device is not currently driving:
    /// every colour attribute of a light that is off, and
    /// `color_temp_kelvin` on its own whenever the light is in some
    /// other colour mode. Answering the axis minimum instead would turn
    /// "there is no brightness" into "the brightness is zero", which is
    /// a claim about the bulb, and a false one (#94).
    pub fn value(&self, index: usize) -> Option<f32> {
        let axis = self.control.axes().nth(index)?;

        self.pending(index)
            .or(axis.current)
            .map(|value| value.clamp(axis.min, axis.max))
    }
}

pub fn view(ctx: WidgetView<'_>) -> Element<'_, Message> {
    let WidgetView {
        state,
        palette: p,
        connected,
        update_available: update,
        settings: widget_settings,
        priority,
        title,
        controls,
    } = ctx;

    let (_friendly, main_opt, detail) = format_main_value(state);

    let update_button = components::icon_button(
        Icon::Download,
        components::tooltip_message("Update available", crate::ui::theme::MessageType::Error, p),
        Some(p.danger),
        Some(widget_settings.widget_size.title_font()),
        Message::OpenSettingsTo(crate::ui::settings::SettingsPage::Updates),
        IconVisual::danger(p),
        p,
    );

    let update_icon: Element<Message> = mouse_area(update_button)
        .on_press(Message::OpenReleaseNotes)
        .interaction(iced::mouse::Interaction::Pointer)
        .into();

    // Actionable widget affordance (issue #81). When the entity's domain
    // is one of the five supported ones (switch/light/scene/script/
    // input_boolean) AND we're currently connected to HA, render a small
    // accent-colored button in the header. Tap fires a REST
    // `call_service`, or arms the widget when its confirmation gate is on
    // (#85). Placed before the update icon so when both are present the
    // update alert stays rightmost (matches existing priority: system
    // health first).
    //
    // This is the *only* way to fire the action: the card body is the
    // drag handle, deliberately, per
    // `docs/adr/0001-widget-interaction-model.md`.
    //
    // Hiding the button while disconnected keeps the UI honest: no dead
    // affordance, and no way for a mid-reconnect tap to fire a REST call
    // that would fight the WS handshake still in progress. The handler
    // guards the same condition; this just makes the intent visible.
    let action_kind = connected
        .then(|| ActionKind::primary_for_entity(&state.entity_id))
        .flatten();
    let action_button: Option<Element<Message>> = action_kind.and_then(|action| {
        let (icon, tooltip) = match action {
            ActionKind::ToggleSwitch => (Icon::Toggle, "Toggle switch"),
            ActionKind::ToggleLight => (Icon::Toggle, "Toggle light"),
            ActionKind::TriggerScene => (Icon::Play, "Activate scene"),
            ActionKind::TriggerScript => (Icon::Play, "Run script"),
            ActionKind::ToggleInputBoolean => (Icon::Toggle, "Toggle input"),
            // Value-carrying actions are built from a control, never
            // from an entity id, so `primary_for_entity` cannot hand one
            // back here and there is no header affordance for them.
            ActionKind::SetBrightness(_)
            | ActionKind::SetColorTemp(_)
            | ActionKind::SetHs { .. }
            | ActionKind::SetTemperature(_)
            | ActionKind::SetPosition(_) => return None,
        };
        Some(components::icon_button(
            icon,
            components::tooltip_message(tooltip, crate::ui::theme::MessageType::Info, p),
            Some(p.accent),
            Some(widget_settings.widget_size.title_font()),
            Message::WidgetActionTriggered {
                entity_id: state.entity_id.clone(),
                action,
            },
            IconVisual::accent(p),
            p,
        ))
    });

    let title_widget = text(title)
        .size(widget_settings.widget_size.title_font())
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(p.text_secondary),
        });

    let mut title_text = row![column![title_widget].width(iced::Fill)];

    if let Some(button) = action_button {
        title_text = title_text.push(button);
    }

    // The expand chevron, next to the action icon (#87). Both stay
    // visible at rest: the action icon is the signal that this widget
    // does something, and the chevron the signal that it has a value
    // worth adjusting. Gated on `connected` for the same reason the
    // action is, a control that cannot reach HA would swallow drags.
    if !controls.is_empty() && connected {
        let (icon, tooltip) = if state.is_expanded() {
            (Icon::ChevronUp, "Hide controls")
        } else {
            (Icon::ChevronDown, "Adjust value")
        };

        title_text = title_text.push(components::icon_button(
            icon,
            components::tooltip_message(tooltip, crate::ui::theme::MessageType::Info, p),
            Some(p.accent),
            Some(widget_settings.widget_size.title_font()),
            Message::ToggleWidgetControls(state.entity_id.clone()),
            IconVisual::accent(p),
            p,
        ));
    }

    if update {
        title_text = title_text.push(update_icon)
    }

    let main = main_opt.unwrap_or_else(|| "-".into());
    let maybe_adapted_value = widget_settings.adaptive.adapted_value(&main);
    let maybe_adaptet_font = widget_settings.adaptive.font_size(
        widget_settings.widget_size.value_font(),
        maybe_adapted_value.chars().count(),
    );

    let value_color = match priority {
        Priority::High => p.accent,
        Priority::Normal => p.text_primary,
        Priority::Low => p.text_dim,
    };

    let value_text = text(maybe_adapted_value)
        .size(maybe_adaptet_font)
        .wrapping(iced::widget::text::Wrapping::None)
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(value_color),
        });

    let detail_line: Element<'static, Message> = if let Some(d) = detail {
        text(d)
            .size(widget_settings.widget_size.detail_font())
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.text_dim),
            })
            .into()
    } else {
        space().height(0).width(0).into()
    };

    let disconnected_text =
        components::error_message("You are disconected from Home Assistant!", p);

    let ring = pulse_border(p, state.pulse.value(), priority);

    let mut inner_column = column![]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    let inner = if !connected {
        inner_column = inner_column.push(title_text);
        inner_column =
            inner_column.push(space().height(widget_settings.widget_size.title_value_gap()));
        inner_column = inner_column.push(disconnected_text);

        inner_column
    } else {
        inner_column = inner_column.push(title_text);
        inner_column =
            inner_column.push(space().height(widget_settings.widget_size.title_value_gap()));
        // An armed widget shows the question instead of the value. It is
        // the one state where the card deliberately stops mirroring the
        // house, which is why being armed is on a clock (#85).
        let armed = state.armed_at.is_some();

        let body: Element<Message> = if armed {
            confirm_prompt(
                &state.entity_id,
                widget_settings.widget_size.title_font(),
                widget_settings.widget_size.title_value_gap(),
                p,
            )
        } else {
            iced::widget::container(value_text)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };

        inner_column = inner_column.push(body);

        // The status line is the other half of the card's vertical
        // budget, and while the question is up the prompt needs all of
        // it: a Small card has around 27px to give each of two `Fill`
        // children, and the question plus its buttons do not fit in
        // that. A widget that has stopped mirroring the house for five
        // seconds can stop showing its connection dot for the same five
        // seconds, and the modal card is cleaner for it (#85).
        if !armed {
            inner_column =
                inner_column.push(space().height(widget_settings.widget_size.value_detail_gap()));

            let detail_line = if widget_settings.widget_size == WidgetSize::Small
                || !widget_settings.show_measurement_info
            {
                space().height(0).width(0).into()
            } else {
                detail_line
            };

            let status_line = row![status_line(p, connected), detail_line]
                .spacing(metric::GAP)
                .height(Length::Fill)
                .align_y(Alignment::End);

            inner_column = inner_column.push(status_line);
        }

        // The controls sit below the status line, in the height the
        // window grew by. They are part of the widget rather than a
        // window of their own, so they cannot drift away from the value
        // they belong to (#87).
        if state.is_expanded() {
            for view in &controls {
                inner_column = inner_column
                    .push(space().height(widget_settings.widget_size.value_detail_gap()));
                inner_column = inner_column.push(control_block(
                    &state.entity_id,
                    view,
                    widget_settings.widget_size,
                    p,
                ));
            }
        }

        inner_column
    };

    components::card_with_border(inner.into(), p, ring)
}
