use iced::widget::{column, container, row, stack, text};
use iced::{Background, Border, Element, Length};

use crate::app::Message;
use crate::theme::{Palette, metric, text_size};

pub fn card(content: Element<Message>, p: Palette) -> Element<Message> {
    container(content)
        .padding(metric::PAD)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.card)),
            border: Border {
                radius: {
                    // On Windows the OS (DWM) renders rounded corners on
                    // the window outer edge. Our container fills the window,
                    // so a non-zero inner radius would create a visible
                    // double-round artifact (8px DWM + N px inner).
                    // Use 0 inner radius and let DWM handle rounding.
                    #[cfg(target_os = "windows")]
                    {
                        iced::border::Radius::from(0.0)
                    }

                    #[cfg(not(target_os = "windows"))]
                    {
                        metric::RADIUS.into()
                    }
                },
                width: {
                    #[cfg(target_os = "windows")]
                    {
                        0.0
                    } // DWM clip handles the visible edge

                    #[cfg(not(target_os = "windows"))]
                    {
                        1.0
                    }
                },
                color: p.border,
            },
            shadow: {
                #[cfg(target_os = "windows")]
                {
                    iced::Shadow::default()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    p.shadow
                }
            },
            ..Default::default()
        })
        .into()
}

pub fn subcard(content: Element<Message>, p: Palette) -> Element<Message> {
    container(content)
        .padding(12)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.card_2)),
            border: Border {
                radius: (metric::RADIUS - 4.0).into(),
                width: 1.0,
                color: p.border,
            },
            ..Default::default()
        })
        .into()
}

pub fn card_with_border(
    content: Element<Message>,
    p: Palette,
    border_color: iced::Color,
) -> Element<Message> {
    let border_width = if border_color.a <= 0.1 { 0.0 } else { 1.0 };
    #[cfg(target_os = "windows")]
    {
        // Windows: DWM rounds the outer window; inner radius
        // would create double-round artifact. Border line is
        // still drawn (animated alpha) and gets pixel-clipped
        // by DWM at the corners — so visually we still get
        // rounded edges on the border highlight, just from
        // the OS rather than from our shader.

        let base: Element<Message> = container(content)
            .padding(metric::PAD)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(p.card)),
                border: Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: p.border,
                },
                shadow: iced::Shadow::default(),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let ring: Element<Message> = container(
            container(iced::widget::space())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_theme| container::Style {
                    background: None,
                    border: Border {
                        radius: 8.0.into(),
                        width: border_width,
                        color: border_color,
                    },
                    shadow: iced::Shadow::default(),
                    snap: true,
                    ..Default::default()
                }),
        )
        .padding(1.0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        return stack![base, ring]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
    #[cfg(not(target_os = "windows"))]
    container(content)
        .padding(metric::PAD)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(p.card)),
            border: Border {
                radius: { metric::RADIUS.into() },
                width: border_width,
                color: border_color,
            },
            shadow: { p.shadow },
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn fieldset<'a>(
    legend: &'a str,
    content: Element<'a, Message>,
    p: Palette,
) -> Element<'a, Message> {
    let legend_chip: Element<'a, Message> = container(text(legend).size(text_size::SMALL).style(
        move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(p.text_dim),
        },
    ))
    .padding([0, 8])
    .style(move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(p.card_2.into()),
        ..Default::default()
    })
    .into();

    let panel: Element<'a, Message> = container(content)
        .width(Length::Fill)
        .padding(
            iced::Padding::default()
                .top(12)
                .right(12)
                .bottom(12)
                .left(12),
        )
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: None,
            border: Border {
                radius: 14.0.into(),
                width: 1.0,
                color: p.border,
            },
            ..Default::default()
        })
        .into();

    let base: Element<'a, Message> = column![iced::widget::space().height(7), panel,]
        .spacing(0)
        .width(Length::Fill)
        .into();

    let legend_overlay: Element<'a, Message> =
        container(row![iced::widget::space().width(14), legend_chip,])
            .width(Length::Fill)
            .align_left(Length::Fill)
            .align_top(Length::Shrink)
            .into();

    stack![base, legend_overlay].width(Length::Fill).into()
}
