//! User-selectable size preset for entity widget windows. Persisted in
//! Config and applied at window-creation time plus in entity_window's
//! view to scale fonts, spacing and the card itself.

use serde::{Deserialize, Serialize};

use crate::ha::Control;
use crate::helpers;
use crate::theme::metric;

/// The height of the rail a scalar control puts under its label.
///
/// `iced::widget::Slider::DEFAULT_HEIGHT`, which these rows do not
/// override. Named here because the colour surface's height is stated
/// relative to a slider row's: both are headed by the same
/// label-and-readout line, and only what sits beneath it differs.
const SLIDER_HEIGHT: f32 = 16.0;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adaptive {
    pub adaptive_font: bool,
    pub adaptive_value: bool,
}

impl Adaptive {
    /// Returns the font size value to use for the
    /// given text lenght.
    /// Shrink gracefully, so long values stays on one
    /// line and don`t push the widget layout aroud.
    pub fn font_size(self, base: f32, text_len: usize) -> f32 {
        if !self.adaptive_font {
            return base;
        }

        let factor = match text_len {
            0..=9 => 1.0,
            10 => 0.85,
            11..=13 => 0.7,
            _ => 0.55,
        };
        base * factor
    }

    /// Humanize a numeric value if self.adaptive_value is on.
    /// TODO: implement compression (1234567 -> 1.23M).
    /// For now passes the raw value.
    pub fn adapted_value(self, raw: &str) -> String {
        if !self.adaptive_value {
            return raw.to_string();
        }

        helpers::humanize_magnitude(raw)
    }
}

/// Per-widget visual emphasis. Doesn't change window size — only how
/// prominent the value reads (color) and whether the card gets a steady
/// accent ring. Stored per entity in Config (mirrors widget_positions);
/// missing key = Normal.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

impl Priority {
    pub const ALL: &[Self] = &[Self::Low, Self::Normal, Self::High];

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

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

    /// Height a single-axis control adds to an expanded widget (#87):
    /// a label with its readout, and the slider under it.
    pub fn control_row_height(self) -> f32 {
        match self {
            Self::Small => 40.0,
            Self::Normal => 44.0,
            Self::Large => 50.0,
        }
    }

    /// The colour surface's size (#97): the width of a slider track, and
    /// half as tall.
    ///
    /// The track is the card's inner width, which is the window less the
    /// card's padding at both edges - 132, 172 and 212 across the three
    /// presets. Half as tall is the shape `ui::colour_texture` computes
    /// the field at, so a degree of hue and a point of saturation are
    /// about the same distance under the finger, and so that ticket 06
    /// can draw the texture into these bounds without stretching it.
    pub fn colour_field_size(self) -> iced::Size {
        let width = self.window_size().width - 2.0 * metric::PAD;

        iced::Size::new(width, width / 2.0)
    }

    /// Height one control adds, its own separating gap included.
    ///
    /// Every control is preceded by its own gap, which is how
    /// `entity_window` builds them: an entity with two controls gets two
    /// gaps, not one. Counting a single gap for the whole block left the
    /// window short by one gap per extra control, which the last row
    /// paid for out of its own slack.
    pub fn control_height(self, control: &Control) -> f32 {
        let body = match control {
            Control::Value(_) => self.control_row_height(),
            // The same label-and-readout line a slider row is headed by,
            // with the field standing where the rail would. Taking the
            // rail out rather than adding to the whole row is what keeps
            // a colour block and a slider block lining their labels up.
            Control::Color { .. } => {
                self.control_row_height() - SLIDER_HEIGHT + self.colour_field_size().height
            }
        };

        self.value_detail_gap() + body
    }

    /// Total height the controls area adds, or `0.0` when the entity has
    /// none to show.
    ///
    /// A sum rather than a multiplication, because controls are not all
    /// the same height: a colour surface is a field, not a slider.
    pub fn controls_height(self, controls: &[Control]) -> f32 {
        controls
            .iter()
            .map(|control| self.control_height(control))
            .sum()
    }
}

#[cfg(test)]
mod tests;
