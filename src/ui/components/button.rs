use iced::{Background, Element, Length, widget::container};

use crate::{app::Message, theme::Palette};

#[derive(Clone, Copy)]
pub struct ButtonVisual {
    pub bg: iced::Color,
    pub bg_hovered: iced::Color,
    pub bg_pressed: iced::Color,
    pub bg_disabled: iced::Color,
    pub border: iced::Color,
    pub border_hovered: iced::Color,
    pub border_width: f32,
    pub text: iced::Color,
    pub text_disabled: iced::Color,
    pub radius: f32,
}

impl ButtonVisual {
    pub fn pill(p: Palette) -> Self {
        Self {
            bg: p.card_2,
            bg_hovered: p.card,
            bg_pressed: p.accent_tint,
            bg_disabled: p.card_2,
            border: p.border,
            border_hovered: p.border_hovered,
            border_width: 1.0,
            text: p.text_body,
            text_disabled: p.text_disabled,
            radius: 999.0,
        }
    }

    pub fn primary(p: Palette) -> Self {
        Self {
            bg: p.accent_tint,
            bg_hovered: p.accent,
            bg_pressed: p.accent,
            bg_disabled: p.accent_tint,
            border: iced::Color::TRANSPARENT,
            border_hovered: iced::Color::TRANSPARENT,
            border_width: 1.0,
            text: p.text_primary,
            text_disabled: p.text_disabled,
            radius: 999.0,
        }
    }

    pub fn bg(mut self, c: iced::Color) -> Self {
        self.bg = c;
        self
    }
    pub fn bg_hovered(mut self, c: iced::Color) -> Self {
        self.bg_hovered = c;
        self
    }
    pub fn bg_pressed(mut self, c: iced::Color) -> Self {
        self.bg_pressed = c;
        self
    }
    pub fn bg_disabled(mut self, c: iced::Color) -> Self {
        self.bg_disabled = c;
        self
    }
    pub fn text(mut self, c: iced::Color) -> Self {
        self.text = c;
        self
    }
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    pub fn border(mut self, c: iced::Color) -> Self {
        self.border = c;
        self
    }
}

fn styled_button<'a>(
    content: impl Into<Element<'a, Message>>,
    visual: ButtonVisual,
    padding: impl Into<iced::Padding>,
    height: Length,
) -> iced::widget::Button<'a, Message> {
    use iced::widget::button::{Status, Style};

    iced::widget::button(content)
        .padding(padding)
        .height(height)
        .style(move |_theme, status| {
            let (bg, border) = match status {
                Status::Hovered => (visual.bg_hovered, visual.border_hovered),
                Status::Pressed => (visual.bg_pressed, visual.border),
                Status::Disabled => (visual.bg_disabled, visual.border),
                Status::Active => (visual.bg, visual.border),
            };

            Style {
                background: Some(Background::Color(bg)),
                border: iced::Border {
                    radius: visual.radius.into(),
                    width: visual.border_width,
                    color: border,
                },
                text_color: if matches!(status, Status::Disabled) {
                    visual.text_disabled
                } else {
                    visual.text
                },
                ..Default::default()
            }
        })
}

pub fn primary_button<'a>(
    label: impl Into<Element<'a, Message>>,
    p: Palette,
    on_press: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    let content = container(label)
        .width(Length::Shrink)
        .height(Length::Fill)
        .center_x(Length::Shrink)
        .center_y(Length::Fill);

    let mut b = styled_button(
        content,
        ButtonVisual::primary(p),
        [0, 12],
        Length::Fixed(36.0),
    );

    if let Some(msg) = on_press {
        b = b.on_press(msg)
    }

    b
}

pub fn pill_button<'a>(
    label: impl Into<Element<'a, Message>>,
    p: Palette,
    on_press: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    let content = container(label)
        .width(Length::Shrink)
        .height(Length::Fill)
        .center_x(Length::Shrink)
        .center_y(Length::Fill);

    let mut b = styled_button(content, ButtonVisual::pill(p), [0, 12], Length::Fixed(36.0));

    if let Some(msg) = on_press {
        b = b.on_press(msg);
    }
    b
}

pub fn pill_button_with<'a>(
    label: impl Into<Element<'a, Message>>,
    visual: ButtonVisual,
    on_press: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    let content = container(label)
        .width(Length::Shrink)
        .height(Length::Fill)
        .center_x(Length::Shrink)
        .center_y(Length::Fill);

    let mut b = styled_button(content, visual, [0, 12], Length::Fixed(36.0));

    if let Some(msg) = on_press {
        b = b.on_press(msg);
    }
    b
}

// pub fn icon(
//     content: impl Into<Element<'static, Message>>,
//     p: Palette,
//     on_press: Option<Message>,
// ) -> iced::widget::Button<'static, Message> {
//     use iced::widget::button::Status;

//     let mut b = button(content)
//         .padding([0, 12])
//         .height(36)
//         .style(move |_theme, status| {
//             let mut bg = p.card_2;
//             let mut border = p.border;

//             match status {
//                 Status::Hovered => {
//                     bg = p.card;
//                     border = p.border_hovered
//                 }
//                 Status::Pressed => bg = p.accent_tint,
//                 Status::Disabled => {
//                     bg = p.card_2;
//                     border = p.border
//                 }
//                 Status::Active => {}
//             }

//             button::Style {
//                 background: Some(Background::Color(bg)),
//                 border: Border {
//                     radius: 999.0.into(),
//                     width: 1.0,
//                     color: border,
//                 },
//                 text_color: if matches!(status, Status::Disabled) {
//                     p.text_disabled
//                 } else {
//                     p.text_body
//                 },
//                 ..Default::default()
//             }
//         });

//     if let Some(msg) = on_press {
//         b = b.on_press(msg);
//     }

//     b
// }
