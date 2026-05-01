use iced::widget::text::IntoFragment;
use iced::widget::{column, container, text};
use iced::{Background, Border, Element, Length};

use crate::app::Message;
use crate::theme::Palette;
use crate::ui::icon::Icon;
use crate::ui::theme::{MessageType, UiTheme};

pub fn title(label: &'static str, p: Palette) -> text::Text<'static> {
    text(label).color(p.text_primary).size(22)
}

pub fn section(label: &'static str, p: Palette) -> text::Text<'static> {
    text(label).color(p.text_secondary).size(16)
}

// Left here intentionaly, usage in future?
pub fn label<S: Into<String>>(label: S, p: Palette) -> text::Text<'static> {
    text(label.into()).color(p.text_secondary).size(14)
}

pub fn badge<'a>(label: impl Into<String>, p: Palette) -> Element<'a, Message> {
    container(
        text(label.into())
            .size(11)
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
    label: impl Into<String>,
    icon: Icon,
    ui_theme: UiTheme,
) -> Element<'a, Message> {
    let p = ui_theme.palette;

    let label_text =
        text(label.into())
            .size(11)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.accent),
            });

    let inner: Element<'a, Message> =
        iced::widget::row![icon.text(ui_theme).color(p.accent).size(11), label_text]
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

pub fn dimmed<S: Into<String>>(label: S, p: Palette) -> text::Text<'static> {
    text(label.into()).color(p.text_dim).size(10)
}

/// Standard body text, used for setting row labels and descriptions
/// that need to be readable at arm's length.
pub fn body<S: Into<String>>(label: S, p: Palette) -> text::Text<'static> {
    text(label.into()).color(p.text_body).size(13)
}

pub fn helper<S: Into<String>>(label: S, p: Palette) -> text::Text<'static> {
    text(label.into()).color(p.text_dim).size(11)
}

pub fn body_with_helper<'a, S: Into<String>>(
    label: S,
    helper_text: S,
    p: Palette,
) -> Element<'a, Message> {
    column![body(label, p), helper(helper_text, p)]
        .width(Length::Fill)
        .into()
}

pub fn message<'a>(
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
            .size(12)
            .style(move |_: &iced::Theme| iced::widget::text::Style { color: Some(color) }),
    )
    .padding([8, 12])
    .style(move |_| iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color { a: 0.12, ..color })),
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
    message(content, MessageType::Error, p)
}

pub fn success_message<'a>(content: impl IntoFragment<'a>, p: Palette) -> Element<'a, Message> {
    message(content, MessageType::Success, p)
}
