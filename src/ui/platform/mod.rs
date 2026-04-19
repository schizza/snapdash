//! Platform-specific window sizing and chrome wrapping.
//!
//! Different OSes handle transparent frameless windows differently:
//!
//! - **macOS / Windows**: OS-level hacks in iced_winit (`CALayer.cornerRadius +
//!   masksToBounds`, `SetWindowRgn`) clip the window to a rounded shape, and
//!   the platform's WindowServer/DWM renders its own native drop shadow around
//!   that shape. The card fills the window edge-to-edge. The iced-side shader
//!   shadow is effectively invisible (it has nowhere to render since the card
//!   covers everything). Clear color can stay at the theme default.
//!
//! - **Linux (X11 / Wayland)**: There's no OS-level rounding hack that works
//!   well — XShape gives 1-bit jagged masks, and the compositor won't add a
//!   drop shadow to an undecorated transparent window for us. Instead we make
//!   the window surface LARGER than the visible card by [`SHADOW_MARGIN`] on
//!   every side, wrap the content in a transparent padding, and let the
//!   iced wgpu shader render its own anti-aliased rounded corners and drop
//!   shadow into the margin. The compositor then blends the margin's alpha
//!   with the desktop. This is how GTK4/libadwaita apps do it.
//!
//! Both branches expose the same small API ([`window_size`], [`wrap_outer`])
//! so `app.rs` stays platform-agnostic at the call sites.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(target_os = "linux"))]
mod native;
#[cfg(not(target_os = "linux"))]
pub use native::*;
