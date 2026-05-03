use iced::widget::text::IntoFragment;
use iced::widget::{column, container, text};
use iced::{Background, Border, Element, Length};

use crate::app::Message;
use crate::theme::{Palette, text_size};
use crate::ui::icon::Icon;
use crate::ui::theme::MessageType;

pub fn title<'a>(label: impl IntoFragment<'a>, p: Palette) -> text::Text<'a> {
    text(label).color(p.text_primary).size(text_size::XLARGE)
}

pub fn section<'a>(label: impl IntoFragment<'a>, p: Palette) -> text::Text<'a> {
    text(label).color(p.text_secondary).size(text_size::LARGE)
}

pub fn label<'a>(label: impl IntoFragment<'a>, p: Palette) -> text::Text<'a> {
    text(label).color(p.text_secondary).size(text_size::LARGE)
}

pub fn badge<'a>(label: impl IntoFragment<'a>, p: Palette) -> Element<'a, Message> {
    container(
        text(label)
            .size(text_size::NORMAL)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.accent),
            }),
    )
    .padding([3, 10])
    .style(move |_| container::Style {
        background: Some(Background::Color(p.accent_tint)),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: p.accent,
        },
        ..Default::default()
    })
    .into()
}

pub fn badge_with_icon<'a>(
    label: impl IntoFragment<'a>,
    icon: Icon,
    p: Palette,
) -> Element<'a, Message> {
    let label_text = text(label)
        .size(text_size::NORMAL)
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(p.accent),
        });

    let inner: Element<'a, Message> =
        iced::widget::row![icon.text(p).color(p.accent).size(11), label_text]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .into();

    container(inner)
        .padding([3, 10])
        .style(move |_| container::Style {
            background: Some(Background::Color(p.accent_tint)),
            border: Border {
                radius: 999.0.into(),
                width: 1.0,
                color: p.accent,
            },
            ..Default::default()
        })
        .into()
}

/// Standard body text, used for setting row labels and descriptions
/// that need to be readable at arm's length.
pub fn body<'a>(label: impl IntoFragment<'a>, p: Palette) -> text::Text<'a> {
    text(label).color(p.text_body).size(text_size::NORMAL)
}

pub fn dimmed<'a>(label: impl IntoFragment<'a>, p: Palette) -> text::Text<'a> {
    text(label).color(p.text_dim).size(text_size::SMALL)
}

pub fn helper<'a>(label: impl IntoFragment<'a>, p: Palette) -> text::Text<'a> {
    text(label).color(p.text_dim).size(text_size::SMALL)
}

/// Returns `column![...]` as Element<Message>
pub fn body_with_helper<'a>(
    label: impl IntoFragment<'a>,
    helper_text: impl IntoFragment<'a>,
    p: Palette,
) -> Element<'a, Message> {
    column![body(label, p), helper(helper_text, p)]
        .width(Length::Fill)
        .spacing(2)
        .into()
}
/// alpha is tuple: backgroud alpha, border alpha
pub fn message<'a>(
    content: impl IntoFragment<'a>,
    msg_type: MessageType,
    alpha: (f32, f32),
    p: Palette,
) -> Element<'a, Message> {
    let color = match msg_type {
        MessageType::Error => p.danger,
        MessageType::Warning => p.accent,
        MessageType::Success => p.success,
    };

    let (bg_a, b_a) = alpha;

    iced::widget::container(
        iced::widget::text(content)
            .size(text_size::SMALL)
            .style(move |_: &iced::Theme| iced::widget::text::Style { color: Some(color) }),
    )
    .padding([8, 12])
    .style(move |_| iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color { a: bg_a, ..color })),
        border: iced::Border {
            radius: 6.0.into(),
            width: 1.0,
            color: iced::Color { a: b_a, ..color },
        },
        ..Default::default()
    })
    .width(iced::Length::Fill)
    .into()
}

pub fn tooltip_message<'a>(
    content: impl IntoFragment<'a>,
    msg_type: MessageType,
    p: Palette,
) -> Element<'a, Message> {
    let color = match msg_type {
        MessageType::Error => p.danger,
        MessageType::Warning => p.accent,
        MessageType::Success => p.success,
    };

    iced::widget::container(
        iced::widget::text(content)
            .size(text_size::SMALL)
            .style(move |_: &iced::Theme| iced::widget::text::Style { color: Some(color) }),
    )
    .padding([8, 12])
    .style(move |_| iced::widget::container::Style {
        background: Some(iced::Background::Color(p.card_2)),
        border: iced::Border {
            radius: 6.0.into(),
            width: 1.0,
            color: iced::Color { a: 0.4, ..color },
        },
        ..Default::default()
    })
    .width(iced::Length::Fill)
    .into()
}

pub fn error_message<'a>(content: impl IntoFragment<'a>, p: Palette) -> Element<'a, Message> {
    message(content, MessageType::Error, (0.12, 0.4), p)
}

pub fn success_message<'a>(content: impl IntoFragment<'a>, p: Palette) -> Element<'a, Message> {
    message(content, MessageType::Success, (0.12, 0.4), p)
}
