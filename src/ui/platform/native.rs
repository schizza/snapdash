//! macOS / Windows platform bits. See `super` module doc for the big picture.
//!
//! Both platforms rely on OS-level hacks in iced_winit (`CALayer.cornerRadius
//! and masksToBounds` on macOS, `SetWindowRgn` on Windows) to clip the window
//! surface to a rounded rectangle, with a platform-native drop shadow rendered
//! by WindowServer / DWM outside the clipped region. The card fills the
//! window edge-to-edge and the iced-side shader shadow is not used.
//!
//! The helpers here are therefore pass-throughs that keep `app.rs` tidy.

use iced::Element;

use crate::app::Message;

/// Window surface size for a given card size.
///
/// On macOS/Windows the surface is exactly the card size — the OS clips the
/// outer edge to a rounded shape and provides the drop shadow itself.
pub fn window_size(card_width: f32, card_height: f32) -> iced::Size {
    iced::Size::new(card_width, card_height)
}

/// Pass-through. The card already fills the window; no extra outer margin.
pub fn wrap_outer<'a>(inner: Element<'a, Message>) -> Element<'a, Message> {
    inner
}
