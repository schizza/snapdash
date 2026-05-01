use iced::overlay::menu;
use iced::widget::{pick_list, text_input};
use iced::{Background, Border};

use crate::app::Message;
use crate::theme::{Palette, ThemeKind, metric};

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

pub fn picker<'a, V, M>(
    options: Vec<V>,
    selected: V,
    on_select: impl Fn(V) -> M + 'a,
    p: Palette,
) -> pick_list::PickList<'a, V, Vec<V>, V, M>
where
    V: ToString + PartialEq + Clone + 'a,
    M: Clone + 'a,
{
    use iced::widget::pick_list::Status;

    pick_list(options, Some(selected), on_select)
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

/// wrapper aroud pick_list
pub fn themepicker(
    options: Vec<ThemeKind>,
    selected: ThemeKind,
    p: Palette,
) -> pick_list::PickList<'static, ThemeKind, Vec<ThemeKind>, ThemeKind, Message> {
    picker(options, selected, Message::ThemeSelected, p)
}
