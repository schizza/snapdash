use iced::widget::{checkbox, column, container, row, text};
use iced::{Background, Border, Element, Length};

use crate::app::Message;
use crate::theme::{Palette, metric};

pub fn status_dot(p: Palette, ok: bool) -> Element<'static, Message> {
    let color = if ok { p.success } else { p.danger };

    container(
        iced::widget::space()
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0)),
    )
    .style(move |_theme| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: p.border,
        },
        ..Default::default()
    })
    .into()
}

pub fn active_sensor_section(app: &crate::app::Snapdash, p: Palette) -> Element<'_, Message> {
    let active_sensors = app.active_settings_sensors.iter();

    let mut col = column![];
    for sensor in active_sensors {
        let id = sensor.entity_id.clone();
        let friendly_name = sensor.friendly_name.clone();
        let id_for_msg = id.clone();
        let row_el = row![
            checkbox(true)
                .label(friendly_name)
                .on_toggle(move |_| Message::ToggleWidget(id_for_msg.clone()))
                .text_size(14)
                .style(move |_, _| iced::widget::checkbox::Style {
                    background: iced::Background::Color(p.bg),
                    text_color: Some(p.text_primary),
                    icon_color: p.accent,
                    border: Border {
                        color: p.text_primary,
                        width: 0.5,
                        radius: 15.0.into()
                    }
                }),
            text(format!(" ({})", sensor.entity_id))
                .size(14)
                .style(move |_: &iced::Theme| iced::widget::text::Style {
                    color: Some(p.text_dim)
                }),
        ]
        .padding(metric::PAD);
        col = col.push(row_el);
    }

    crate::ui::components::scrollable(col.into(), p)
        .height(Length::Fill)
        .into()
}

pub fn sensors_section(app: &crate::app::Snapdash, p: Palette) -> Element<'_, Message> {
    // Seznam všech entit
    // Např. jen senzory:
    // let sensors = entities
    //     .iter()
    //     .filter(|e| e.entity_id.starts_with("sensor."));

    let query = app.entity_search_query.trim().to_lowercase();

    let sensors = app
        .settings_sensors
        .iter()
        .filter(|sensor| query.is_empty() || sensor.search_key.contains(&query));

    let mut col = column![];

    for sensor in sensors {
        let id = sensor.entity_id.clone();
        let friendly = sensor.friendly_name.clone();
        let selected = app.selected_widgets.contains(&id);

        let id_for_msg = id.clone();

        let row_el = row![
            checkbox(selected)
                .label(friendly)
                .on_toggle(move |_| Message::ToggleWidget(id_for_msg.clone()))
                .text_size(14)
                .style(move |_, _| iced::widget::checkbox::Style {
                    background: iced::Background::Color(p.bg),
                    text_color: Some(p.text_primary),
                    icon_color: p.accent,
                    border: Border {
                        color: p.text_primary,
                        width: 0.5,
                        radius: 15.0.into()
                    }
                }),
            text(format!(" ({})", sensor.entity_id))
                .size(14)
                .style(move |_: &iced::Theme| iced::widget::text::Style {
                    color: Some(p.text_dim)
                }),
        ]
        .padding(8);
        col = col.push(row_el);
    }

    crate::ui::components::scrollable(col.into(), p)
        .height(Length::Fill)
        .into()
}
