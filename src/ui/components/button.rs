use iced::widget::text::IntoFragment;
use iced::{Background, Element, Length, widget::container};
use iced::{Color, Padding};

use crate::ui::icon::Icon;
use crate::{
    app::Message,
    theme::{Palette, text_size},
};

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

#[derive(Clone, Copy)]
pub struct IconVisual {
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

pub enum ButtonType {
    PILL,
    PRIMARY,
    DANGER,
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
            bg_hovered: p.accent_tint,
            bg_pressed: p.accent_tint,
            bg_disabled: p.accent_tint,
            border: p.accent_tint,
            border_hovered: p.accent_dim,
            border_width: 1.0,
            text: p.text_primary,
            text_disabled: p.text_disabled,
            radius: 999.0,
        }
    }

    pub fn danger(p: Palette) -> Self {
        Self {
            bg: iced::Color {
                a: 0.16,
                ..p.danger
            },
            bg_hovered: iced::Color {
                a: 0.24,
                ..p.danger
            },
            bg_pressed: iced::Color {
                a: 0.32,
                ..p.danger
            },
            bg_disabled: p.card_2,
            border: p.danger,
            border_hovered: p.danger,
            border_width: 1.0,
            text: p.danger,
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

impl IconVisual {
    pub fn danger(p: Palette) -> Self {
        Self {
            bg: iced::Color {
                a: 0.16,
                ..p.danger
            },
            bg_hovered: iced::Color {
                a: 0.24,
                ..p.danger
            },
            bg_pressed: iced::Color {
                a: 0.32,
                ..p.danger
            },
            bg_disabled: p.card_2,
            border: p.danger,
            border_hovered: p.danger,
            border_width: 0.0,
            text: p.danger,
            text_disabled: p.text_disabled,
            radius: 999.0,
        }
    }
}

fn button_content<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Length::Shrink)
        .height(Length::Fill)
        .center_x(Length::Shrink)
        .center_y(Length::Fill)
        .into()
}

fn text_content<'a>(label: impl IntoFragment<'a>) -> Element<'a, Message> {
    iced::widget::text(label).size(text_size::NORMAL).into()
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

fn styled_icon_button<'a>(
    content: impl Into<Element<'a, Message>>,
    visual: IconVisual,
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
    label: impl IntoFragment<'a>,
    p: Palette,
    on_press: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    let mut b = styled_button(
        button_content(text_content(label)),
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
    label: impl IntoFragment<'a>,
    p: Palette,
    on_press: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    let mut b = styled_button(
        button_content(text_content(label)),
        ButtonVisual::pill(p),
        [0, 12],
        Length::Fixed(36.0),
    );

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
    let mut b = styled_button(button_content(label), visual, [0, 12], Length::Fixed(36.0));

    if let Some(msg) = on_press {
        b = b.on_press(msg);
    }
    b
}

pub fn danger_button_with<'a>(
    content: Element<'a, Message>,
    p: Palette,
    on_press: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    let mut b = styled_button(
        button_content(content),
        ButtonVisual::danger(p),
        [0, 12],
        Length::Fixed(36.0),
    );

    if let Some(m) = on_press {
        b = b.on_press(m);
    };
    b
}

pub fn danger_button<'a>(
    label: impl IntoFragment<'a>,
    p: Palette,
    on_press: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    let mut b = styled_button(
        button_content(text_content(label)),
        ButtonVisual::danger(p),
        [0, 12],
        Length::Fixed(36.0),
    );

    if let Some(msg) = on_press {
        b = b.on_press(msg);
    }
    b
}

pub fn icon_button<'a>(
    icon: Icon,
    hint: Element<'a, Message>,
    color: Option<Color>,
    text_size: Option<f32>,
    on_click: Message,

    p: Palette,
) -> Element<'a, Message> {
    let size = text_size.unwrap_or(text_size::SMALL);
    let mut i = icon.text(p).size(size);
    if let Some(c) = color {
        i = i.color(c);
    }

    iced::widget::tooltip(
        styled_icon_button(
            i,
            IconVisual::danger(p),
            Padding {
                top: 2.0,
                bottom: 2.0,
                left: 4.0,
                right: 4.0,
            },
            Length::Shrink,
        )
        .on_press(on_click)
        .width(Length::Shrink),
        hint,
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}
