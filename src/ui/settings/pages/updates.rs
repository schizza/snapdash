use iced::widget::{column, row};
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components::{self, error_message, success_message};
use crate::ui::theme::UiTheme;
use crate::ui::update_view;
use crate::update::{self, InstallProgress, UpdateState};

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();
    let ui_theme = UiTheme::from(&snap.theme);

    let latest = match &snap.update.latest_release {
        Some(release) => release.tag_name.to_string(),
        None => "N/A".to_string(),
    };

    let status_text = match snap.update.state {
        UpdateState::Unknown => "Checking for updates…".to_string(),
        UpdateState::UptoDate => "You're up to date.".to_string(),
        UpdateState::UpdateAvailable => format!("A new version is available: {}", latest),
    };

    let status_row = row![
        update_view::status_icon(snap.update.state, ui_theme, p),
        components::label(status_text, p),
    ]
    .spacing(metric::GAP)
    .align_y(iced::Alignment::Center);

    let install_block: Element<Message> = match &snap.update.install {
        InstallProgress::Idle if snap.update.is_available() => {
            components::primary_button("Install update", p, Some(Message::InstallUpdate)).into()
        }
        InstallProgress::Idle => iced::widget::space().width(0).height(0).into(),
        InstallProgress::Installing => {
            components::pill_button("Installing update ...", p, None).into()
        }
        InstallProgress::ReadyToRestart(exec) => {
            let exec = exec.clone();
            components::pill_button("Restart now", p, Some(Message::RestartAfterUpdate(exec)))
                .into()
        }
        InstallProgress::Failed(_err) => {
            components::pill_button("Retry", p, Some(Message::InstallUpdate)).into()
        }
    };

    // Status line
    let install_status: Option<Element<Message>> = match &snap.update.install {
        InstallProgress::Failed(err) => Some(error_message(format!("Update failed: {err}"), p)),
        InstallProgress::ReadyToRestart(..) => {
            Some(success_message("Update succesfull. Restart to apply.", p))
        }
        _ => None,
    };

    let actions: Element<Message> = if !snap.update.is_available() {
        row![
            components::pill_button("Check for updates", p, Some(Message::CheckForUpdate),),
            components::pill_button(
                "Show latest release notes",
                p,
                Some(Message::OpenReleaseNotes),
            ),
        ]
        .spacing(metric::GAP)
        .into()
    } else {
        row![
            install_block,
            components::pill_button(
                "Show latest release notes",
                p,
                Some(Message::OpenReleaseNotes),
            ),
        ]
        .spacing(metric::GAP)
        .into()
    };
    let mut body = column![
        components::label(format!("Current version: {}", update::CURRENT_VERSION), p,),
        iced::widget::space().height(metric::GAP),
        status_row,
        iced::widget::space().height(metric::GAP),
        actions,
    ]
    .spacing(metric::GAP);

    if let Some(status_box) = install_status {
        body = body.push(status_box);
        body = body.push(iced::widget::space().height(metric::GAP));
    }

    column![
        components::title(snap.settings_page.label(), p),
        components::subcard(body.into(), p)
    ]
    .spacing(metric::PAD)
    .width(Length::Fill)
    .into()
}
