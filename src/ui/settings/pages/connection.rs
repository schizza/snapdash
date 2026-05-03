use iced::Element;

use crate::app::{Message, Snapdash};
use crate::ha::token::TokenPresence;
use crate::theme::text_size;
use crate::ui::components::{self, ButtonVisual, settings_components};
use crate::ui::icon::Icon;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let token_item = match &snap.token_presence {
        TokenPresence::Missing => settings_components::item_with_input(
            "Token",
            Some("Enter your Home Assistant`s Long-lived token"),
            "Long-Lived Token belongs here ...",
            &snap.ha.token_draft,
            Message::HaTokenDraftChanged,
            Some(Message::SavePressed),
            p,
        ),
        TokenPresence::Present if snap.ha.auth_failed => {
            let title_element = components::error_message(
                "Token possibly invalid. HA rejected authorization with AuthFailed.",
                p,
            );
            let action_element = components::danger_button_with(
                Icon::Trash.text(p).size(text_size::LARGE).into(),
                p,
                Some(Message::HaTokenDelete),
            )
            .into();
            settings_components::item_with_element(title_element, action_element)
        }
        TokenPresence::Present => {
            let title_element = components::success_message("Token is stored in the key-chain", p);
            let action_element = components::danger_button_with(
                Icon::Trash.text(p).size(text_size::LARGE).into(),
                p,
                Some(Message::HaTokenDelete),
            )
            .into();
            settings_components::item_with_element(title_element, action_element)
        }
        TokenPresence::AccessFailed(..) => {
            let title_element = components::error_message("Access to key-chain was denied.", p);
            let action_element = components::pill_button_with(
                Icon::Refresh.text(p).size(text_size::LARGE),
                ButtonVisual::pill(p),
                Some(Message::RetryHaTokenPresence),
            )
            .into();
            settings_components::item_with_element(title_element, action_element)
        }
        TokenPresence::Checking | TokenPresence::Unchecked => {
            settings_components::item_with_icon_button(
                "Checking token",
                Some("Trying to access your OS key-chain"),
                Icon::Refresh,
                Message::RetryHaTokenPresence,
                p,
            )
        }
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
