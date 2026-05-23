use iced::widget::{MouseArea, button, container, mouse_area, row, space};

use iced::{Alignment, Element, Length};
use iced::{Background, Border, window};

use crate::app::{Message, WindowKind, WindowState};
use crate::theme::Palette;
use crate::ui::icon::Icon;
use crate::ui::theme::icon_button;
use crate::widget_size::Priority;

/// Returns window content based on its kind (`Settings` / `Entity`).
pub fn window_content<'a>(
    app: &'a crate::app::Snapdash,
    win: &'a WindowState,
    id: window::Id,
) -> Element<'a, Message> {
    match &win.kind {
        WindowKind::Settings => crate::ui::settings::view(app, id),
        WindowKind::Entity { entity_id } => {
            let priority = app
                .config
                .widget_priorities
                .get(entity_id)
                .copied()
                .unwrap_or_default();
            crate::ui::entity_window::view(
                &win.entity,
                app.theme.palette,
                app.ha.connected,
                app.update.is_available(),
                app.config.widget_settings,
                priority,
            )
        }
        WindowKind::ReleaseNotes => crate::ui::release_notes::view(app, id),
        WindowKind::ThemeGallery => crate::ui::gallery::view(app, id),
    }
}

/// Adds a "gear" overlay in the window corner that opens settings.
pub fn with_gear_overlay<'a>(
    app: &crate::app::Snapdash,
    inner: Element<'a, Message>,
    win: &WindowState,
) -> Element<'a, Message> {
    // Conditional rendering: when not hovered, gear is not part of the
    // widget tree at all. Avoids relying on alpha-0 transparency, which
    // doesn't actually hide the widget on Windows wgpu pipeline (state +
    // re-render are correct, alpha=0.0 reaches the renderer, but the
    // glyph still rasterizes visibly — likely a blend-mode quirk for
    // text-on-transparent-surface).

    if !win.entity.hovered {
        return inner;
    }

    let p = app.theme.palette;

    let gear_button = iced::widget::button(Icon::Gear.text(p))
        .padding(0)
        .on_press(Message::OpenSettings)
        .style(icon_button(p, 1.0));

    let gear_layer: Element<Message> = iced::widget::container(gear_button)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .align_y(Alignment::End)
        .padding(10)
        .into();

    let current = app
        .config
        .widget_priorities
        .get(&win.entity.entity_id)
        .copied()
        .unwrap_or_default();

    let priority_layer: Element<Message> =
        iced::widget::container(priority_selector(&win.entity.entity_id, current, p))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Start)
            .align_y(Alignment::End)
            .padding(10)
            .into();
    iced::widget::stack![inner, priority_layer, gear_layer].into()
}

fn priority_selector<'a>(entity_id: &str, current: Priority, p: Palette) -> Element<'a, Message> {
    let dots = Priority::ALL.iter().fold(row![].spacing(6), |acc, &level| {
        acc.push(priority_dot(level, current, entity_id.to_owned(), p))
    });

    container(dots)
        .padding([4, 8])
        .style(move |_| container::Style {
            background: Some(Background::Color(p.card_2)),
            border: Border {
                radius: 999.0.into(),
                width: 1.0,
                color: p.border,
            },
            ..Default::default()
        })
        .into()
}

fn priority_dot<'a>(
    level: Priority,
    current: Priority,
    entity_id: String,
    p: Palette,
) -> Element<'a, Message> {
    let active = level == current;
    // Dot grows with the level so the control reads as low → high.
    let size = match level {
        Priority::Low => 6.0,
        Priority::Normal => 8.0,
        Priority::High => 10.0,
    };
    let fill = if active { p.accent } else { p.text_disabled };

    let dot = container(space())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_| container::Style {
            background: Some(Background::Color(fill)),
            border: Border {
                radius: 999.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    button(container(dot).center(Length::Fixed(18.0)))
        .padding(0)
        .on_press(Message::WidgetPriorityChanged(entity_id, level))
        .style(|_, _| iced::widget::button::Style {
            background: None,
            ..Default::default()
        })
        .into()
}

/// Wraps `MouseArea` with hover effect and added drag.
pub fn with_mouse_area<'a>(
    content: Element<'a, Message>,
    id: window::Id,
    win: &WindowState,
) -> Element<'a, Message> {
    let ma: MouseArea<Message> = mouse_area(content)
        .on_enter(Message::EntityHover {
            window: id,
            on: true,
        })
        .on_exit(Message::EntityHover {
            window: id,
            on: false,
        });

    match win.kind {
        WindowKind::Entity { .. } => ma.on_press(Message::StartDrag(id)).into(),
        WindowKind::Settings => ma.into(),
        WindowKind::ReleaseNotes => ma.into(),
        WindowKind::ThemeGallery => ma.into(),
    }
}
