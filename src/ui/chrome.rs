use iced::widget::{MouseArea, mouse_area};
use iced::window;
use iced::{Alignment, Element, Length};

use crate::app::{Message, WindowKind, WindowState};
use crate::ui::theme::{UiTheme, icon_button, icon_text};

/// Vrátí obsah okna podle typu (`Settings` / `Entity`)
pub fn window_content<'a>(
    app: &'a crate::app::Snapdash,
    win: &'a WindowState,
) -> Element<'a, Message> {
    match &win.kind {
        WindowKind::Settings => crate::ui::settings::view(app),
        WindowKind::Entity { .. } => {
            crate::ui::entity_window::view(&win.entity, app.theme.palette(), app.ha_connected)
        }
    }
}

/// Přidá do rohu okna "gear" overlay, který otevírá nastavení.
pub fn with_gear_overlay<'a>(
    app: &crate::app::Snapdash,
    inner: Element<'a, Message>,
    win: &WindowState,
) -> Element<'a, Message> {
    let ui_theme = UiTheme::from(&app.theme);
    let p = app.theme.palette();
    let a = if win.entity.hovered { 1.0 } else { 0.0 };

    let gear_icon = iced::widget::text("⚙")
        .size(28)
        .style(icon_text(ui_theme, a));

    let gear_button = iced::widget::button(gear_icon)
        .padding(8)
        .on_press_maybe(if a > 0.0 {
            Some(Message::OpenSettings)
        } else {
            None
        })
        .style(icon_button(ui_theme, a));

    let gear_layer: Element<Message> = iced::widget::container(gear_button)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .align_y(Alignment::End)
        .padding(10)
        .into();

    iced::widget::stack![inner, gear_layer].into()
}

/// Obalí obsah do `MouseArea` s hover efektem a případným dragem.
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
        })
        .into();

    match win.kind {
        WindowKind::Entity { .. } => ma.on_press(Message::StartDrag(id)).into(),
        WindowKind::Settings => ma.into(),
    }
}
