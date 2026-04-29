use iced::widget::{column, container, mouse_area, row};
use iced::{Element, Length, mouse};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components::dimmed;
use crate::ui::icon::Icon;

use crate::ui::theme::UiTheme;
use crate::ui::update_view;
use crate::update;

use super::components;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    Connection,
    General,
    Appearance,
    Sensors,
    Updates,
    Advanced,
}

impl SettingsPage {
    pub const ALL: &[Self] = &[
        Self::General,
        Self::Connection,
        Self::Appearance,
        Self::Sensors,
        Self::Updates,
        Self::Advanced,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Connection => "Connection",
            Self::Appearance => "Appearance",
            Self::Sensors => "Sensors",
            Self::Updates => "Updates",
            Self::Advanced => "Advanced",
        }
    }
}

mod pages;

pub fn view(snap: &Snapdash, id: iced::window::Id) -> Element<'_, Message> {
    let p = snap.theme.palette();
    let ui_theme = UiTheme::from(&snap.theme);

    let connection_card = pages::connection::view(snap);
    let appearance_card = pages::appearance::view(snap);
    let sensors_card = pages::sensors::view(snap);

    let status: Element<Message> = if !snap.status.is_empty() {
        dimmed(snap.status.clone(), p).into()
    } else {
        dimmed("Status", p).into()
    };

    let update_icon = update_view::status_icon(snap.update.state, ui_theme, p);

    let update_icon: Element<Message> = if snap.update.is_available() {
        mouse_area(update_icon)
            .on_press(Message::OpenReleaseNotes)
            .interaction(iced::mouse::Interaction::Pointer)
            .into()
    } else {
        update_icon.into()
    };

    let version: Element<Message> = row![
        dimmed(format!("version: {} ", update::CURRENT_VERSION), p,),
        update_icon
    ]
    .padding([4, 0])
    // .align_x(iced::Alignment::End)
    .into();

    let update_badge: Element<Message> = if snap.update.is_available() {
        mouse_area(components::badge("Update Available", p))
            .on_press(Message::OpenReleaseNotes)
            .interaction(mouse::Interaction::Pointer)
            .into()
    } else {
        iced::widget::space().width(0).height(0).into()
    };

    let title_bar: Element<Message> = row![
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
    .into();

    let save_row: Element<Message> = row![
        components::pill_button("Save", p, Some(Message::SavePressed)),
        iced::widget::space().width(Length::Fill),
        components::pill_button("Quit App", p, Some(Message::QuitApp)),
    ]
    .spacing(metric::GAP)
    .align_y(iced::Alignment::Center)
    .into();

    let status_row: Element<Message> = row![
        column![status].width(iced::Length::Fill),
        column![version].align_x(iced::Alignment::End)
    ]
    .width(iced::Fill)
    .into();

    let mut content = column![title_bar, iced::widget::space().height(2),]
        .spacing(14)
        .height(Length::Fill)
        .push(connection_card)
        .push(appearance_card);

    content = content.push(sensors_card).push(save_row).push(status_row);

    let content: Element<Message> = content.into();

    components::card(content, p)
}
