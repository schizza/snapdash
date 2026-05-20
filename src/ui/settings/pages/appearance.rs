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
                        snap.config.widget_settings.widget_size,
                        Message::WidgetSizeChanged,
                        p,
                    ),
                    settings_components::item_with_toggle(
                        "Adaptive font size",
                        Some("Scale font on long values so they fit on one line."),
                        snap.config.widget_settings.adaptive.adaptive_font,
                        Message::AdaptiveFontChanged,
                        p,
                    ),
                    settings_components::item_with_toggle(
                        "Smart number formatting",
                        Some("Compress large values - 1234567 W -> 1.23 MW"),
                        snap.config.widget_settings.adaptive.adaptive_value,
                        Message::AdaptiveValueChanged,
                        p,
                    ),
                    settings_components::item_with_toggle(
                        "Show status bar",
                        None,
                        snap.config.widget_settings.show_measurement_info,
                        Message::ShowMeasurementInfoChanged,
                        p,
                    ),
                ],
                p,
            ),
        ],
        false,
        p,
    )
}
