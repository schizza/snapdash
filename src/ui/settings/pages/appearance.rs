use iced::Element;

use crate::app::{Message, Snapdash};
use crate::ui::components::settings_components;
use crate::widget_size::WidgetSize;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();

    settings_components::page_with_sections(
        "Appearance",
        [
            // Theme section
            settings_components::section(
                [settings_components::item_with_picker(
                    "Theme",
                    None,
                    snap.theme_options.clone(),
                    snap.theme,
                    Message::ThemeSelected,
                    p,
                )],
                p,
            ),
            // Widget section
            settings_components::section(
                [
                    settings_components::item_with_picker(
                        "Widget size",
                        Some("Affects new and currently opened widgets"),
                        WidgetSize::ALL.to_vec(),
                        snap.config.widget_size,
                        Message::WidgetSizeChanged,
                        p,
                    ),
                    settings_components::item_with_toggle(
                        "Adaptive font size",
                        Some("Scale font on long values so they fit on one line."),
                        snap.config.adaptive.adaptive_font,
                        Message::AdaptiveFontChanged,
                        p,
                    ),
                ],
                p,
            ),
        ],
        p,
    )
}
