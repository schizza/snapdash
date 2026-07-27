use iced::widget::{column, mouse_area, row, space, text};
use iced::{Alignment, Element, Length};

use super::components;
use crate::app::{EntityWindowState, Message};
use crate::ha::ActionKind;
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
fn confirm_prompt<'a>(entity_id: &str, font: f32, p: Palette) -> Element<'a, Message> {
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
            .spacing(metric::GAP)
            .align_x(Alignment::Center),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
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

pub fn view(
    state: &EntityWindowState,
    p: Palette,
    connected: bool,
    update: bool,
    widget_settings: crate::config::WidgetSettings,
    priority: Priority,
    title: String,
) -> Element<'_, Message> {
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
        .then(|| ActionKind::from_entity_id(&state.entity_id))
        .flatten();
    let action_button: Option<Element<Message>> = action_kind.map(|action| {
        let (icon, tooltip) = match action {
            ActionKind::ToggleSwitch => (Icon::Toggle, "Toggle switch"),
            ActionKind::ToggleLight => (Icon::Toggle, "Toggle light"),
            ActionKind::TriggerScene => (Icon::Play, "Activate scene"),
            ActionKind::TriggerScript => (Icon::Play, "Run script"),
            ActionKind::ToggleInputBoolean => (Icon::Toggle, "Toggle input"),
        };
        components::icon_button(
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
        )
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
        let body: Element<Message> = if state.armed_at.is_some() {
            confirm_prompt(
                &state.entity_id,
                widget_settings.widget_size.title_font(),
                p,
            )
        } else {
            iced::widget::container(value_text)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };

        inner_column = inner_column.push(body);
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
            .height(Length::Fill)
            .align_y(Alignment::End);

        //        inner_column = inner_column.push(detail_line);
        inner_column = inner_column.push(status_line);

        inner_column
    };

    components::card_with_border(inner.into(), p, ring)
}
