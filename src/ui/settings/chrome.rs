//! Persistent chrome around the settings pages: title bar (drag area +
//! update badge + close), the save/quit row at the bottom, and the
//! footer status line. Identical for every page — pages render in the
//! gap between these pieces.

use iced::widget::{container, mouse_area, row};
use iced::{Element, Length, mouse, window};

use crate::app::{Message, Snapdash};
use crate::theme::{metric, text_size};
use crate::ui::components;
use crate::ui::icon::Icon;
use crate::ui::update_view;

pub fn title_bar<'a>(snap: &'a Snapdash, id: window::Id) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let update_badge: Element<Message> = if snap.update.is_available() {
        mouse_area(components::badge_with_icon(
            "Update Available",
            Icon::Download,
            p,
        ))
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
        components::pill_button_with(
            Icon::Close.text(p).size(text_size::NORMAL),
            components::ButtonVisual::pill(p),
            Some(Message::CloseWindow(id))
        ),
    ]
    .spacing(metric::GAP)
    .align_y(iced::Alignment::Center)
    .into()
}

pub fn footer<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let status: Element<Message> = if !snap.status.is_empty() {
        components::dimmed(snap.status.clone(), p).into()
    } else {
        components::dimmed("Status", p).into()
    };

    //Save = primary action (filled accent)
    let save_btn = components::primary_button("Save", p, Some(Message::SavePressed));

    let quit_btn = components::pill_button("Quit", p, Some(Message::QuitApp));
    let update_icon = update_view::status_icon(snap.update.state, p);
    let update_icon: Element<Message> = if snap.update.is_available() {
        iced::widget::mouse_area(update_icon)
            .on_press(Message::SettingsPageSelected(
                crate::ui::settings::SettingsPage::Updates,
            ))
            .interaction(iced::mouse::Interaction::Pointer)
            .into()
    } else {
        update_icon.into()
    };
    let version: Element<Message> = iced::widget::row![
        components::dimmed(format!("version: {}", crate::update::CURRENT_VERSION), p),
        update_icon,
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .into();

    iced::widget::row![
        status,
        iced::widget::space().width(iced::Length::Fill),
        save_btn,
        quit_btn,
        iced::widget::space().width(metric::GAP),
        version,
    ]
    .spacing(metric::GAP)
    .align_y(iced::Alignment::Center)
    .into()
}
