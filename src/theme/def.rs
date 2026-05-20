use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::core::Palette;

/// Lightness hint for matching the OS dark/light mode and grouping in
/// the picker. Not derived from the palette — declared by the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    #[default]
    Dark,
}

/// Where a theme came from — drives whether it can be deleted/edited
/// and how it's labeled in the picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeSource {
    Builtin,
    UserFile(PathBuf),
}

/// A complete theme: identity metadata + the color palette. This is
/// the unit the loader produces and the picker lists. The on-disk JSON
/// shape matches the #[serde] fields below; `source` is set by the
/// loader, not stored in the file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDef {
    /// Schema version for forward-compat migrations. Bump when the
    /// palette gains required fields.
    #[serde(default = "default_schema")]
    pub schema: u32,

    pub name: String,

    #[serde(default)]
    pub author: Option<String>,

    #[serde(default)]
    pub appearance: Appearance,

    pub palette: Palette,

    /// Not part of the JSON — filled in by the loader.
    #[serde(skip, default = "default_source")]
    pub source: ThemeSource,
}

fn default_schema() -> u32 {
    1
}

fn default_source() -> ThemeSource {
    ThemeSource::Builtin
}

impl ThemeDef {
    /// Display label for the picker — "Dracula" or "Dracula — by X".
    pub fn label(&self) -> String {
        match &self.author {
            Some(author) => format!("{} — by {author}", self.name),
            None => self.name.clone(),
        }
    }
}

//
// TESTS
//

#[cfg(test)]
mod tests {
    use super::*;

    fn palette_json() -> &'static str {
        r##"{
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
           }"##
    }
    #[test]
    fn parses_full_theme_json() {
        let json = format!(
            r##"{{
                   "schema": 1,
                   "name": "Dracula",
                   "author": "Zeno Rocha",
                   "appearance": "dark",
                   "palette": {palette}
               }}"##,
            palette = palette_json(),
        );

        let theme: ThemeDef = serde_json::from_str(&json).expect("parse");
        assert_eq!(theme.name, "Dracula");
        assert_eq!(theme.schema, 1);
        assert_eq!(theme.appearance, Appearance::Dark);
        assert_eq!(theme.label(), "Dracula — by Zeno Rocha");
        assert_eq!(theme.palette.accent.into_rgba8(), [189, 147, 249, 255]);
        // source isn't in the JSON — defaults to Builtin
        assert_eq!(theme.source, ThemeSource::Builtin);
    }

    #[test]
    fn author_optional() {
        let json = format!(
            r##"{{
                   "name": "Minimal",
                   "palette": {palette}
               }}"##,
            palette = palette_json(),
        );

        let theme: ThemeDef = serde_json::from_str(&json).expect("parse");
        assert_eq!(theme.name, "Minimal");
        // author missing → None → label is just the name
        assert_eq!(theme.author, None);
        assert_eq!(theme.label(), "Minimal");
        // schema missing → default 1
        assert_eq!(theme.schema, 1);
        // appearance missing → Default (Dark)
        assert_eq!(theme.appearance, Appearance::Dark);
    }

    #[test]
    fn rejects_malformed_palette() {
        // Missing required field (no "success") → parse error.
        let json = format!(
            r##"{{
                   "name": "Broken",
                   "palette": {{ "bg": "#000000" }}
               }}"##,
        );
        assert!(serde_json::from_str::<ThemeDef>(&json).is_err());
    }
}
