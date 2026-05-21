use crate::theme::{hex_color, shadow};
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum ThemeKind {
    MacLight,
    #[default]
    MacDark,
}

impl fmt::Display for ThemeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeKind::MacLight => write!(f, "Mac Light"),
            ThemeKind::MacDark => write!(f, "Mac Dark"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Palette {
    #[serde(with = "hex_color")]
    pub bg: iced::Color,
    #[serde(with = "hex_color")]
    pub card: iced::Color,
    #[serde(with = "hex_color")]
    pub card_2: iced::Color,
    #[serde(with = "hex_color")]
    pub text_primary: iced::Color, // main headings
    #[serde(with = "hex_color")]
    pub text_secondary: iced::Color, // subhedings, labels
    #[serde(with = "hex_color")]
    pub text_body: iced::Color, // normal text
    #[serde(with = "hex_color")]
    pub text_dim: iced::Color, // placeholder, hint
    #[serde(with = "hex_color")]
    pub text_disabled: iced::Color,

    #[serde(with = "hex_color")]
    pub border: iced::Color,
    #[serde(with = "hex_color")]
    pub border_hovered: iced::Color,

    #[serde(with = "hex_color")]
    pub accent: iced::Color,
    #[serde(with = "hex_color")]
    pub accent_dim: iced::Color,
    #[serde(with = "hex_color")]
    pub accent_tint: iced::Color,

    #[serde(with = "shadow")]
    pub shadow: iced::Shadow,

    #[serde(with = "hex_color")]
    pub danger: iced::Color,
    #[serde(with = "hex_color")]
    pub success: iced::Color,
}

impl ThemeKind {
    pub fn palette(self) -> Palette {
        match self {
            ThemeKind::MacLight => Palette {
                bg: iced::Color::from_rgba8(246, 246, 248, 0.5),
                card: iced::Color::from_rgb8(255, 255, 255),
                card_2: iced::Color::from_rgb8(250, 250, 252),

                text_primary: iced::Color::from_rgb8(28, 28, 30),
                text_secondary: iced::Color::from_rgb8(60, 60, 67),
                text_body: iced::Color::from_rgb8(44, 44, 46),
                text_dim: iced::Color::from_rgb8(142, 142, 147),
                text_disabled: iced::Color::from_rgb8(174, 174, 178),

                border: iced::Color::from_rgba8(0, 0, 0, 0.08),
                border_hovered: iced::Color::from_rgba8(0, 0, 0, 0.20),

                accent: iced::Color::from_rgb8(0, 122, 255),
                accent_dim: iced::Color::from_rgb8(0, 102, 215),

                // hodně jemný “selection fill”
                accent_tint: iced::Color::from_rgba8(0, 122, 255, 0.14),

                shadow: iced::Shadow {
                    color: iced::Color::from_rgba8(0, 0, 0, 0.25),
                    offset: iced::Vector::new(0.0, 10.0),
                    blur_radius: 20.0,
                },

                danger: iced::Color::from_rgb8(255, 59, 48),
                success: iced::Color::from_rgb8(52, 199, 89),
            },

            ThemeKind::MacDark => Palette {
                bg: iced::Color::from_rgb8(20, 20, 22),
                card: iced::Color::from_rgb8(30, 30, 34),
                card_2: iced::Color::from_rgb8(36, 36, 40),

                text_primary: iced::Color::from_rgb8(238, 238, 244),
                text_secondary: iced::Color::from_rgb8(200, 200, 210),
                text_body: iced::Color::from_rgb8(220, 220, 228),
                text_dim: iced::Color::from_rgb8(150, 150, 165),
                text_disabled: iced::Color::from_rgb8(118, 118, 128),

                border: iced::Color::from_rgba8(255, 255, 255, 0.10),
                border_hovered: iced::Color::from_rgba8(255, 255, 255, 0.20),

                accent: iced::Color::from_rgb8(10, 132, 255),
                accent_dim: iced::Color::from_rgb8(64, 156, 255),

                accent_tint: iced::Color::from_rgba8(10, 132, 255, 0.16),

                shadow: iced::Shadow {
                    color: iced::Color::from_rgba8(0, 0, 0, 0.35),
                    offset: iced::Vector::new(0.0, 10.0),
                    blur_radius: 22.0,
                },

                danger: iced::Color::from_rgb8(255, 69, 58),
                success: iced::Color::from_rgb8(48, 209, 88),
            },
        }
    }
}

pub mod metric {
    pub const RADIUS: f32 = 20.0;
    pub const PAD: f32 = 14.0;
    pub const GAP: f32 = 12.0;
}

pub mod text_size {
    pub const NORMAL: f32 = 13.0;
    pub const SMALL: f32 = 11.0;
    pub const XSMALL: f32 = 10.0;
    pub const LARGE: f32 = 15.0;
    pub const XLARGE: f32 = 22.0;
}

//
//   TESTS
//
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_roundtrips_through_json() {
        let original = ThemeKind::MacDark.palette();
        let json = serde_json::to_string_pretty(&original).expect("serialize");
        let restored: Palette = serde_json::from_str(&json).expect("deserialize");

        // Compare a few key fields (Palette isn't PartialEq — compare components)
        assert_eq!(original.accent.into_rgba8(), restored.accent.into_rgba8());
        assert_eq!(original.bg.into_rgba8(), restored.bg.into_rgba8());
        assert_eq!(original.shadow.blur_radius, restored.shadow.blur_radius);
    }

    #[test]
    fn parses_opaque_and_alpha_hex() {
        let json = r##"{
            "bg": "#1e1e2e",
            "card": "#282a36",
            "card_2": "#21222c",
            "text_primary": "#f8f8f2",
            "text_secondary": "#e0e0d0",
            "text_body": "#cdd6f4",
            "text_dim": "#6272a4",
            "text_disabled": "#45475a",
            "border": "#44475a80",
            "border_hovered": "#44475a",
            "accent": "#bd93f9",
            "accent_dim": "#a679e0",
            "accent_tint": "#bd93f924",
            "shadow": { "color": "#00000040", "offset_x": 0.0, "offset_y": 10.0, "blur_radius": 20.0 },
            "danger": "#ff5555",
            "success": "#50fa7b"
        }"##;
        let p: Palette = serde_json::from_str(json).expect("parse");
        assert_eq!(p.accent.into_rgba8(), [189, 147, 249, 255]);
    }
}
