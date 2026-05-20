use iced::Element;
use iced::widget::scrollable::{AutoScroll, Direction, Rail, Scrollbar, Scroller, Status, Style};

use crate::app::Message;
use crate::theme::Palette;

pub fn scrollable<'a>(
    content: Element<'a, Message>,
    p: Palette,
) -> iced::widget::Scrollable<'a, Message> {
    // Configure the vertical scrollbar with a gap so it sits in its
    // own gutter instead of overlaying the content text on the right.
    let scrollbar = Scrollbar::default()
        .width(8.0)
        .margin(2.0)
        .scroller_width(6.0)
        .spacing(8.0);

    iced::widget::scrollable(content)
        .direction(Direction::Vertical(scrollbar))
        .style(move |_theme, status| {
            let is_interacting = matches!(
                status,
                Status::Hovered {
                    is_horizontal_scrollbar_hovered: true,
                    ..
                } | Status::Hovered {
                    is_vertical_scrollbar_hovered: true,
                    ..
                } | Status::Dragged { .. }
            );

            let scroller_color = if is_interacting {
                p.accent_dim
            } else {
                p.border_hovered
            };

            let rail = Rail {
                background: Some(iced::Background::Color(p.accent_tint)),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                scroller: Scroller {
                    background: iced::Background::Color(scroller_color),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                },
            };

            Style {
                container: iced::widget::container::Style::default(),
                vertical_rail: rail,
                horizontal_rail: rail,
                gap: None,
                auto_scroll: AutoScroll {
                    background: iced::Background::Color(p.accent_tint),
                    border: iced::Border::default(),
                    icon: p.text_body,
                    shadow: iced::Shadow::default(),
                },
            }
        })
}
