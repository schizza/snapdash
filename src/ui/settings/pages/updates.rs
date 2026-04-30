use iced::widget::{column, row};
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;
use crate::ui::theme::UiTheme;
use crate::ui::update_view;
use crate::update::{self, UpdateState};

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();
    let ui_theme = UiTheme::from(&snap.theme);

    let status_text = match snap.update.state {
        UpdateState::Unknown => "Checking for updates…",
        UpdateState::UptoDate => "You're up to date.",
        UpdateState::UpdateAvailable => "A new version is available.",
    };

    let status_row = row![
        update_view::status_icon(snap.update.state, ui_theme, p),
        components::label(status_text, p),
    ]
    .spacing(metric::GAP)
    .align_y(iced::Alignment::Center);

    let actions: Element<Message> = if snap.update.is_available() {
        row![
            components::pill_button("Check for updates", p, Some(Message::CheckForUpdate),),
            components::pill_button("Show release notes", p, Some(Message::OpenReleaseNotes),),
        ]
        .spacing(metric::GAP)
        .into()
    } else {
        components::pill_button("Check for updates", p, Some(Message::CheckForUpdate)).into()
    };

    let body = column![
        components::section("Version", p),
        components::label(format!("Current: {}", update::CURRENT_VERSION), p,),
        iced::widget::space().height(metric::GAP),
        status_row,
        iced::widget::space().height(metric::GAP),
        actions,
    ]
    .spacing(metric::GAP);

    column![
        components::title(&snap.settings_page.label(), p),
        components::subcard(body.into(), p)
    ]
    .spacing(metric::PAD)
    .width(Length::Fill)
    .into()
}
