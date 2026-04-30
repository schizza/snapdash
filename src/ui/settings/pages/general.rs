use iced::widget::{column, row};
use iced::{Element, Length};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;
use crate::ui::icon::Icon;
use crate::ui::theme::UiTheme;
use crate::update;

pub fn view<'a>(snap: &'a Snapdash) -> Element<'a, Message> {
    let p = snap.theme.palette();
    let ui_theme = UiTheme::from(&snap.theme);

    // Hero
    let hero = column![
        iced::widget::text("Snapdash")
            .size(28)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.text_primary),
            }),
        iced::widget::text(format!("Version {}", update::CURRENT_VERSION))
            .size(13)
            .style(move |_: &iced::Theme| iced::widget::text::Style {
                color: Some(p.text_secondary),
            }),
    ]
    .spacing(4);

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

    let ha_row = stat_row("Home Assistant", ha_status_text, ha_color, p);
    let sensors_row = stat_row(
        "Selected senors",
        format!("{}", snap.selected_widgets.len()),
        p.text_body,
        p,
    );
    let action_config = action_link(
        "Edit configuration",
        "config.json",
        Icon::Gear,
        Message::OpenConfigFile,
        ui_theme,
        p,
    );
    let action_logs = action_link(
        "Open log directory",
        "Browse runtime logs",
        Icon::Download,
        Message::OpenLogFile,
        ui_theme,
        p,
    );

    let stats = components::subcard(column![ha_row, sensors_row].spacing(metric::GAP).into(), p);

    let actions = components::subcard(
        column![action_config, action_logs]
            .spacing(metric::GAP)
            .into(),
        p,
    );

    column![
        components::title("General", p),
        hero,
        iced::widget::space().height(metric::PAD),
        stats,
        actions,
    ]
    .spacing(metric::PAD)
    .width(Length::Fill)
    .into()
}

fn stat_row(
    label: &'static str,
    value: String,
    value_color: iced::Color,
    p: crate::theme::Palette,
) -> Element<'static, Message> {
    row![
        iced::widget::text(label)
            .size(13)
            .style(move |_: &iced::Theme| {
                iced::widget::text::Style {
                    color: Some(p.text_dim),
                }
            }),
        iced::widget::space().width(Length::Fill),
        iced::widget::text(value)
            .size(11)
            .style(move |_: &iced::Theme| {
                iced::widget::text::Style {
                    color: Some(value_color),
                }
            }),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}

fn action_link<'a>(
    label: &'static str,
    helper: &'static str,
    icon: Icon,
    msg: Message,
    ui_theme: UiTheme,
    p: crate::theme::Palette,
) -> Element<'a, Message> {
    row![
        column![
            iced::widget::text(label)
                .size(13)
                .style(move |_: &iced::Theme| {
                    iced::widget::text::Style {
                        color: Some(p.text_body),
                    }
                }),
            iced::widget::text(helper)
                .size(11)
                .style(move |_: &iced::Theme| {
                    iced::widget::text::Style {
                        color: Some(p.text_dim),
                    }
                }),
        ]
        .spacing(2)
        .width(Length::Fill),
        iced::widget::mouse_area(icon.text(ui_theme).size(14).color(p.text_dim),)
            .on_press(msg)
            .interaction(iced::mouse::Interaction::Pointer)
    ]
    .align_y(iced::Alignment::Center)
    .spacing(metric::GAP)
    .into()
}
