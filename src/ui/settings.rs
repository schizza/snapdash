use iced::Element;
use iced::widget::{column, row};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components::{label, sensors_section};

use super::components;

pub fn view(snap: &Snapdash) -> Element<'_, Message> {
    let p = snap.theme.palette();

    let url: Element<Message> = components::mac_input("Home Assistant URL", &snap.config.ha_url, p)
        .on_input(Message::HaUrlChanged)
        .into();
    let placeholder = match snap.config.ha_token_present {
        true => "Token stored in key-chain",
        _ => "Enter your token ...",
    };

    let token: Element<Message> =
        components::mac_input(placeholder.into(), &snap.ha_token_draft, p)
            .on_input(Message::HaTokenDraftChanged)
            .into();

    let token_delete: Element<Message> =
        components::icon("🗑", p, Some(Message::HaTokenDelete)).into();

    let theme_picker: Element<Message> =
        components::themepicker(snap.theme_options.clone(), snap.theme, p).into();

    let status: Element<Message> = if !snap.status.is_empty() {
        label(snap.status.clone(), p).into()
    } else {
        label("-", p).into()
    };

    let content = column![
        components::title("Snapdash Settings", p),
        iced::widget::space().height(6),
        components::subcard(
            column![
                components::section("Home Assistant", p),
                url,
                token,
                token_delete
            ]
            .spacing(metric::GAP)
            .into(),
            p
        ),
        components::subcard(
            row![components::section("Theme", p), theme_picker]
                .spacing(metric::GAP)
                .align_y(iced::Alignment::Center)
                .into(),
            p
        ),
        row![
            components::pill_button("Save", p, Some(Message::SavePressed)),
            components::pill_button(
                "Open entity window",
                p,
                Some(Message::OpenEntity(
                    "sensor.home_udmpro_cpu_utilization_3".into()
                ))
            )
        ],
        components::section("sensors", p),
        sensors_section(snap, p),
        row![status]
    ]
    .spacing(14)
    .into();

    components::card(content, p)
}
