use iced::widget::{column, row};
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components::{self, body_with_helper};
use crate::widget_size::WidgetSize;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let theme_picker: Element<Message> =
        components::themepicker(snap.theme_options.clone(), snap.theme, p).into();

    let theme_row: Element<Message> = row![
        components::body("Theme", p),
        iced::widget::space().width(iced::Length::Fill),
        theme_picker
    ]
    .align_y(iced::Alignment::Center)
    .into();

    let size_picker: Element<Message> = components::picker(
        WidgetSize::ALL.to_vec(),
        snap.config.widget_size,
        Message::WidgetSizeChanged,
        p,
    )
    .into();

    let size_row: Element<Message> = row![
        body_with_helper("Widget size", "Affects new and currently opened widgets", p),
        iced::widget::space().width(Length::Fill),
        size_picker
    ]
    .align_y(iced::Alignment::Center)
    .spacing(metric::GAP)
    .into();

    let body: Element<Message> = column![theme_row, size_row].spacing(metric::GAP).into();

    column![components::title(snap.settings_page.label(), p), body]
        .spacing(metric::PAD)
        .width(Length::Fill)
        .into()
}
