use iced::widget::{MouseArea, mouse_area};

use iced::window;
use iced::{Alignment, Element, Length};

use crate::app::{Message, UpdateState, WindowKind, WindowState};
use crate::ui::theme::{UiTheme, icon_button, icon_text};

/// Returns window content based on its kind (`Settings` / `Entity`).
pub fn window_content<'a>(
    app: &'a crate::app::Snapdash,
    win: &'a WindowState,
    id: window::Id,
) -> Element<'a, Message> {
    match &win.kind {
        WindowKind::Settings => crate::ui::settings::view(app, id),
        WindowKind::Entity { .. } => crate::ui::entity_window::view(
            &win.entity,
            app.theme.palette(),
            app.ha_connected,
            app.update_state == UpdateState::UpdateAvailable,
        ),
        WindowKind::ReleaseNotes => crate::ui::release_notes::view(app, id),
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

    let ui_theme = UiTheme::from(&app.theme);

    let gear_icon = iced::widget::text("⚙")
        .size(28)
        .style(icon_text(ui_theme, 1.0));

    let gear_button = iced::widget::button(gear_icon)
        .padding(8)
        .on_press(Message::OpenSettings)
        .style(icon_button(ui_theme, 1.0));

    let gear_layer: Element<Message> = iced::widget::container(gear_button)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .align_y(Alignment::End)
        .padding(10)
        .into();

    iced::widget::stack![inner, gear_layer].into()
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
    }
}
