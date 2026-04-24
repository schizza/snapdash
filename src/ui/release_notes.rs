use iced::widget::{column, container, markdown, mouse_area, row, text};
use iced::{Alignment, Element, Length, window};

use crate::app::{Message, Snapdash};
use crate::theme::metric;
use crate::ui::components;

pub fn view<'a>(snap: &'a Snapdash, id: window::Id) -> Element<'a, Message> {
    let p = snap.theme.palette();

    let Some(release) = &snap.latest_release else {
        return components::card(
            container(components::dimmed("No update available", p))
                .center(Length::Fill)
                .into(),
            p,
        );
    };

    let title_widget = text(format!("Latest version is {}", release.tag_name))
        .size(18)
        .style(move |_| text::Style {
            color: Some(p.text_primary),
        });

    let title_bar: Element<Message> = row![
        mouse_area(container(title_widget).width(Length::Fill).padding([4, 0]))
            .on_press(Message::StartDrag(id)),
        components::icon("✕", p, Some(Message::CloseWindow(id))),
    ]
    .align_y(Alignment::Center)
    .into();

    let version_line: Element<Message> = components::dimmed(
        format!(
            "Current: v{} → New: {}",
            crate::update::CURRENT_VERSION,
            release.tag_name
        ),
        p,
    )
    .into();

    let md_theme: iced::Theme = match snap.theme {
        crate::theme::ThemeKind::MacLight => iced::Theme::Light,
        crate::theme::ThemeKind::MacDark => iced::Theme::Dark,
    };

    let base_md_style = markdown::Style::from_palette(md_theme.palette());
    let md_style = markdown::Style {
        inline_code_highlight: markdown::Highlight {
            background: iced::Background::Color(p.accent_tint),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
        },
        inline_code_color: p.text_body,
        link_color: p.accent,
        ..base_md_style
    };

    let md_inner: Element<Message> = markdown::view(
        &snap.release_notes_items,
        markdown::Settings::with_text_size(13, md_style),
    )
    .map(Message::OpenUrl);

    let md_view: Element<Message> = iced::widget::themer(Some(md_theme), md_inner)
        .text_color(|theme: &iced::Theme| theme.palette().text)
        .into();

    let md_scroll: Element<Message> = components::scrollable(
        container(md_view)
            .padding(metric::GAP)
            .width(Length::Fill)
            .into(),
        p,
    )
    .height(Length::Fill)
    .into();

    let actions: Element<Message> = row![
        components::pill_button(
            "Open on GitHub",
            p,
            Some(Message::OpenUrl(release.html_url.clone())),
        ),
        components::pill_button("Close", p, Some(Message::CloseWindow(id))),
    ]
    .spacing(metric::GAP)
    .align_y(Alignment::Center)
    .into();

    let content: Element<Message> = column![title_bar, version_line, md_scroll, actions]
        .spacing(metric::GAP)
        .into();

    components::card(content, p)
}
