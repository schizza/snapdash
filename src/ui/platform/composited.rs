//! Composited shadow rendering for Linux + Windows.
//!
//! Both platforms render their own anti-aliased rounded corners + drop
//! shadow inside the window via the iced-wgpu shader, since the OS-level
//! mechanisms either don't exist (Linux compositors) or produce jagged
//! 1-bit results (Windows `SetWindowRgn`).
//!
//! See `super` module doc for the high-level approach.

use iced::window::settings::PlatformSpecific;
use iced::{Element, Length};

use crate::app::Message;

/// Transparent margin around the card reserved for the iced-wgpu drop shadow
/// to fade into. Must be >= `shadow.blur_radius + |shadow.offset|` of the
/// largest theme shadow.
///
/// Current themes: `blur_radius` 20–22, `offset.y` 10 → 32 covers every side
/// with 0–10 px headroom. Bump if the theme shadow grows.
pub const SHADOW_MARGIN: f32 = 32.0;

/// Window surface size for a given card size.
///
/// On Linux we enlarge the surface by `SHADOW_MARGIN` on each side so the
/// shader-rendered drop shadow has transparent space to fade into, instead
/// of being clipped at the window edge and showing up as jagged "corner
/// triangles".
pub fn window_size(card_width: f32, card_height: f32) -> iced::Size {
    iced::Size::new(
        card_width + 2.0 * SHADOW_MARGIN,
        card_height + 2.0 * SHADOW_MARGIN,
    )
}

/// Wraps the window's content in a transparent `SHADOW_MARGIN` padding.
///
/// The iced-wgpu quad shader draws the card's drop shadow in this margin
/// area; clearing the surface to `Color::TRANSPARENT` (see
/// `Snapdash::style`) lets the compositor blend those semi-transparent
/// shadow pixels with the desktop.
pub fn wrap_outer<'a>(inner: Element<'a, Message>) -> Element<'a, Message> {
    iced::widget::container(inner)
        .padding(SHADOW_MARGIN)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn window_settings(size: iced::Size, resizable: bool) -> window::Settings {
    window::Settings {
        size: window_size(size.width, size.height),
        resizable,
        decorations: false,
        transparent: true,
        platform_specific: build_platform_specific(),
        ..window::Settings::default()
    }
}

fn build_platform_specific() -> PlatformSpecific {
    { PlatformSpecific::default() }
}
