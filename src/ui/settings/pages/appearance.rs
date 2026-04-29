use iced::Element;
use iced::widget::row;

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let theme_picker: Element<Message> =
        components::themepicker(snap.theme_options.clone(), snap.theme, p).into();

    components::subcard(
        row![components::section("Theme", p), theme_picker]
            .spacing(metric::GAP)
            .align_y(iced::Alignment::Center)
            .into(),
        p,
    )
}
