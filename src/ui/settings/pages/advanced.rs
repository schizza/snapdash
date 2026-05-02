use iced::Element;

use crate::app::{Message, Snapdash};
use crate::ui::components::ButtonType;
use crate::ui::components::settings_components::{item_with_button, page};

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    page(
        "Advanced",
        [
            item_with_button(
                "Edit config.json",
                Some("Open the JSON config in your default editor."),
                "Open",
                Some(Message::OpenConfigFile),
                ButtonType::PILL,
                p,
            ),
            item_with_button(
                "Open log directory",
                Some("Browse runtime logs in your file manager."),
                "Open",
                Some(Message::OpenLogFile),
                ButtonType::PILL,
                p,
            ),
            item_with_button(
                "Reset to defaults",
                Some(
                    "Wipes HA URL, theme and selected sensors. The HA token in the keychain is not affected.",
                ),
                "Reset",
                Some(Message::ResetConfig),
                ButtonType::DANGER,
                p,
            ),
        ],
        p,
    )
}
