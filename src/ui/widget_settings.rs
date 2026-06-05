use iced::widget::{column, container, mouse_area, row, space};
use iced::{Alignment, Element, Length, window};

use crate::app::{Message, Snapdash};
use crate::theme::{metric, text_size};
use crate::ui::components::{self, settings_components};
use crate::ui::icon::Icon;
use crate::widget_size::Priority;

pub fn view<'a>(snap: &'a Snapdash, id: window::Id, entity_id: &'a str) -> Element<'a, Message> {
    let p = snap.theme.palette;

    // Page title reflects whats on the widget, dialog title updates live
    let display_name = snap.display_name(entity_id);

    // HA's friendly_name serves as the placeholder so the user sees what
    // the title will be if override is cleared
    let friendly_placeholder = snap
        .ha
        .entities
        .get(entity_id)
        .and_then(|s| s.attributes.get("friendly_name").and_then(|v| v.as_str()))
        .unwrap_or(entity_id);

    let current_override = snap
        .config
        .widget_names
        .get(entity_id)
        .map(String::as_str)
        .unwrap_or("");

    let display_section = settings_components::section(
        [settings_components::item_with_input(
            "Custom name",
            Some("Override the title shown on the widget. Empty resets to HA's name."),
            friendly_placeholder,
            current_override,
            {
                let entity_id = entity_id.to_owned();
                move |val: String| Message::WidgetNameChanged(entity_id.clone(), val)
            },
            None, // no submit action - chnges are live
            p,
        )],
        p,
    );

    // Behavior section
    let current_priority = snap
        .config
        .widget_priorities
        .get(entity_id)
        .copied()
        .unwrap_or_default();

    let priority_item = settings_components::item_with_picker(
        "Priority",
        Some("Low → dimmed value · High → accent value + steady ring"),
        Priority::ALL.to_vec(),
        current_priority,
        {
            let entity_id = entity_id.to_owned();
            move |new: Priority| Message::WidgetPriorityChanged(entity_id.clone(), new)
        },
        p,
    );

    let behavior_section = settings_components::section([priority_item], p);

    let page = settings_components::page_with_sections(
        display_name,
        [display_section, behavior_section],
        false,
        p,
    );

    let body: Element<Message> = container(page)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(metric::PAD)
        .into();

    let outer = column![title_bar(snap, id), body, footer(p, entity_id)].spacing(metric::GAP);

    components::card(outer.into(), p)
}

/// Persistent top chrome — drag area + close. Mirrors
/// `settings::chrome::title_bar`.
fn title_bar<'a>(snap: &'a Snapdash, id: window::Id) -> Element<'a, Message> {
    let p = snap.theme.palette;
    row![
        mouse_area(
            container(components::title("Widget settings", p))
                .width(Length::Fill)
                .padding([4, 0])
        )
        .on_press(Message::StartDrag(id)),
        components::pill_button_with(
            Icon::Close.text(p).size(text_size::NORMAL),
            components::ButtonVisual::pill(p),
            Some(Message::CloseWindow(id)),
        ),
    ]
    .spacing(metric::GAP)
    .align_y(Alignment::Center)
    .into()
}

/// Footer — currently a dimmed entity_id hint so the user can see the
/// raw HA identifier for whatever they're tweaking. Shape mirrors
/// `settings::chrome::footer` (row, right-aligned info on the end).
fn footer<'a>(p: crate::theme::Palette, entity_id: &'a str) -> Element<'a, Message> {
    row![
        space().width(Length::Fill),
        components::dimmed(format!("entity: {entity_id}"), p),
    ]
    .align_y(Alignment::Center)
    .into()
}
