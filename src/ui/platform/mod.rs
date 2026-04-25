//! Platform-specific window sizing and chrome wrapping.
//!
//! Different OSes handle transparent frameless windows differently:
//!
//! - **macOS**: OS-level hacks in iced_winit (`CALayer.cornerRadius +
//!   masksToBounds`, `SetWindowRgn`) clip the window to a rounded shape, and
//!   the platform's WindowServer/DWM renders its own native drop shadow around
//!   that shape. The card fills the window edge-to-edge. The iced-side shader
//!   shadow is effectively invisible (it has nowhere to render since the card
//!   covers everything). Clear color can stay at the theme default.
//!
//! - **Linux (X11 / Wayland) and Windows**: There's no OS-level rounding
//!   that yields anti-aliased edges + shadow for an undecorated transparent
//!   window. Linux compositors won't draw shadows for borderless surfaces;
//!   Windows uses `SetWindowRgn` which produces 1-bit jagged ("cranky")
//!   masks and no DWM shadow. Both platforms instead rely on:
//!     1. Window surface enlarged by [`SHADOW_MARGIN`] on every side.
//!     2. Visible content wrapped in a transparent padding of the same width.
//!     3. `Snapdash::style` clearing the surface to `Color::TRANSPARENT`.
//!     4. The iced wgpu shader rendering anti-aliased rounded corners +
//!        drop shadow into the margin, blended onto the desktop by the
//!        compositor / DWM.
//!
//!   This is how GTK4/libadwaita apps draw their own shadow on Linux, and
//!   the same approach works on Windows for an identical look.
//!
//! Both branches expose the same small API ([`window_size`], [`wrap_outer`])
//! so `app.rs` stays platform-agnostic at the call sites.

#[cfg(any(target_os = "linux", target_os = "windows"))]
mod composited;
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub use composited::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;
