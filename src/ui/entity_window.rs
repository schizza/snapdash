use iced::widget::{column, mouse_area, row, space, text};
use iced::{Alignment, Element, Length};

use super::components;
use crate::app::{EntityWindowState, Message};
use crate::theme::Palette;
use crate::ui::format::format_entity_value;
use crate::widget_size::WidgetSize;

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

    row![dot].spacing(8).align_y(Alignment::End).into()
}

fn pulse_border(p: Palette, pulse: f32) -> iced::Color {
    let t = pulse.clamp(0.0, 1.0);

    let a = 0.10 + 0.55 * t;

    iced::Color {
        r: p.accent.r,
        g: p.accent.g,
        b: p.accent.b,
        a,
    }
}

pub fn view(
    state: &EntityWindowState,
    p: Palette,
    connected: bool,
    update: bool,
    widget_settings: crate::config::WidgetSettings,
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
    let value_text = text(maybe_adapted_value)
        .size(maybe_adaptet_font)
        .wrapping(iced::widget::text::Wrapping::None)
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(p.text_primary),
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

    let ring = pulse_border(p, state.pulse);

    let inner = column![
        title_text,
        space().height(widget_settings.widget_size.title_value_gap()),
        {
            if !connected {
                disconnected_text
            } else {
                value_text.into()
            }
        },
        space().height(widget_settings.widget_size.value_detail_gap()),
        {
            if widget_settings.widget_size == WidgetSize::Small
                || !widget_settings.show_measurement_info
            {
                space().height(0).width(0).into()
            } else {
                detail_line
            }
        },
        status_line(p, connected),
    ]
    .spacing(0)
    .width(Length::Fill)
    .height(Length::Fill);

    components::card_with_border(inner.into(), p, ring)
}
