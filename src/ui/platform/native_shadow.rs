//! Native platform shadow rendering for macOS + Windows.
//!
//! macOS uses an iced_winit layer setup (`CALayer.cornerRadius + masksToBounds`)
//! and Windows uses DWM corner/shadow settings. In both cases the OS clips the
//! surface to a rounded rectangle and renders a platform-native drop shadow
//! around the clipped region. The card fills the window edge-to-edge and the
//! iced shader shadow is not used.
//!
//! Helpers here are therefore pass-throughs that keep `app.rs` tidy.

use iced::window::settings::PlatformSpecific;
use iced::{Element, window};

use crate::app::Message;

/// Window surface size for a given card size.
///
/// The surface is exactly the card size — the OS clips the outer edge to a
/// rounded shape and provides the drop shadow itself.
pub fn window_size(card_width: f32, card_height: f32) -> iced::Size {
    iced::Size::new(card_width, card_height)
}

/// Pass-through. The card already fills the window; no extra outer margin.
pub fn wrap_outer<'a>(inner: Element<'a, Message>) -> Element<'a, Message> {
    inner
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
    #[cfg(target_os = "windows")]
    {
        use iced::window::settings::platform::CornerPreference;

        PlatformSpecific {
            undecorated_shadow: true,
            corner_preference: CornerPreference::Round,
            ..Default::default()
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        PlatformSpecific::default()
    }
}
