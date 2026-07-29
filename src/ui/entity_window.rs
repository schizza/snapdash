use iced::widget::{column, mouse_area, row, space, text};
use iced::{Alignment, Element, Length};

use super::components;
use crate::app::{EntityWindowState, Message};
use crate::ha::{ActionKind, Axis, AxisKind, Control};
use crate::theme::{Palette, metric};
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
    font: f32,
    p: Palette,
) -> Element<'a, Message> {
    match &view.control {
        Control::Value(axis) => control_row(entity_id, axis, view.pending(0), font, p),
    }
}

/// One axis inside an expanded widget: a label with its readout, and the
/// slider beneath.
///
/// The slider renders the *pending* value whenever the user is driving
/// it, falling back to what HA reported once the interaction reconciles.
/// Falling back to `min` keeps the slider in range for an entity that
/// reports no value at all, such as an unavailable light with no
/// brightness. See `crate::app::pending`.
fn control_row<'a>(
    entity_id: &str,
    control: &Axis,
    pending: Option<f32>,
    font: f32,
    p: Palette,
) -> Element<'a, Message> {
    let value = pending
        .or(control.current)
        .unwrap_or(control.min)
        .clamp(control.min, control.max);

    let label = text(axis_label(control.kind))
        .size(font)
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(p.text_dim),
        });

    let value_text = text(readout(control, value))
        .size(font)
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(p.text_secondary),
        });

    let axis = control.kind;
    let bar = iced::widget::slider(control.min..=control.max, value, {
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
    });

    column![
        row![label, space().width(Length::Fill), value_text].align_y(Alignment::Center),
        bar,
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
                    widget_settings.widget_size.detail_font(),
                    p,
                ));
            }
        }

        inner_column
    };

    components::card_with_border(inner.into(), p, ring)
}
