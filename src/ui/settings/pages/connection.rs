use iced::Element;

use crate::app::{Message, Snapdash};
use crate::theme::text_size;
use crate::ui::components::{self, settings_components};
use crate::ui::icon::Icon;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let token_item = if !snap.config.ha_token_present {
        settings_components::item_with_input(
            "Token",
            Some("Enter your Home Assistant`s Long-lived token"),
            "Long-Lived Token belong here ...",
            &snap.ha.token_draft,
            Message::HaTokenDraftChanged,
            Some(Message::SavePressed),
            p,
        )
    } else {
        let title_element = components::success_message("Token is stored in the key-chain", p);
        let action_element = components::danger_button_with(
            Icon::Trash.text(p).size(text_size::LARGE).into(),
            p,
            Some(Message::HaTokenDelete),
        )
        .into();
        settings_components::item_with_element(title_element, action_element)
    };

    settings_components::page(
        "Connection",
        [
            settings_components::item_with_input(
                "Home Assistant URL",
                Some("Enter fully qualified URL or IP (with http or https prefix)."),
                "Enter your Home Assistant's URL or IP address ...",
                &snap.config.ha_url,
                Message::HaUrlChanged,
                Some(Message::SavePressed),
                p,
            ),
            token_item,
        ],
        p,
    )
}
