//! Application glue: the `Snapdash` state struct, its `Message` enum, and
//! the iced runtime hooks that drive them. Domain logic (HA, update,
//! config) lives in sibling modules and is composed here.
//!
//! Re-exports are explicit (not `pub use *`) so adding a private helper
//! to `snapdash` or `window` doesn't accidentally leak it.

mod lifecycle;
mod snapdash;
mod window;

pub use snapdash::{FocusDirection, Message, SettingsSensor, Snapdash};
pub use window::{EntityWindowState, WindowKind, WindowState, find_window_id};
