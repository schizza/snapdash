use crate::theme::{Palette, metric, text_size};
use crate::ui::components::button::ButtonType;
use crate::ui::components::{self};
use iced::Length;
use iced::widget::{column, row};
use iced::{Element, widget::text::IntoFragment};

use crate::app::Message;

pub fn section<'a, I>(items: I, p: Palette) -> Element<'a, Message>
where
    I: IntoIterator<Item = Element<'a, Message>>,
{
    let body = items
        .into_iter()
        .fold(column![].spacing(metric::GAP), |col, item| col.push(item));

    components::subcard(body.into(), p)
}

pub fn page<'a, I>(title: impl IntoFragment<'a>, items: I, p: Palette) -> Element<'a, Message>
where
    I: IntoIterator<Item = Element<'a, Message>>,
{
    column![components::title(title, p), section(items, p)]
        .spacing(metric::GAP)
        .width(Length::Fill)
        .into()
}

pub fn page_with_sections<'a, I>(
    title: impl IntoFragment<'a>,
    items: I,
    scrollable: bool,
    p: Palette,
) -> Element<'a, Message>
where
    I: IntoIterator<Item = Element<'a, Message>>,
{
    let outer = column![components::title(title, p)]
        .spacing(metric::GAP)
        .width(Length::Fill);

    let items = items
        .into_iter()
        .fold(outer, |col, item| col.push(item))
        .into();

    if scrollable {
        components::scrollable(items, p).into()
    } else {
        items
    }
}

/// Make settings page with scrollable content and non-scrollable bottom.
///
/// Scrollable items will be on the top of page
/// Non scrollabele (fixed) items will be at the bottom page.
///
/// You should consider make non scrollable only buttons row or just
/// few items of any of item_with_*
pub fn page_with_scrollable_sections<'a, S, F>(
    title: impl IntoFragment<'a>,
    scrollable_items: S,
    fixed_items: F,
    p: Palette,
) -> Element<'a, Message>
where
    S: IntoIterator<Item = Element<'a, Message>>,
    F: IntoIterator<Item = Element<'a, Message>>,
{
    // prepare scrollable items as Element
    let scrollable_col = column![]
        .spacing(metric::GAP)
        .width(Length::Fill)
        .height(Length::Fill);
    let mut scrollable_items: Element<'a, Message> = scrollable_items
        .into_iter()
        .fold(scrollable_col, |col, item| col.push(item))
        .into();
    scrollable_items = components::scrollable(scrollable_items, p)
        .height(Length::Fill)
        .into();

    // prepare non scrolable items as Element
    let non_scrollable_col = column![].width(Length::Fill).spacing(metric::GAP);
    let non_scrollable_items: Element<'a, Message> = fixed_items
        .into_iter()
        .fold(non_scrollable_col, |col, item| col.push(item))
        .into();

    // page generation
    column![
        components::title(title, p),
        scrollable_items,
        non_scrollable_items,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(metric::GAP)
    .into()
}

/// One row in a settings card: left side a label and optional
/// helper text, right side carry action widget.
fn item<'a>(
    label: impl IntoFragment<'a>,
    helper: Option<impl IntoFragment<'a>>,
    action: Element<'a, Message>,
    p: Palette,
) -> Element<'a, Message> {
    let mut col = column![components::body(label, p)]
        .width(iced::Length::Fill)
        .spacing(2);

    if let Some(h) = helper {
        col = col.push(components::helper(h, p));
    }

    row![col, action]
        .align_y(iced::Alignment::Center)
        .spacing(metric::GAP)
        .into()
}

pub fn item_with_button<'a>(
    label: impl IntoFragment<'a>,
    helper: Option<&'a str>,
    button_label: impl IntoFragment<'a>,
    on_click: Option<Message>,
    button_type: ButtonType,
    p: Palette,
) -> Element<'a, Message> {
    let button = match button_type {
        ButtonType::PILL => components::pill_button(button_label, p, on_click),
        ButtonType::PRIMARY => components::primary_button(button_label, p, on_click),
        ButtonType::DANGER => components::danger_button(button_label, p, on_click),
    };
    item(label, helper, button.into(), p)
}

pub fn item_with_toggle<'a, F>(
    label: impl IntoFragment<'a>,
    helper: Option<&'a str>,
    is_checked: bool,
    on_toggle: F,
    p: Palette,
) -> Element<'a, Message>
where
    F: Fn(bool) -> Message + 'a,
{
    item(
        label,
        helper,
        components::toggler(is_checked, on_toggle, p).into(),
        p,
    )
}

pub fn item_with_picker<'a, V>(
    label: impl IntoFragment<'a>,
    helper: Option<&'a str>,
    options: Vec<V>,
    selected: V,
    on_select: impl Fn(V) -> Message + 'a,
    p: Palette,
) -> Element<'a, Message>
where
    V: ToString + PartialEq + Clone + 'a,
{
    item(
        label,
        helper,
        components::picker(options, selected, on_select, p).into(),
        p,
    )
}

pub fn item_with_status<'a>(
    label: impl IntoFragment<'a>,
    helper: Option<&'a str>,
    status_text: impl IntoFragment<'a>,
    p: Palette,
) -> Element<'a, Message> {
    item(label, helper, components::body(status_text, p).into(), p)
}

pub fn item_with_input<'a>(
    label: impl IntoFragment<'a>,
    helper: Option<&'a str>,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> Message + 'a,
    on_submit: Option<Message>,
    p: Palette,
) -> Element<'a, Message> {
    let mut action = components::mac_input(placeholder, value, p)
        .on_input(on_change)
        .size(text_size::NORMAL);

    if let Some(a) = on_submit {
        action = action.on_submit(a)
    }
    item(label, helper, action.into(), p)
}

pub fn item_with_icon_button<'a>(
    label: impl IntoFragment<'a>,
    helper: Option<impl IntoFragment<'a>>,
    icon: crate::ui::icon::Icon,
    on_click: Message,
    p: Palette,
) -> Element<'a, Message> {
    let action: Element<'a, Message> =
        iced::widget::mouse_area(icon.text(p).size(14).color(p.text_dim))
            .on_press(on_click)
            .interaction(iced::mouse::Interaction::Pointer)
            .into();

    item(label, helper, action, p)
}

/// Function will create settings item from any Element as left side
/// and action from Element on right side.
/// This is generic Element / Element action scope.
/// Please use only if necessary.
pub fn item_with_element<'a>(
    title_element: Element<'a, Message>,
    action_element: Element<'a, Message>,
) -> Element<'a, Message> {
    let col = column![title_element].width(iced::Length::Fill).spacing(2);

    row![col, action_element]
        .align_y(iced::Alignment::Center)
        .spacing(metric::GAP)
        .into()
}
