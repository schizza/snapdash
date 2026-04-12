#[cfg(feature = "diagnostics")]
use iced::widget::checkbox;
use iced::widget::{column, container, mouse_area, row};
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components::{active_sensor_section, dimmed, sensors_section};

use super::components;

pub fn view(snap: &Snapdash, id: iced::window::Id) -> Element<'_, Message> {
    let p = snap.theme.palette();

    let url: Element<Message> = components::mac_input("Home Assistant URL", &snap.config.ha_url, p)
        .on_input(Message::HaUrlChanged)
        .into();
    let placeholder = match snap.config.ha_token_present {
        true => "Token stored in key-chain",
        _ => "Enter your token ...",
    };

    let token: Element<Message> = components::mac_input(placeholder, &snap.ha_token_draft, p)
        .on_input(Message::HaTokenDraftChanged)
        .into();

    let token_delete: Element<Message> =
        components::icon("🗑", p, Some(Message::HaTokenDelete)).into();

    let theme_picker: Element<Message> =
        components::themepicker(snap.theme_options.clone(), snap.theme, p).into();

    let status: Element<Message> = if !snap.status.is_empty() {
        dimmed(snap.status.clone(), p).into()
    } else {
        dimmed("Status", p).into()
    };
    let search: Element<Message> =
        components::mac_input("Search entities ...", &snap.entity_search_query, p)
            .on_input(Message::EntitySearchChanged)
            .width(Length::Fill)
            .into();

    let available_panel: Element<Message> = components::fieldset(
        "Available",
        column![search, sensors_section(snap, p),]
            .spacing(metric::GAP)
            .width(Length::Fill)
            .into(),
        p,
    );

    let selected_panel: Element<Message> = components::fieldset(
        "Selected",
        column![active_sensor_section(snap, p),]
            .spacing(metric::GAP)
            .width(Length::Fill)
            .into(),
        p,
    );

    let home_assistant_card = components::subcard(
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

    let theme_card = components::subcard(
        row![components::section("Theme", p), theme_picker]
            .spacing(metric::GAP)
            .align_y(iced::Alignment::Center)
            .into(),
        p,
    );

    let sensors_card = components::subcard(
        column![
            components::section("Sensors", p),
            row![
                container(available_panel).width(Length::FillPortion(1)),
                container(selected_panel).width(Length::FillPortion(1)),
            ]
            .spacing(metric::GAP)
            .align_y(iced::Alignment::Start)
            .width(Length::Fill)
            .height(Length::Fill)
        ]
        .spacing(metric::GAP)
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
        p,
    );

    let title_bar: Element<Message> = row![
        mouse_area(
            container(components::title("Snapdash Settings", p))
                .width(Length::Fill)
                .padding([4, 0])
        )
        .on_press(Message::StartDrag(id)),
        components::icon("✕", p, Some(Message::CloseWindow(id))),
    ]
    .spacing(metric::GAP)
    .align_y(iced::Alignment::Center)
    .into();

    let save_row: Element<Message> = row![
        components::pill_button("Save", p, Some(Message::SavePressed)),
        iced::widget::space().width(Length::Fill),
        components::pill_button("Quit App", p, Some(Message::QuitApp)),
    ]
    .spacing(metric::GAP)
    .align_y(iced::Alignment::Center)
    .into();

    let status_row: Element<Message> = row![status].into();

    let mut content = column![title_bar, iced::widget::space().height(2),]
        .spacing(14)
        .height(Length::Fill)
        .push(home_assistant_card)
        .push(theme_card);

    #[cfg(feature = "diagnostics")]
    {
        let debug_toggle: Element<Message> = checkbox(snap.config.debug_overlay)
            .label("Show debug overlay in windows")
            .on_toggle(Message::DebugOverlayToggled)
            .text_size(14)
            .style(move |_, _| iced::widget::checkbox::Style {
                background: iced::Background::Color(p.bg),
                text_color: Some(p.text_primary),
                icon_color: p.accent,
                border: iced::Border {
                    color: p.text_primary,
                    width: 0.5,
                    radius: 15.0.into(),
                },
            })
            .into();

        let diagnostics_card = components::subcard(
            column![
                components::section("Diagnostics", p),
                debug_toggle,
                components::dimmed(
                    "FPS is derived from real iced redraw events after diagnostics are enabled.",
                    p
                ),
            ]
            .spacing(metric::GAP)
            .into(),
            p,
        );

        content = content.push(diagnostics_card);
    }

    content = content.push(sensors_card).push(save_row).push(status_row);

    let content: Element<Message> = content.into();

    components::card(content, p)
}
