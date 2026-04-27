//! Platform-specific window sizing and chrome wrapping.
//!
//! Different OSes handle transparent frameless windows differently:
//!
//! - **macOS and Windows**: iced_winit/platform settings clip the window to a
//!   rounded shape and the platform compositor renders its native drop shadow
//!   around that shape. The card fills the window edge-to-edge. The iced-side
//!   shader shadow is effectively invisible (it has nowhere to render since the
//!   card covers everything). Clear color can stay at the theme default.
//!
//! - **Linux (X11 / Wayland)**: There's no OS-level rounding that yields
//!   anti-aliased edges + shadow for an undecorated transparent window. Linux
//!   compositors won't draw shadows for borderless surfaces, so we rely on:
//!     1. Window surface enlarged by [`SHADOW_MARGIN`] on every side.
//!     2. Visible content wrapped in a transparent padding of the same width.
//!     3. `Snapdash::style` clearing the surface to `Color::TRANSPARENT`.
//!     4. The iced wgpu shader rendering anti-aliased rounded corners +
//!        drop shadow into the margin, blended onto the desktop by the
//!        compositor.
//!
//!   This is how GTK4/libadwaita apps draw their own shadow on Linux, and
//!   keeps us away from 1-bit XShape corners when a compositor is present.
//!
//! Both branches expose the same small API ([`window_size`], [`wrap_outer`])
//! so `app.rs` stays platform-agnostic at the call sites.

#[cfg(target_os = "linux")]
mod composited;
#[cfg(target_os = "linux")]
pub use composited::*;

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod native_shadow;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub use native_shadow::*;
