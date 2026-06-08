use iced::widget::{button, checkbox, column, container, row, space, text};
use iced::{Alignment, Background, Border, Element, Length, Padding};

use crate::app::Message;
use crate::theme::{Palette, metric, text_size};
use crate::ui::icon::Icon;
use crate::ui::theme::icon_button;

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
        let badge = if app.config.widget_visibility.contains_key(&sensor.entity_id)
            && !app.is_widget_visible(&sensor.entity_id)
        {
            Some(hidden_badge(p))
        } else {
            None
        };

        col = col.push(sensor_row(
            sensor.entity_id.as_str(),
            sensor.friendly_name.as_str(),
            true,
            metric::PAD.into(),
            badge,
            Some(Message::OpenWidgetSettings(sensor.entity_id.clone())),
            p,
        ));
    }

    crate::ui::components::scrollable(col.into(), p)
        .height(Length::Fill)
        .into()
}

pub fn sensors_section(app: &crate::app::Snapdash, p: Palette) -> Element<'_, Message> {
    let query = app.entity_search_query.trim().to_lowercase();

    let sensors = app
        .settings_sensors
        .iter()
        .filter(|sensor| query.is_empty() || sensor.search_key.contains(&query));

    let mut col = column![];

    for sensor in sensors {
        col = col.push(sensor_row(
            sensor.entity_id.as_str(),
            sensor.friendly_name.as_str(),
            app.selected_widgets.contains(&sensor.entity_id),
            8.0.into(),
            None,
            None,
            p,
        ));
    }

    crate::ui::components::scrollable(col.into(), p)
        .height(Length::Fill)
        .into()
}

fn sensor_row<'a>(
    entity_id: &'a str,
    friendly_name: &'a str,
    selected: bool,
    padding: Padding,
    badge: Option<Element<'a, Message>>,
    on_configure: Option<Message>,
    p: Palette,
) -> Element<'a, Message> {
    let id_for_msg = entity_id.to_owned();

    let cb = checkbox(selected)
        .label(friendly_name)
        .on_toggle(move |_| Message::ToggleWidget(id_for_msg.clone()))
        .text_size(text_size::NORMAL)
        .style(move |_, _| iced::widget::checkbox::Style {
            background: iced::Background::Color(p.bg),
            text_color: Some(p.text_primary),
            icon_color: p.accent,
            border: Border {
                color: p.text_primary,
                width: 0.5,
                radius: 15.0.into(),
            },
        });
    let top_row: Element<'a, Message> = if let Some(msg) = on_configure {
        let configure_btn = button(Icon::Sliders.text(p).size(13.0))
            .padding(2)
            .on_press(msg)
            .style(icon_button(p, 0.5));
        row![cb, space().width(Length::Fill), configure_btn,]
            .align_y(Alignment::Start)
            .spacing(8)
            .into()
    } else {
        cb.into()
    };

    let id_text: Element<'a, Message> = text(format!(" ({entity_id})"))
        .size(text_size::XSMALL)
        .width(Length::Fill)
        .wrapping(text::Wrapping::WordOrGlyph)
        .style(move |_: &iced::Theme| iced::widget::text::Style {
            color: Some(p.text_dim),
        })
        .align_x(Alignment::Start)
        .into();

    let mut id_row = row![id_text].spacing(6).align_y(Alignment::Start);

    if let Some(b) = badge {
        id_row = id_row.push(b);
    }

    let id_row = id_row.padding(Padding {
        top: 0.0,
        right: 6.0,
        bottom: 0.0,
        left: 20.0,
    });

    column![top_row, id_row].padding(padding).into()
}

fn hidden_badge<'a>(p: Palette) -> Element<'a, Message> {
    container(
        text("Hidden by rule")
            .size(text_size::XSMALL)
            .wrapping(text::Wrapping::None)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.danger),
            }),
    )
    .padding([2, 8])
    .style(move |_| iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color {
            a: 0.12,
            ..p.danger
        })),
        border: iced::Border {
            radius: 999.0.into(),
            width: 1.0,
            color: iced::Color { a: 0.4, ..p.danger },
        },
        ..Default::default()
    })
    .into()
}
