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

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: iced::Color,
    pub card: iced::Color,
    pub card_2: iced::Color,

    pub text_primary: iced::Color,   // main headings
    pub text_secondary: iced::Color, // subhedings, labels
    pub text_body: iced::Color,      // normal text
    pub text_dim: iced::Color,       // placeholder, hint
    pub text_disabled: iced::Color,

    pub border: iced::Color,
    pub border_hovered: iced::Color,

    pub accent: iced::Color,
    pub accent_dim: iced::Color,
    pub accent_tint: iced::Color,

    pub shadow: iced::Shadow,

    pub danger: iced::Color,
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
