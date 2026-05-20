use std::path::PathBuf;

use directories::ProjectDirs;

use super::def::{Appearance, ThemeDef, ThemeSource};
use crate::theme::ThemeKind;

/// Directory where user themes live: `<config>/themes/`. Same
/// ProjectDirs root as config.json, so users find both in one plac
pub fn themes_dir() -> Option<PathBuf> {
    ProjectDirs::from("dev", "snapdash", "Snapdash").map(|proj| proj.config_dir().join("themes"))
}

/// The two built-in themes, always available regardless of user files.
/// Wrap the existing ThemeKind palettes as ThemeDef so the picker can
/// treat builtin and user themes uniformly.
pub fn builtin_themes() -> Vec<ThemeDef> {
    vec![
        ThemeDef {
            schema: 1,
            name: "Mac Light".to_string(),
            author: None,
            appearance: Appearance::Light,
            palette: ThemeKind::MacLight.palette(),
            source: ThemeSource::Builtin,
        },
        ThemeDef {
            schema: 1,
            name: "Mac Dark".to_string(),
            author: None,
            appearance: Appearance::Dark,
            palette: ThemeKind::MacDark.palette(),
            source: ThemeSource::Builtin,
        },
    ]
}

/// Loads all valid `.json` themes from the themes directory. Malformed
/// files (bad JSON, missing fields, invalid hex) are logged and
/// skipped — one broken theme never blocks the rest or the picker.
/// Missing directory returns an empty list (not an error).
pub fn load_user_themes() -> Vec<ThemeDef> {
    match themes_dir() {
        Some(dir) => load_themes_from(&dir),
        None => {
            tracing::warn!("could not resolve themes directory");
            Vec::new()
        }
    }
}

pub fn load_themes_from(dir: &std::path::Path) -> Vec<ThemeDef> {
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // No themes dir yet — fine, user hasn't added any.
            return Vec::new();
        }
        Err(e) => {
            tracing::warn!(dir = %dir.display(), error = %e, "cannot read themes dir");
            return Vec::new();
        }
    };

    let mut themes = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        match load_one(&path) {
            Ok(theme) => {
                tracing::info!(name = %theme.name, path = %path.display(), "loaded theme");
                themes.push(theme);
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "skipping invalid theme");
            }
        }
    }

    themes
}

/// Parses a single theme file and tags it with its source path.
fn load_one(path: &std::path::Path) -> anyhow::Result<ThemeDef> {
    let bytes = std::fs::read(path)?;
    let mut theme: ThemeDef = serde_json::from_slice(&bytes)?;
    theme.source = ThemeSource::UserFile(path.to_path_buf());
    Ok(theme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_themes_always_present() {
        let themes = builtin_themes();
        assert_eq!(themes.len(), 2);
        assert!(themes.iter().any(|t| t.name == "Mac Light"));
        assert!(themes.iter().any(|t| t.name == "Mac Dark"));
        assert!(themes.iter().all(|t| t.source == ThemeSource::Builtin));
    }

    #[test]
    fn loads_valid_theme_and_tags_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("dracula.json");
        std::fs::write(&path, valid_theme_json()).unwrap();

        let themes = load_themes_from(dir.path());

        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].name, "Dracula");
        // loader tags the file path as the source
        assert_eq!(themes[0].source, ThemeSource::UserFile(path));
    }

    #[test]
    fn skips_invalid_files_keeps_valid() {
        // Use a temp dir with one valid + one broken theme.
        let dir = tempfile::tempdir().expect("tempdir");

        std::fs::write(dir.path().join("good.json"), valid_theme_json()).unwrap();

        std::fs::write(
            dir.path().join("broken.json"),
            r##"{ "name": "Broken", "palette": { "bg": "#000" } }"##, // missing fields
        )
        .unwrap();

        std::fs::write(dir.path().join("notes.txt"), "ignore me").unwrap();

        // load_dir is a testable variant that takes an explicit path
        let themes = load_themes_from(dir.path());
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].name, "Dracula");
    }

    #[test]
    fn missing_dir_returns_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nonexistent = dir.path().join("does_not_exist");

        let themes = load_themes_from(&nonexistent);

        assert!(themes.is_empty());
    }

    fn valid_theme_json() -> &'static str {
        r##"{
               "schema": 1,
               "name": "Dracula",
               "author": "Zeno Rocha",
               "appearance": "dark",
               "palette": {
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
               }
           }"##
    }
}
