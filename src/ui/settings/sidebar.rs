use iced::widget::button::{Status, Style};
use iced::widget::{Button, button, column, text};
use iced::{Background, Border, Color, Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::{Palette, metric};
use crate::ui::components;

use super::SettingsPage;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let search: Element<Message> =
        components::mac_input("Search settings ...", &snap.settings_search, p)
            .on_input(Message::SettingsSearchChanged)
            .width(Length::Fill)
            .into();

    let mut nav = column![].spacing(2);
    for &page in SettingsPage::ALL {
        nav = nav.push(nav_item(page, snap.settings_page == page, p));
    }

    column![search, nav].spacing(metric::GAP).width(180).into()
}

fn nav_item(page: SettingsPage, is_active: bool, p: Palette) -> Button<'static, Message> {
    button(text(page.label()).size(13))
        .style(move |_theme, status| {
            let bg = if is_active {
                p.accent_tint
            } else if matches!(status, Status::Hovered) {
                p.card_2
            } else {
                Color::TRANSPARENT
            };

            Style {
                background: Some(Background::Color(bg)),
                text_color: p.text_body,
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .on_press(Message::SettingsPageSelected(page))
        .width(Length::Fill)
        .padding([6, 12])
}
