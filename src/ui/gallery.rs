use iced::widget::{column, container, mouse_area, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, window};

use crate::app::{GalleryState, Message, Snapdash};
use crate::theme::{Palette, ThemeDef, metric, text_size};
use crate::ui::components::{self, title};
use crate::ui::icon::Icon;

pub fn view<'a>(snap: &'a Snapdash, id: window::Id) -> Element<'a, Message> {
    let p = snap.theme.palette;

    let title = title("Theme gallery", p);

    let title_bar: Element<Message> = row![
        mouse_area(container(title).width(Length::Fill).padding([4, 0]))
            .on_press(Message::StartDrag(id)),
        components::pill_button_with(
            Icon::Close.text(p).size(text_size::NORMAL),
            components::ButtonVisual::pill(p),
            Some(Message::CloseWindow(id)),
        ),
    ]
    .align_y(Alignment::Center)
    .into();

    let mut col = column![title_bar].spacing(metric::GAP);

    // Last install result, if any.
    match &snap.gallery_status {
        Some(Ok(msg)) => col = col.push(components::success_message(msg.clone(), p)),
        Some(Err(e)) => col = col.push(components::error_message(e.clone(), p)),
        None => {}
    }

    let body: Element<Message> = match &snap.gallery {
        GalleryState::Idle | GalleryState::Loading => {
            container(components::dimmed("Loading themes…", p))
                .center(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        GalleryState::Failed(e) => container(
            column![
                components::error_message(format!("Couldn't load themes: {e}"), p),
                components::pill_button("Retry", p, Some(Message::OpenThemeGallery)),
            ]
            .spacing(metric::GAP)
            .align_x(Alignment::Center),
        )
        .center(Length::Fill)
        .height(Length::Fill)
        .into(),
        GalleryState::Loaded(themes) if themes.is_empty() => {
            container(components::dimmed("No themes published yet.", p))
                .center(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        GalleryState::Loaded(themes) => {
            let rows = themes.iter().fold(
                column![].spacing(metric::GAP).width(Length::Fill),
                |acc, theme| acc.push(theme_row(theme, snap, p)),
            );

            components::scrollable(rows.into(), p)
                .height(Length::Fill)
                .into()
        }
    };

    col = col.push(body);

    components::card(col.into(), p)
}

/// One gallery entry: swatch preview from the theme's own palette, name +
/// author + appearance, and an Install/Reinstall button.
fn theme_row<'a>(theme: &'a ThemeDef, snap: &Snapdash, p: Palette) -> Element<'a, Message> {
    let installed = snap.available_themes.iter().any(|t| t.name == theme.name);

    let name = text(theme.name.clone())
        .size(text_size::NORMAL)
        .style(move |_| text::Style {
            color: Some(p.text_primary),
        });

    let meta_text = match &theme.author {
        Some(a) => format!("by {a} · {:?}", theme.appearance),
        None => format!("{:?}", theme.appearance),
    };

    let info = column![name, components::dimmed(meta_text, p)].spacing(2);

    let (label, icon) = if installed {
        ("Reinstall", Some(Icon::Refresh))
    } else {
        ("Install", Some(Icon::Download))
    };

    let action = components::badge_button(
        label,
        icon,
        Some(Message::InstallGalleryTheme(theme.clone())),
        p,
    );

    let header = row![
        container(info).width(Length::Fill),
        container(action).align_y(Alignment::Center),
    ]
    .align_y(Alignment::Center)
    .spacing(metric::GAP);

    let body = column![header, swatches(theme.palette)].spacing(metric::GAP);

    components::subcard(body.into(), p)
}

/// A strip of color chips from the theme's palette so the user sees the
/// look before installing.
fn swatches<'a>(palette: Palette) -> Element<'a, Message> {
    let chips = [
        palette.bg,
        palette.card,
        palette.accent,
        palette.text_primary,
        palette.text_body,
        palette.success,
        palette.danger,
    ];

    chips
        .into_iter()
        .fold(row![].spacing(6), |acc, c| acc.push(chip(c)))
        .into()
}

fn chip<'a>(color: Color) -> Element<'a, Message> {
    container(text(""))
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(20.0))
        .style(move |_| container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                // Hairline so near-bg chips stay visible on the subcard.
                color: Color {
                    a: 0.15,
                    ..Color::BLACK
                },
            },
            ..Default::default()
        })
        .into()
}
