use iced::Element;
use iced::widget::column;

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;
use crate::update;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let body = column![
        components::section("About Snapdash", p),
        components::dimmed(format!("Snapdash {}", update::CURRENT_VERSION), p,),
        components::dimmed("Lightweight Home Assistant dashboard.", p),
    ]
    .spacing(metric::GAP);

    components::subcard(body.into(), p)
}
