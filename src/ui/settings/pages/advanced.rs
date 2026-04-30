use iced::widget::{column, row};
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let config_row = row![
        column![
            components::body("Edit config.json", p),
            components::helper("Open the JSON config in your default editor.", p,),
        ]
        .width(iced::Length::Fill),
        components::pill_button("Open", p, Some(Message::OpenConfigFile)),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(metric::GAP);

    let log_row = row![
        column![
            components::body("Open log directory", p),
            components::helper("Browse runtime logs in your file manager.", p,),
        ]
        .width(iced::Length::Fill),
        components::pill_button("Open", p, Some(Message::OpenLogFile)),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(metric::GAP);

    let reset_row = row![
        column![
            components::body("Reset to defaults", p),
            components::helper(
                "Wipes HA URL, theme and selected sensors. The HA token in the keychain is not affected.",
                p,
            ),
        ]
        .width(iced::Length::Fill),
        components::pill_button_with("Reset", components::ButtonVisual::pill(p).bg(p.danger).bg_hovered(p.danger), Some(Message::ResetConfig)),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(metric::GAP);

    let body = column![
        config_row,
        iced::widget::space().height(metric::GAP),
        log_row,
        iced::widget::space().height(metric::GAP),
        reset_row,
    ]
    .spacing(metric::GAP);
    column![
        components::title(snap.settings_page.label(), p),
        components::subcard(body.into(), p)
    ]
    .spacing(metric::PAD)
    .width(Length::Fill)
    .into()
}
