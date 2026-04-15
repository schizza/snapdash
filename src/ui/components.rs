use iced::overlay::menu;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, stack, text, text_input,
};
use iced::{Background, Border, Element, Length};

use crate::app::Message;
use crate::theme::{Palette, ThemeKind, metric};

pub fn card(content: Element<Message>, p: Palette) -> Element<Message> {
    container(content)
        .padding(metric::PAD)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.card)),
            border: Border {
                radius: metric::RADIUS.into(),
                width: 1.0,
                color: p.border,
            },
            shadow: p.shadow,
            ..Default::default()
        })
        .into()
}

pub fn subcard(content: Element<Message>, p: Palette) -> Element<Message> {
    container(content)
        .padding(12)
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

/// Pill-shaped button (mac-ish), wrapper aroud `button`
/// `text-input` wrapper styled as a mac input
/// wrapper arroud `pick_list`
pub fn pill_button(
    label: &'static str,
    p: Palette,
    on_press: Option<Message>,
) -> iced::widget::Button<'static, Message> {
    use iced::widget::button::Status;

    let content = container(text(label))
        .width(Length::Shrink)
        .height(Length::Fill)
        .center_x(Length::Shrink)
        .center_y(Length::Fill);

    let mut b = button(content)
        .padding([0, 12])
        .height(36)
        .style(move |_theme, status| {
            let mut bg = p.card_2;
            let mut border = p.border;

            match status {
                Status::Hovered => {
                    bg = p.card;
                    border = p.border_hovered
                }
                Status::Pressed => bg = p.accent_tint,
                Status::Disabled => {
                    bg = p.card_2;
                    border = p.border
                }
                Status::Active => {}
            }

            button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    radius: 999.0.into(),
                    width: 1.0,
                    color: border,
                },
                text_color: if matches!(status, Status::Disabled) {
                    p.text_disabled
                } else {
                    p.text_body
                },
                ..Default::default()
            }
        });

    if let Some(msg) = on_press {
        b = b.on_press(msg);
    }

    b
}

pub fn icon(
    label: &'static str,
    p: Palette,
    on_press: Option<Message>,
) -> iced::widget::Button<'static, Message> {
    use iced::widget::button::Status;

    let content = container(text(label))
        .width(Length::Shrink)
        .height(Length::Fill)
        .center_x(Length::Shrink)
        .center_y(Length::Fill);

    let mut b = button(content)
        .padding([0, 12])
        .height(36)
        .style(move |_theme, status| {
            let mut bg = p.card_2;
            let mut border = p.border;

            match status {
                Status::Hovered => {
                    bg = p.card;
                    border = p.border_hovered
                }
                Status::Pressed => bg = p.accent_tint,
                Status::Disabled => {
                    bg = p.card_2;
                    border = p.border
                }
                Status::Active => {}
            }

            button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    radius: 999.0.into(),
                    width: 1.0,
                    color: border,
                },
                text_color: if matches!(status, Status::Disabled) {
                    p.text_disabled
                } else {
                    p.text_body
                },
                ..Default::default()
            }
        });

    if let Some(msg) = on_press {
        b = b.on_press(msg);
    }

    b
}

/// text_input wrapper to sytle as mac input
pub fn mac_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    p: Palette,
) -> text_input::TextInput<'a, Message> {
    use iced::widget::text_input::Status;

    text_input(placeholder, value)
        .padding(10)
        .style(move |_theme, status| {
            let mut border_color = p.border;
            let mut border_width = 1.0;

            match status {
                Status::Focused { is_hovered: _ } => {
                    border_color = p.accent;
                    border_width = 1.5;
                }
                Status::Hovered => border_color = p.border_hovered,
                Status::Disabled | Status::Active => {}
            }

            text_input::Style {
                background: Background::Color(p.card_2),
                border: Border {
                    radius: (metric::RADIUS - 6.0).into(),
                    width: border_width,
                    color: border_color,
                },
                icon: p.text_dim,
                placeholder: p.text_dim,
                value: p.text_body,
                selection: p.accent,
            }
        })
}

/// wrapper aroud pick_list
pub fn themepicker(
    options: Vec<ThemeKind>,
    selected: ThemeKind,
    p: Palette,
) -> pick_list::PickList<'static, ThemeKind, Vec<ThemeKind>, ThemeKind, Message> {
    use iced::widget::pick_list::Status;

    pick_list(options, Some(selected), Message::ThemeSelected)
        .padding([0, 12])
        .style(move |_theme, status| {
            let bg = p.card_2;
            let mut border = p.border;

            match status {
                Status::Hovered => {
                    border = p.border_hovered;
                }
                Status::Opened { is_hovered: _ } => {
                    border = p.accent;
                }

                Status::Active => {}
            }

            pick_list::Style {
                background: iced::Background::Color(bg),
                border: iced::Border {
                    radius: 999.0.into(),
                    width: 1.0,
                    color: border,
                },
                text_color: p.text_body,
                placeholder_color: p.text_dim,
                handle_color: p.text_dim,
            }
        })
        .menu_style(move |_theme| menu::Style {
            background: iced::Background::Color(p.card),
            border: iced::Border {
                radius: 12.0.into(),
                width: 1.0,
                color: p.border,
            },
            text_color: p.text_body,
            selected_text_color: p.text_body,
            selected_background: iced::Background::Color(p.accent_tint),
            shadow: p.shadow,
        })
}

pub fn title(label: &'static str, p: Palette) -> text::Text<'static> {
    text(label).color(p.text_primary).size(22)
}

pub fn section(label: &'static str, p: Palette) -> text::Text<'static> {
    text(label).color(p.text_secondary).size(16)
}

// Left here intentionaly, usage in future?
// pub fn label<S: Into<String>>(label: S, p: Palette) -> text::Text<'static> {
//     text(label.into()).color(p.text_secondary).size(14)
// }

pub fn dimmed<S: Into<String>>(label: S, p: Palette) -> text::Text<'static> {
    text(label.into()).color(p.text_dim).size(10)
}

// pub fn logbox<S: Into<String>>(label: S, p: Palette) -> TextInput<'static, Message> {
//     text_input("Empty", &label.into()).size(10).into()
// }

pub fn card_with_border(
    content: Element<Message>,
    p: Palette,
    border_color: iced::Color,
) -> Element<Message> {
    let border_width = if border_color.a <= 0.1 { 0.0 } else { 1.0 };

    container(content)
        .padding(metric::PAD)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(p.card)),
            border: Border {
                radius: metric::RADIUS.into(),
                width: border_width,
                color: border_color,
            },
            shadow: p.shadow,
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// pub fn card_plain(
//     content: Element<Message>,
//     p: Palette,
//     border_color: iced::Color,
// ) -> Element<Message> {
//     iced::widget::container(content)
//         .padding(16)
//         .style(move |_| iced::widget::container::Style {
//             background: Some(iced::Background::Color(p.card)), // bílá
//             border: iced::Border {
//                 radius: crate::theme::metric::RADIUS.into(),
//                 width: 0.0,
//                 color: iced::Color::TRANSPARENT,
//             },
//             shadow: p.shadow, // klidně nech / nebo dej menší
//             ..Default::default()
//         })
//         .into()
// }
//
pub fn status_dot(p: Palette, ok: bool) -> Element<'static, Message> {
    let color = if ok { p.success } else { p.danger };

    container(
        iced::widget::space()
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0)),
    )
    .style(move |_theme| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: p.border,
        },
        ..Default::default()
    })
    .into()
}

pub fn active_sensor_section(app: &crate::app::Snapdash, p: Palette) -> Element<'_, Message> {
    let active_sensors = app.active_settings_sensors.iter();

    let mut col = column![];
    for sensor in active_sensors {
        let id = sensor.entity_id.clone();
        let friendly_name = sensor.friendly_name.clone();
        let id_for_msg = id.clone();
        let row_el = row![
            checkbox(true)
                .label(friendly_name)
                .on_toggle(move |_| Message::ToggleWidget(id_for_msg.clone()))
                .text_size(14)
                .style(move |_, _| iced::widget::checkbox::Style {
                    background: iced::Background::Color(p.bg),
                    text_color: Some(p.text_primary),
                    icon_color: p.accent,
                    border: Border {
                        color: p.text_primary,
                        width: 0.5,
                        radius: 15.0.into()
                    }
                }),
            text(format!(" ({})", sensor.entity_id))
                .size(14)
                .style(move |_: &iced::Theme| iced::widget::text::Style {
                    color: Some(p.text_dim)
                }),
        ]
        .padding(metric::PAD);
        col = col.push(row_el);
    }

    let scroll = scrollable(col)
        .height(Length::Fill)
        .style(move |_: &iced::Theme, _| iced::widget::scrollable::Style {
            container: iced::widget::container::Style {
                background: Some(iced::Background::Color(p.card)),
                border: Border {
                    radius: 20.0.into(),
                    width: 0.0,
                    color: p.border,
                },
                text_color: Some(p.accent_dim),
                ..Default::default()
            },
            vertical_rail: iced::widget::scrollable::Rail {
                background: Some(iced::Background::Color(p.accent_tint)),
                scroller: iced::widget::scrollable::Scroller {
                    background: iced::Background::Color(p.accent),
                    border: Border {
                        color: p.accent_dim,
                        width: 0.3,
                        radius: 20.0.into(),
                    },
                },
                border: Border {
                    color: p.border,
                    width: 0.3,
                    radius: 20.0.into(),
                },
            },
            horizontal_rail: iced::widget::scrollable::Rail {
                background: Some(iced::Background::Color(p.accent_dim)),
                scroller: iced::widget::scrollable::Scroller {
                    background: iced::Background::Color(p.accent_dim),
                    border: iced::Border::default(),
                },
                border: iced::Border::default(),
            },
            gap: None,
            auto_scroll: iced::widget::scrollable::AutoScroll {
                background: iced::Background::Color(p.accent_dim),
                border: iced::Border::default(),
                icon: p.bg,
                shadow: iced::Shadow {
                    ..Default::default()
                },
            },
        });

    scroll.into()
}

pub fn sensors_section(app: &crate::app::Snapdash, p: Palette) -> Element<'_, Message> {
    // Seznam všech entit
    // Např. jen senzory:
    // let sensors = entities
    //     .iter()
    //     .filter(|e| e.entity_id.starts_with("sensor."));

    let query = app.entity_search_query.trim().to_lowercase();

    let sensors = app
        .settings_sensors
        .iter()
        .filter(|sensor| query.is_empty() || sensor.search_key.contains(&query));

    let mut col = column![];

    for sensor in sensors {
        let id = sensor.entity_id.clone();
        let friendly = sensor.friendly_name.clone();
        let selected = app.selected_widgets.contains(&id);

        let id_for_msg = id.clone();

        let row_el = row![
            checkbox(selected)
                .label(friendly)
                .on_toggle(move |_| Message::ToggleWidget(id_for_msg.clone()))
                .text_size(14)
                .style(move |_, _| iced::widget::checkbox::Style {
                    background: iced::Background::Color(p.bg),
                    text_color: Some(p.text_primary),
                    icon_color: p.accent,
                    border: Border {
                        color: p.text_primary,
                        width: 0.5,
                        radius: 15.0.into()
                    }
                }),
            text(format!(" ({})", sensor.entity_id))
                .size(14)
                .style(move |_: &iced::Theme| iced::widget::text::Style {
                    color: Some(p.text_dim)
                }),
        ]
        .padding(8);
        col = col.push(row_el);
    }

    let scroll = scrollable(col)
        .height(Length::Fill)
        .style(move |_: &iced::Theme, _| iced::widget::scrollable::Style {
            container: iced::widget::container::Style {
                background: Some(iced::Background::Color(p.card)),
                border: Border {
                    radius: 20.0.into(),
                    width: 0.0,
                    color: p.border,
                },
                text_color: Some(p.accent_dim),
                ..Default::default()
            },
            vertical_rail: iced::widget::scrollable::Rail {
                background: Some(iced::Background::Color(p.accent_tint)),
                scroller: iced::widget::scrollable::Scroller {
                    background: iced::Background::Color(p.accent),
                    border: Border {
                        color: p.accent_dim,
                        width: 0.3,
                        radius: 20.0.into(),
                    },
                },
                border: Border {
                    color: p.border,
                    width: 0.3,
                    radius: 20.0.into(),
                },
            },
            horizontal_rail: iced::widget::scrollable::Rail {
                background: Some(iced::Background::Color(p.accent_dim)),
                scroller: iced::widget::scrollable::Scroller {
                    background: iced::Background::Color(p.accent_dim),
                    border: iced::Border::default(),
                },
                border: iced::Border::default(),
            },
            gap: None,
            auto_scroll: iced::widget::scrollable::AutoScroll {
                background: iced::Background::Color(p.accent_dim),
                border: iced::Border::default(),
                icon: p.bg,
                shadow: iced::Shadow {
                    ..Default::default()
                },
            },
        });

    scroll.into()
}

pub fn fieldset<'a>(
    legend: &'a str,
    content: Element<'a, Message>,
    p: Palette,
) -> Element<'a, Message> {
    let legend_chip: Element<'a, Message> = container(text(legend).size(13).style(
        move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(p.text_dim),
        },
    ))
    .padding([0, 8])
    .style(move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(p.card_2)),
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
