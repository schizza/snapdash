//! macOS platform bits. See `super` module doc for the big picture.
//!
//! macOS uses an iced_winit hack (`CALayer.cornerRadius +
//! masksToBounds`) to clip the window surface to a rounded rectangle,
//! with WindowServer rendering a platform-native drop shadow around the
//! clipped region. The card fills the window edge-to-edge and the iced
//! shader shadow is not used.
//!
//! Helpers here are therefore pass-throughs that keep `app.rs` tidy.

use iced::Element;

use crate::app::Message;

/// Window surface size for a given card size.
///
/// On macOS the surface is exactly the card size — the OS clips the
/// outer edge to a rounded shape and provides the drop shadow itself.
pub fn window_size(card_width: f32, card_height: f32) -> iced::Size {
    iced::Size::new(card_width, card_height)
}

/// Pass-through. The card already fills the window; no extra outer margin.
pub fn wrap_outer<'a>(inner: Element<'a, Message>) -> Element<'a, Message> {
    inner
}
