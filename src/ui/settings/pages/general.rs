use iced::Element;

use crate::app::{Message, Snapdash};
use crate::ui::components::settings_components::{item_with_status, page_with_sections};
use crate::ui::components::{self, inline_links, settings_components};
use crate::ui::icon::Icon;
use crate::{DOCS_URL, HOMEPAGE_URL, ISSUES_URL};

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette;

    let in_line_links = inline_links(
        [
            ("Homepage", HOMEPAGE_URL),
            ("Docs", DOCS_URL),
            ("Issues", ISSUES_URL),
        ],
        p,
    );
    let in_line_links = settings_components::section([in_line_links], p);
    // Stats grid
    let ha_status_text = if snap.ha.connected {
        format!("Connected to {}", snap.config.ha_url)
    } else if snap.config.ha_url.trim().is_empty() {
        "No HA URL configured".to_string()
    } else {
        format!("Disconnected ({})", snap.config.ha_url)
    };

    let ha_color = if snap.ha.connected {
        p.success
    } else {
        p.text_dim
    };

    let ha_row = settings_components::item_with_element(
        components::body("Home Assistant", p).into(),
        iced::widget::text(ha_status_text)
            .size(13)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(ha_color),
            })
            .into(),
    );

    let sensors_row = item_with_status(
        "Selected sensors",
        None,
        format!("{}", snap.selected_widgets.len()),
        p,
    );
    let stats = settings_components::section([ha_row, sensors_row], p);

    // Behavior section
    let mut behav_items: Vec<Element<Message>> = vec![settings_components::item_with_toggle(
        "Start at login",
        Some("Launch Snapdash automatically when you log into your computer."),
        snap.config.autostart,
        Message::AutostartChanged,
        p,
    )];

    if cfg!(target_os = "macos") && snap.config.autostart {
        behav_items.push(
            components::helper(
                "macOS may ask you to allow Snapdash in System Settings → \
                         Login Items the first time. After approval, autostart \
                         persists across reboots.",
                p,
            )
            .into(),
        );
    }

    let behav = settings_components::section(behav_items, p);

    let actions = settings_components::section(
        [
            settings_components::item_with_icon_button(
                "Edit configuration",
                Some("Open config.json in your default editor."),
                Icon::Gear,
                Message::OpenConfigFile,
                p,
            ),
            settings_components::item_with_icon_button(
                "Open log directory",
                Some("Browse runtime logs in your file manager."),
                Icon::Download,
                Message::OpenLogFile,
                p,
            ),
        ],
        p,
    );

    page_with_sections(
        "General",
        [in_line_links, stats.into(), behav, actions],
        false,
        p,
    )
}
