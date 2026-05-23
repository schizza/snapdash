use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

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

/// The full theme catalog: builtins first, then user themes, deduped
/// by name. Builtins win over same-named user files. Does filesystem
/// IO (scans the themes dir) — call once at startup.
pub fn available_themes() -> Vec<ThemeDef> {
    merge_unique(builtin_themes(), load_user_themes())
}

/// Merges two theme lists keeping the first occurrence of each name,
/// so name-based lookup (resolve_theme) stays deterministic. Split out
/// from available_themes() so the dedup rule is unit-testable without
/// touching the filesystem.
fn merge_unique(first: Vec<ThemeDef>, second: Vec<ThemeDef>) -> Vec<ThemeDef> {
    let mut seen: HashSet<String> = first.iter().map(|t| t.name.clone()).collect();
    let mut out = first;

    for theme in second {
        if seen.insert(theme.name.clone()) {
            out.push(theme);
        } else {
            tracing::warn!(name = %theme.name, "duplicate theme name — skipping");
        }
    }
    out
}

/// Resolves a stored theme name against the available catalog.
/// Falls back through legacy ThemeKind enum names ("MacDark") for
/// configs written before themes were named. Returns None if nothing
/// matches — caller picks a default.
pub fn resolve_theme<'a>(name: &str, available: &'a [ThemeDef]) -> Option<&'a ThemeDef> {
    if let Some(t) = available.iter().find(|t| t.name == name) {
        return Some(t);
    }
    // Legacy migration: old config stores the ThemeKind enum variant.
    let migrated = match name {
        "MacDark" => "Mac Dark",
        "MacLight" => "Mac Light",
        _ => return None,
    };

    available.iter().find(|t| t.name == migrated)
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
    let entries = match std::fs::read_dir(dir) {
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

/// Validates raw bytes as a theme, returning a descriptive error
/// suitable for showing the user. Unlike the directory loader (which
/// silently skips malformed files), import is explicit — the user
/// picked this file and needs to know exactly why it failed.
pub fn validate_theme_bytes(bytes: &[u8]) -> Result<ThemeDef, String> {
    serde_json::from_slice::<ThemeDef>(bytes).map_err(|e| format!("Invalid theme file: {e}"))
}

/// Imports a theme file: validates it, then copies it into the user
/// themes directory under a sanitized filename derived from the theme
/// name. Returns the imported theme's display name on success.
pub fn import_theme_file(source: &std::path::Path) -> Result<String, String> {
    let bytes =
        std::fs::read(source).map_err(|e| format!("Cannot read {}: {e}", source.display()))?;

    // Validate first, do not copy garbage into theme folder
    let theme = validate_theme_bytes(&bytes)?;

    let dir = themes_dir().ok_or_else(|| "Cannon resolve themes directory".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create themes dir: {e}"))?;

    // Drop any prior file for this name first (incl. older naming schemes)
    // so reimport overwrites cleanly instead of leaving a stale duplicate
    remove_existing_by_name(&dir, &theme.name);
    let dest = theme_dest_path(&dir, &theme.name);

    std::fs::write(&dest, &bytes).map_err(|e| format!("Cannot write theme: {e}"))?;

    tracing::info!(name = %theme.name, path = %dest.display(), "imported theme");
    Ok(theme.name)
}

/// Install a theme fetched from the gallery into the user themes
/// directory. Unlike `import_theme_file` (which copies the original file
/// bytes), this serializes the already-parsed `ThemeDef` back to JSON —
/// the gallery hands us full defs, not a file on disk. Overwrites an
/// existing same-named file, consistent with import.
pub fn install_theme(theme: &ThemeDef) -> Result<String, String> {
    let dir = themes_dir().ok_or_else(|| "Cannot resolve themes directory".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create themes dir: {e}"))?;

    let bytes =
        serde_json::to_vec_pretty(theme).map_err(|e| format!("Cannot srialize theme: {e}"))?;

    // Drop any prior file for this name first (incl. older naming schemes)
    // so reimport overwrites cleanly instead of leaving a stale duplicate
    remove_existing_by_name(&dir, &theme.name);
    let dest = theme_dest_path(&dir, &theme.name);

    std::fs::write(&dest, &bytes).map_err(|e| format!("Cannot write theme: {e}"))?;
    tracing::info!(name = %theme.name, path = %dest.display(), "installed theme from gallery");
    Ok(theme.name.clone())
}

/// Filename-safe slug from a theme name: `"Tokyo Night"` → `"tokyo-night"`.
/// No extension, no disambiguation — see `theme_dest_path`.
fn sanitize_slug(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' => c,
            'A'..='Z' => c.to_ascii_lowercase(),
            ' ' | '_' => '-',
            _ => '-',
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Resolve the file a theme should be written to. Clean slug by default
/// (`dracula.json`); a stable hash suffix is appended only when the slug
/// is already taken by a *different* theme name, so distinct names never
/// clobber each other while the common case keeps readable filenames.
///
/// Call `remove_existing_by_name` first — then any occupant of the plain
/// slug is genuinely a different theme, not our own previous file.
fn theme_dest_path(dir: &Path, name: &str) -> PathBuf {
    let slug = sanitize_slug(name);
    let slug = if slug.is_empty() {
        "theme".to_string()
    } else {
        slug
    };

    let plain = dir.join(format!("{slug}.json"));

    let collision = std::fs::read(&plain)
        .ok()
        .and_then(|b| validate_theme_bytes(&b).ok())
        .is_some_and(|t| t.name != name);

    if collision {
        dir.join(format!("{slug}-{:08x}.json", fnv1a(name)))
    } else {
        plain
    }
}

/// Remove every theme file in `dir` whose stored name equals `name`.
/// Keyed on the in-file name, not the filename, so it cleans up files
/// written under any past naming scheme. Non-theme files (index.json,
/// garbage) fail to parse and are left alone.
fn remove_existing_by_name(dir: &Path, name: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let matches = std::fs::read(&path)
            .ok()
            .and_then(|b| validate_theme_bytes(&b).ok())
            .is_some_and(|t| t.name == name);

        if matches {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// FNV-1a 32-bit. Used only to disambiguate filename slugs — small and
/// stable across runs/Rust versions (std's `DefaultHasher` is not
/// guaranteed stable).
fn fnv1a(s: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in s.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

//
// TESTS
//

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

    /// Palette object shared by the JSON fixtures below.
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

    /// A valid theme JSON with an arbitrary `name` — lets tests exercise
    /// the filename/dedupe logic, which keys off the stored name.
    fn theme_json_named(name: &str) -> String {
        format!(
            r##"{{ "schema": 1, "name": "{name}", "appearance": "dark", "palette": {palette} }}"##,
            palette = palette_json(),
        )
    }

    #[test]
    fn validate_accepts_good_theme() {
        let bytes = valid_theme_json().as_bytes(); // reuse helper z loader testů
        assert!(validate_theme_bytes(bytes).is_ok());
    }

    #[test]
    fn validate_rejects_with_descriptive_error() {
        let bytes = br##"{ "name": "Broken", "palette": { "bg": "#000000" } }"##;
        let err = validate_theme_bytes(bytes).unwrap_err();
        assert!(err.contains("Invalid theme file"));
        // serde mentions the missing field
        assert!(err.to_lowercase().contains("missing"));
    }

    #[test]
    fn sanitize_makes_safe_filenames() {
        assert!(sanitize_slug("Dracula").starts_with("dracula"));
        assert!(sanitize_slug("My Cool Theme").starts_with("my-cool-theme"));
        assert!(sanitize_slug("Solarized (Light)").starts_with("solarized--light"));
    }

    #[test]
    fn distinct_names_sharing_a_slug_get_separate_files() {
        let dir = tempfile::tempdir().unwrap();

        // "A/B" lands on the clean slug...
        let p1 = theme_dest_path(dir.path(), "A/B");
        assert_eq!(p1.file_name().unwrap(), "a-b.json");
        std::fs::write(&p1, theme_json_named("A/B")).unwrap();

        // ...and "A?B" (same slug) must NOT overwrite it.
        let p2 = theme_dest_path(dir.path(), "A?B");
        assert_ne!(p1, p2);

        // Re-resolving "A/B" still hits its own file (overwrite on reimport).
        std::fs::write(dir.path().join("a-b.json"), theme_json_named("A/B")).unwrap();
        assert_eq!(theme_dest_path(dir.path(), "A/B"), p1);
    }
}
