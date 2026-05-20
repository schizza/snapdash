use iced::widget::column;
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;

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
    Sysinfo,
}

impl SettingsPage {
    pub const ALL: &[Self] = &[
        Self::General,
        Self::Connection,
        Self::Appearance,
        Self::Sensors,
        Self::Updates,
        Self::Advanced,
        Self::Sysinfo,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Connection => "Connection",
            Self::Appearance => "Appearance",
            Self::Sensors => "Sensors",
            Self::Updates => "Updates",
            Self::Advanced => "Advanced",
            Self::Sysinfo => "System information",
        }
    }
}

mod chrome;
mod pages;
mod sidebar;

pub fn view(snap: &Snapdash, id: iced::window::Id) -> Element<'_, Message> {
    let p = snap.theme.palette();

    let content: Element<Message> = match snap.settings_page {
        SettingsPage::General => pages::general::view(snap),
        SettingsPage::Connection => pages::connection::view(snap),
        SettingsPage::Appearance => pages::appearance::view(snap),
        SettingsPage::Sensors => pages::sensors::view(snap),
        SettingsPage::Updates => pages::updates::view(snap),
        SettingsPage::Advanced => pages::advanced::view(snap),
        SettingsPage::Sysinfo => pages::sysinfo::view(snap, p),
    };

    let body: Element<Message> = iced::widget::row![
        sidebar::view(snap),
        iced::widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(metric::PAD)
    ]
    .spacing(metric::GAP)
    .height(Length::Fill)
    .into();

    let outer = column![chrome::title_bar(snap, id), body, chrome::footer(snap)].spacing(14);

    components::card(outer.into(), p)
}
