use iced::widget::{column, container, row};
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components::{self, active_sensor_section, sensors_section};

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let search: Element<Message> =
        components::mac_input("Search entities ...", &snap.entity_search_query, p)
            .on_input(Message::EntitySearchChanged)
            .width(Length::Fill)
            .into();

    let available_panel: Element<Message> = components::fieldset(
        "Available",
        column![search, sensors_section(snap, p)]
            .spacing(metric::GAP)
            .width(Length::Fill)
            .into(),
        p,
    );

    let selected_panel: Element<Message> = components::fieldset(
        "Selected",
        column![active_sensor_section(snap, p)]
            .spacing(metric::GAP)
            .width(Length::Fill)
            .into(),
        p,
    );

    let body = components::subcard(
        //column![
             row![
                container(available_panel).width(Length::FillPortion(1)),
                container(selected_panel).width(Length::FillPortion(1)),
            ]
            .spacing(metric::GAP)
            .align_y(iced::Alignment::Start)
            .width(Length::Fill)
            .height(Length::Fill)
        
        //.spacing(metric::GAP)
        //.width(Length::Fill)
        //.height(Length::Fill)
        .into(),
        p,
    );

    column![components::title(&snap.settings_page.label(), p), body]
        .width(Length::Fill)
        .spacing(metric::GAP)
        .into()
}
