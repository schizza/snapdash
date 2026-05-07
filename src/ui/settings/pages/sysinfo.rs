use crate::app::Message;
use iced::system;

use crate::app::Snapdash;

pub fn view<'a>(snap: &Snapdash) -> Element<'a, Message> {
    let s = system::information()
}
