use iced::Element;

use crate::app::{Message, Snapdash};
use crate::ui::components::ButtonType;
use crate::ui::components::settings_components::{
    item_with_button, item_with_icon_button, page_with_sections, section,
};
use crate::ui::icon;
use crate::{helpers, logger};

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let size = logger::get_log_size().unwrap_or(0);
    let log_size_helper = format!(
        "Truncate the runtime log file. Current size is {}",
        helpers::humanize_bytes(size)
    );

    page_with_sections(
        "Advanced",
        [
            // Edit section
            section(
                [item_with_icon_button(
                    "Edit config.json",
                    Some("Open the JSON config in your default editor."),
                    icon::Icon::ExternalLink,
                    Message::OpenConfigFile,
                    p,
                )],
                p,
            ),
            // Log section
            section(
                [
                    item_with_icon_button(
                        "Open log directory",
                        Some("Browse runtime logs in your file manager."),
                        icon::Icon::ExternalLink,
                        Message::OpenLogFile,
                        p,
                    ),
                    item_with_icon_button(
                        "Clear log file",
                        Some(log_size_helper),
                        icon::Icon::Refresh,
                        Message::TruncateLogFile,
                        p,
                    ),
                ],
                p,
            ),
            section(
                [item_with_button(
                    "Reset to defaults",
                    Some(
                        "Wipes HA URL, theme and selected sensors. The HA token in the keychain is not affected.",
                    ),
                    "Reset",
                    Some(Message::ResetConfig),
                    ButtonType::DANGER,
                    p,
                )],
                p,
            ),
        ],
        p,
    )
}
