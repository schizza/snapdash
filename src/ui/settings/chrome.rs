//! Persistent chrome around the settings pages: title bar (drag area +
//! update badge + close), the save/quit row at the bottom, and the
//! footer status line. Identical for every page — pages render in the
//! gap between these pieces.

use iced::widget::{column, container, mouse_area, row};
use iced::{Element, Length, mouse, window};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;
use crate::ui::icon::Icon;
use crate::ui::theme::UiTheme;
use crate::ui::update_view;
use crate::update;

pub fn title_bar<'a>(snap: &'a Snapdash, id: window::Id) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let ui_theme = UiTheme::from(&snap.theme);

    let update_badge: Element<Message> = if snap.update.is_available() {
        mouse_area(components::badge("Update Available", p))
            .on_press(Message::OpenReleaseNotes)
            .interaction(mouse::Interaction::Pointer)
            .into()
    } else {
        iced::widget::space().width(0).height(0).into()
    };

    row![
        mouse_area(
            container(components::title("Snapdash Settings", p))
                .width(Length::Fill)
                .padding([4, 0])
        )
        .on_press(Message::StartDrag(id)),
        update_badge,
        components::icon(
            Icon::Close.text(ui_theme),
            p,
            Some(Message::CloseWindow(id))
        ),
    ]
    .spacing(metric::GAP)
    .align_y(iced::Alignment::Center)
    .into()
}

pub fn save_row<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    row![
        components::pill_button("Save", p, Some(Message::SavePressed)),
        iced::widget::space().width(Length::Fill),
        components::pill_button("Quit App", p, Some(Message::QuitApp)),
    ]
    .spacing(metric::GAP)
    .align_y(iced::Alignment::Center)
    .into()
}

pub fn status_row<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();
    let ui_theme = UiTheme::from(&snap.theme);

    let status: Element<Message> = if !snap.status.is_empty() {
        components::dimmed(snap.status.clone(), p).into()
    } else {
        components::dimmed("Status", p).into()
    };

    let update_icon = update_view::status_icon(snap.update.state, ui_theme, p);
    let update_icon: Element<Message> = if snap.update.is_available() {
        mouse_area(update_icon)
            .on_press(Message::OpenReleaseNotes)
            .interaction(mouse::Interaction::Pointer)
            .into()
    } else {
        update_icon.into()
    };

    let version: Element<Message> = row![
        components::dimmed(format!("version: {} ", update::CURRENT_VERSION), p),
        update_icon
    ]
    .padding([4, 0])
    .into();

    row![
        column![status].width(Length::Fill),
        column![version].align_x(iced::Alignment::End)
    ]
    .width(Length::Fill)
    .into()
}
