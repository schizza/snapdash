use std::{collections::HashMap, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use anyhow::{Context, Result};

use crate::theme::ThemeKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ha_url: String,
    pub ha_token_present: bool,
    pub theme: ThemeKind,
    #[serde(default)]
    pub debug_overlay: bool,
    #[serde(default)]
    pub widgets: Vec<String>,
    #[serde(default)]
    pub widget_positions: HashMap<String, WidgetPosition>,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq)]
pub struct WidgetPosition {
    pub x: f32,
    pub y: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ha_url: "http://localhost:8123".into(),
            theme: ThemeKind::default(),
            ha_token_present: false,
            debug_overlay: false,
            widgets: Vec::new(),
            widget_positions: HashMap::new(),
        }
    }
}

impl Config {
    fn project_dirs() -> Result<ProjectDirs> {
        ProjectDirs::from("dev", "snapdash", "Snapdash")
            .context("Cannot determine app config directory")
    }

    pub fn config_path() -> Result<PathBuf> {
        let proj = Self::project_dirs()?;
        Ok(proj.config_dir().join("config.json"))
    }

    pub fn ha_enabled(&self) -> bool {
        !self.ha_url.trim().is_empty() && self.ha_token_present
    }

    pub async fn load() -> anyhow::Result<Self> {
        let path = Self::config_path()?;
        let bytes = match tokio::fs::read(&path).await {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e).with_context(|| format!("Failed to read {:?}", path)),
        };

        match serde_json::from_slice::<Self>(&bytes) {
            Ok(cfg) => Ok(cfg),
            Err(e) => {
                eprintln!("Invalid JSON in {:#?}: {e}. Using default config.", path);
                Ok(Self::default())
            }
        }
    }

    pub async fn save_async(&self) -> Result<()> {
        let path = Self::config_path()?;

        let dir = path
            .parent()
            .context("Config path has no parent directory")?;

        tokio::fs::create_dir_all(dir)
            .await
            .with_context(|| format!("Failed to create config dir {:?}", dir))?;

        let json = serde_json::to_vec_pretty(self).context("Failed to serialize config")?;
        let tmp = path.with_extension("json.tmp");

        tokio::fs::write(&tmp, &json)
            .await
            .with_context(|| format!("Failed to write temp config {:?}", tmp))?;

        tokio::fs::rename(&tmp, &path)
            .await
            .with_context(|| format!("Failed to replace config {:?}", path))?;

        Ok(())
    }
}
