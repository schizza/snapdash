use iced::widget::{column, row};
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let theme_picker: Element<Message> =
        components::themepicker(snap.theme_options.clone(), snap.theme, p).into();

    let body = components::subcard(
        row![components::section("Theme", p), theme_picker]
            .spacing(metric::GAP)
            .align_y(iced::Alignment::Center)
            .into(),
        p,
    );

    column![components::title(snap.settings_page.label(), p), body]
        .spacing(metric::PAD)
        .width(Length::Fill)
        .into()
}
