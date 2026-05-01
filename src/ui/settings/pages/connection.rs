use iced::widget::{column, row};
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;
use crate::ui::icon::Icon;
use crate::ui::theme::UiTheme;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();
    let ui_theme = UiTheme::from(&snap.theme);

    let url: Element<Message> = components::mac_input("Home Assistant URL", &snap.config.ha_url, p)
        .on_input(Message::HaUrlChanged)
        .into();

    let placeholder = match snap.config.ha_token_present {
        true => "Token stored in key-chain",
        _ => "Enter your token ...",
    };

    let token: Element<Message> = components::mac_input(placeholder, &snap.ha.token_draft, p)
        .on_input(Message::HaTokenDraftChanged)
        .into();

    let token_delete: Element<Message> =
        components::pill_button(Icon::Trash.text(ui_theme), p, Some(Message::HaTokenDelete)).into();

    let body = components::subcard(
        column![
            components::section("Home Assistant", p),
            url,
            row![token, token_delete]
                .spacing(metric::GAP)
                .align_y(iced::Alignment::Center)
        ]
        .spacing(metric::GAP)
        .into(),
        p,
    );

    column![components::title(snap.settings_page.label(), p), body]
        .width(Length::Fill)
        .spacing(metric::GAP)
        .into()
}
