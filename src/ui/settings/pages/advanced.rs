use iced::Element;
use iced::widget::{column, row};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let config_row = row![
        column![
            components::dimmed("Edit config.json", p),
            components::dimmed("Open the JSON config in your default editor.", p,),
        ]
        .width(iced::Length::Fill),
        components::pill_button("Open", p, Some(Message::OpenConfigFile)),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(metric::GAP);

    let log_row = row![
        column![
            components::dimmed("Open log directory", p),
            components::dimmed("Browse runtime logs in your file manager.", p,),
        ]
        .width(iced::Length::Fill),
        components::pill_button("Open", p, Some(Message::OpenLogFile)),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(metric::GAP);

    let reset_row = row![
        column![
            components::dimmed("Reset to defaults", p),
            components::dimmed(
                "Wipes HA URL, theme and selected sensors. The HA token in the keychain is not affected.",
                p,
            ),
        ]
        .width(iced::Length::Fill),
        components::pill_button("Reset", p, Some(Message::ResetConfig)),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(metric::GAP);

    let body = column![
        components::section("Configuration", p),
        config_row,
        iced::widget::space().height(metric::GAP),
        log_row,
        iced::widget::space().height(metric::GAP),
        reset_row,
    ]
    .spacing(metric::GAP);

    components::subcard(body.into(), p)
}
