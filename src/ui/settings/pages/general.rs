use iced::widget::column;
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;
use crate::update;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    // let body = column![
    //     components::section("About Snapdash", p),
    //     components::dimmed(format!("Snapdash {}", update::CURRENT_VERSION), p,),
    //     components::dimmed("Lightweight Home Assistant dashboard.", p),
    // ]
    // .spacing(metric::GAP);

    let body = column![
        iced::widget::text("Snapdash")
            .size(28)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.text_primary),
            }),
        iced::widget::text(format!("Version {}", update::CURRENT_VERSION))
            .size(13)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.text_secondary),
            }),
        iced::widget::space().height(metric::GAP),
        iced::widget::text("Lightweight Home Assistant dashboard.")
            .size(13)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.text_body),
            }),
    ]
    .spacing(4);

    column![
        components::title(snap.settings_page.label(), p),
        components::subcard(body.into(), p)
    ]
    .spacing(metric::PAD)
    .width(Length::Fill)
    .into()
}
