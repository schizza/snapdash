use iced::Element;

use crate::app::{Message, Snapdash};
use crate::theme::ThemeDef;
use crate::ui::components::{self, settings_components};
use crate::widget_size::WidgetSize;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette;

    let mut theme_items = vec![
        settings_components::item_with_picker(
            "Theme",
            None,
            snap.available_themes.clone(),
            snap.theme.clone(),
            |theme: ThemeDef| Message::ThemeSelected(theme.name),
            p,
        ),
        settings_components::item_with_badge_button(
            "Theme gallery",
            Some("Browse and install themes published online."),
            "Browse",
            Some(crate::ui::icon::Icon::Download),
            Some(Message::OpenThemeGallery),
            p,
        ),
        settings_components::item_with_badge_button(
            "Import theme",
            Some("Add a theme from a JSON file."),
            "Import",
            Some(crate::ui::icon::Icon::FolderOpen),
            Some(Message::ImportTheme),
            p,
        ),
    ];
    match &snap.theme_import_status {
        Some(Ok(msg)) => theme_items.push(components::success_message(msg.clone(), p)),
        Some(Err(e)) => theme_items.push(components::error_message(e.clone(), p)),
        None => {}
    }

    let theme_section = settings_components::section(theme_items, p);

    settings_components::page_with_sections(
        "Appearance",
        [
            // Theme section
            theme_section,
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
