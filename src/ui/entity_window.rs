use iced::widget::{column, row, space, text};
use iced::{Alignment, Element, Length};

use super::components;
use crate::app::{EntityWindowState, Message};
use crate::theme::Palette;
use crate::ui::format::format_entity_value;

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

    let label = if connected {
        "connected"
    } else {
        "disconected"
    };

    row![
        dot,
        text(label)
            .size(12)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(if connected { p.text_dim } else { p.danger })
            }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
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
) -> Element<'_, Message> {
    let (friendly, main_opt, detail) = format_main_value(state);

    let update_icon = if update {
        components::dimmed(
            '⟳',
            crate::theme::Palette {
                text_dim: iced::Color::from_rgb8(255, 0, 0),
                ..p
            },
        )
    } else {
        components::dimmed("", p)
    };
    let title_text = if let Some(name) = friendly {
        row![
            column![text(name).size(14).style(move |_: &iced::Theme| {
                iced::widget::text::Style {
                    color: Some(p.text_secondary),
                }
            }),]
            .width(iced::Fill),
            update_icon
        ]
    } else {
        row![
            text(pretty_name(&state.entity_id))
                .size(14)
                .style(move |_: &iced::Theme| iced::widget::text::Style {
                    color: Some(p.text_secondary),
                }),
            update_icon
        ]
    };

    let main = main_opt.unwrap_or_else(|| "-".into());
    let value_text = text(main)
        .size(44)
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(p.text_primary),
        });

    let detail_line: Element<'static, Message> = if let Some(d) = detail {
        text(d)
            .size(12)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.text_dim),
            })
            .into()
    } else {
        space().height(0).width(0).into()
    };

    let ring = pulse_border(p, state.pulse);

    let inner = column![
        title_text,
        space().height(6),
        value_text,
        space().height(10),
        detail_line,
        status_line(p, connected),
    ]
    .spacing(0)
    .width(Length::Fill);

    components::card_with_border(inner.into(), p, ring)
}
