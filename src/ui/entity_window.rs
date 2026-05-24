use iced::widget::{column, mouse_area, row, space, text};
use iced::{Alignment, Element, Length};

use super::components;
use crate::app::{EntityWindowState, Message};
use crate::theme::{Palette, metric};
use crate::ui::format::format_entity_value;
use crate::widget_size::{Priority, WidgetSize};

fn pretty_name(entity_id: &str) -> &str {
    entity_id.split('.').nth(1).unwrap_or(entity_id)
}

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
) -> Element<'_, Message> {
    let (friendly, main_opt, detail) = format_main_value(state);

    let update_button = components::icon_button(
        crate::ui::icon::Icon::Download,
        components::tooltip_message("Update available", crate::ui::theme::MessageType::Error, p),
        Some(p.danger),
        Some(widget_settings.widget_size.title_font()),
        Message::OpenSettingsTo(crate::ui::settings::SettingsPage::Updates),
        p,
    );

    let update_icon: Element<Message> = mouse_area(update_button)
        .on_press(Message::OpenReleaseNotes)
        .interaction(iced::mouse::Interaction::Pointer)
        .into();

    let mut title_text = if let Some(name) = friendly {
        row![
            column![
                text(name)
                    .size(widget_settings.widget_size.title_font())
                    .style(move |_: &iced::Theme| {
                        iced::widget::text::Style {
                            color: Some(p.text_secondary),
                        }
                    }),
            ]
            .width(iced::Fill),
        ]
    } else {
        row![
            text(pretty_name(&state.entity_id))
                .size(widget_settings.widget_size.title_font())
                .style(move |_: &iced::Theme| iced::widget::text::Style {
                    color: Some(p.text_secondary),
                }),
        ]
    };

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
        inner_column = inner_column.push(
            iced::widget::container(value_text)
                .height(Length::Fill)
                .width(Length::Fill),
        );
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
