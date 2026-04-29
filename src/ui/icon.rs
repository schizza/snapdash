// Icons for the UI.
//
// Lucide `settings` glyph. Codepoint comes from `assets/fonts/lucide-info.json`;
// bump together with the lucide.ttf bundled in `lib.rs` if you ever swap icons.
// Using a bundled icon font instead of the Unicode gear (U+2699) avoids
// per-OS fallback to Segoe UI Emoji on Windows (colored emoji) vs Apple
// Symbols on macOS (thin outline) vs whatever fontconfig resolves on Linux.

use crate::ui::theme::{UiTheme, icon_text};

#[derive(Copy, Clone)]
pub enum Icon {
    Gear,
    Download,
    Check,
    Trash,
    Close,
    Unknown,
}

impl Icon {
    const ICON_FONT: iced::Font = iced::Font::with_name("lucide");
    const ICON_SIZE: f32 = 20.0;

    const fn glyph(self) -> char {
        match self {
            Self::Gear => '\u{e154}',
            Self::Download => '\u{e0b2}',
            Self::Check => '\u{e06c}',
            Self::Trash => '\u{e18e}',
            Self::Close => '\u{e1b2}',
            Self::Unknown => '\u{e47b}',
        }
    }

    pub fn text<'a>(self, ui_theme: UiTheme) -> iced::widget::Text<'a> {
        iced::widget::text(self.glyph())
            .size(Self::ICON_SIZE)
            .font(Self::ICON_FONT)
            .style(icon_text(ui_theme, 1.0))
    }
}
