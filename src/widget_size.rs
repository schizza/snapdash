//! User-selectable size preset for entity widget windows. Persisted in
//! Config and applied at window-creation time plus in entity_window's
//! view to scale fonts, spacing and the card itself.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum WidgetSize {
    Small,
    Normal,
    #[default]
    Large,
}

impl std::fmt::Display for WidgetSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl WidgetSize {
    pub const ALL: &[Self] = &[Self::Small, Self::Normal, Self::Large];

    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Normal => "Normal",
            Self::Large => "Large",
        }
    }

    pub fn window_size(self) -> iced::Size {
        match self {
            Self::Small => iced::Size::new(160.0, 110.0),
            Self::Normal => iced::Size::new(200.0, 135.0),
            Self::Large => iced::Size::new(240.0, 160.0),
        }
    }

    pub fn value_font(self) -> f32 {
        match self {
            Self::Small => 28.0,
            Self::Normal => 36.0,
            Self::Large => 44.0,
        }
    }

    pub fn title_font(self) -> f32 {
        match self {
            Self::Small => 11.0,
            Self::Normal => 12.0,
            Self::Large => 14.0,
        }
    }

    pub fn detail_font(self) -> f32 {
        match self {
            Self::Small => 10.0,
            Self::Normal => 11.0,
            Self::Large => 12.0,
        }
    }

    pub fn title_value_gap(self) -> f32 {
        match self {
            Self::Small => 4.0,
            Self::Normal => 5.0,
            Self::Large => 6.0,
        }
    }

    pub fn value_detail_gap(self) -> f32 {
        match self {
            Self::Small => 6.0,
            Self::Normal => 8.0,
            Self::Large => 10.0,
        }
    }
}
