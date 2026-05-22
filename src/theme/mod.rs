pub mod core;
pub mod def;
pub mod gallery;
pub mod hex_color;
pub mod loader;
pub mod shadow;

pub use core::{Palette, ThemeKind, metric, text_size};
pub use def::{Appearance, ThemeDef, ThemeSource};
pub use loader::{
    available_themes, builtin_themes, import_theme_file, install_theme, load_user_themes,
    resolve_theme, themes_dir, validate_theme_bytes,
};

pub use gallery::fetch_index;

pub const DEFAULT_THEME: &str = "Mac Dark";
