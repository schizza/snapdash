use iced::Element;

use crate::app::{Message, Snapdash};
use crate::ha::token::TokenPresence;
use crate::theme::text_size;
use crate::ui::components::{self, ButtonVisual, settings_components};
use crate::ui::icon::Icon;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette;

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

    // TLS trust for https:// URLs (#102). A home CA is trusted via its
    // bundle; the danger toggle is for certificates no store can fix,
    // and owns a permanent warning rather than a transient status line.
    let ca_item = match &snap.config.tls_ca_file {
        None => settings_components::item_with_badge_button(
            "Custom CA certificate",
            Some(
                "Trust your own certificate authority (PEM) in addition \
                 to the built-in roots - for HTTPS with a home CA.",
            ),
            "Choose file ...",
            None,
            Some(Message::HaCaFilePick),
            p,
        ),
        Some(path) => {
            let title_element =
                components::success_message(format!("Trusting CA: {}", path.display()), p);
            let action_element = components::danger_button_with(
                Icon::Trash.text(p).size(text_size::LARGE).into(),
                p,
                Some(Message::HaCaFileClear),
            )
            .into();
            settings_components::item_with_element(title_element, action_element)
        }
    };

    let insecure_item = settings_components::item_with_toggle(
        "Disable certificate verification",
        Some(
            "Connect even when the certificate cannot be validated, \
             e.g. a self-signed certificate created with CA:TRUE.",
        ),
        snap.config.tls_accept_invalid_certs,
        Message::HaInsecureTlsChanged,
        p,
    );

    let mut items = vec![
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
        ca_item,
        insecure_item,
    ];

    if snap.config.tls_accept_invalid_certs {
        items.push(components::error_message(
            "Danger: Snapdash now trusts whoever answers at this URL. The \
             connection is encrypted but not authenticated - anyone on your \
             network could impersonate Home Assistant and capture your \
             token. Prefer a proper certificate, or a custom CA, whenever \
             you can.",
            p,
        ));
    }

    settings_components::page("Connection", items, p)
}
