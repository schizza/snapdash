use std::{collections::HashMap, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use anyhow::{Context, Result};

use crate::widget_visibility::VisibilityRule;
use crate::{theme::DEFAULT_THEME, widget_size::Priority};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ha_url: String,
    pub ha_token_present: bool,
    #[serde(default = "default_theme_name")]
    pub theme: String,
    #[serde(default)]
    pub debug_overlay: bool,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub widget_settings: WidgetSettings,
    #[serde(default)]
    pub widgets: Vec<String>,

    /// All per-entity widget state, keyed by entity_id.
    ///
    /// Replaces the four parallel maps this struct used to carry
    /// (positions / priorities / names / visibility). Those could drift
    /// out of sync — `ToggleWidget` removed the entity from `widgets`
    /// but cleaned none of them, so every removed widget leaked its
    /// state into config.json forever. One map makes that impossible:
    /// removal is a single `remove` and there is no "other map" to
    /// forget.
    #[serde(default)]
    pub widget_config: HashMap<String, WidgetConfig>,

    // --- Legacy layout, read once and folded into `widget_config` ---
    //
    // Deserialized so configs written before the consolidation still
    // load with everything intact, never written back. `Config::load`
    // drains them via `migrate_legacy_widget_maps`; by the time anyone
    // else sees a `Config` they are empty.
    #[serde(default, rename = "widget_positions", skip_serializing)]
    legacy_positions: HashMap<String, WidgetPosition>,
    #[serde(default, rename = "widget_priorities", skip_serializing)]
    legacy_priorities: HashMap<String, Priority>,
    #[serde(default, rename = "widget_names", skip_serializing)]
    legacy_names: HashMap<String, String>,
    #[serde(default, rename = "widget_visibility", skip_serializing)]
    legacy_visibility: HashMap<String, VisibilityRule>,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize, PartialEq)]
pub struct WidgetPosition {
    pub x: f32,
    pub y: f32,
}

/// Everything the user has configured for one widget.
///
/// Every field is optional-or-default, so a widget the user hasn't
/// customised is `WidgetConfig::default()` and gets pruned on save —
/// keeping the "only deviations are persisted" property the parallel
/// maps had, at entry granularity instead of key granularity.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct WidgetConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<WidgetPosition>,
    #[serde(default, skip_serializing_if = "is_default_priority")]
    pub priority: Priority,
    /// Custom title override. `None` falls back to HA's friendly_name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<VisibilityRule>,
    /// Require a confirmation step before the primary action fires.
    /// Gates the action only, never the continuous controls: a slider is
    /// continuous and reversible, so a prompt in front of every drag
    /// would be an obstacle rather than a safeguard.
    #[serde(default, skip_serializing_if = "is_false")]
    pub require_confirm: bool,
}

impl WidgetConfig {
    /// `true` when nothing has been customised, so the entry carries no
    /// information and can be dropped on save.
    fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

fn is_default_priority(p: &Priority) -> bool {
    *p == Priority::default()
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct WidgetSettings {
    #[serde(default)]
    pub widget_size: crate::widget_size::WidgetSize,
    #[serde(default)]
    pub adaptive: crate::widget_size::Adaptive,
    #[serde(default)]
    pub show_measurement_info: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ha_url: "http://localhost:8123".into(),
            theme: "Mac Dark".into(),
            ha_token_present: false,
            autostart: false,
            debug_overlay: false,
            widgets: Vec::new(),
            widget_config: HashMap::new(),
            widget_settings: WidgetSettings {
                widget_size: crate::widget_size::WidgetSize::default(),
                adaptive: crate::widget_size::Adaptive::default(),
                show_measurement_info: true,
            },
            legacy_positions: HashMap::new(),
            legacy_priorities: HashMap::new(),
            legacy_names: HashMap::new(),
            legacy_visibility: HashMap::new(),
        }
    }
}

impl Config {
    // --- Per-widget accessors ---
    //
    // Read paths return defaults rather than Option-of-default so call
    // sites don't each repeat `.get(id).copied().unwrap_or_default()`.

    /// Priority for a widget. Un-customised widgets are `Normal`.
    pub fn priority(&self, entity_id: &str) -> Priority {
        self.widget_config
            .get(entity_id)
            .map(|w| w.priority)
            .unwrap_or_default()
    }

    /// The user's custom title override, if they set a non-empty one.
    pub fn name_override(&self, entity_id: &str) -> Option<&str> {
        self.widget_config
            .get(entity_id)?
            .name
            .as_deref()
            .filter(|s| !s.is_empty())
    }

    pub fn position(&self, entity_id: &str) -> Option<WidgetPosition> {
        self.widget_config.get(entity_id)?.position
    }

    /// Whether the primary action needs a confirmation step (#85).
    /// Off unless the user explicitly asked for it.
    pub fn require_confirm(&self, entity_id: &str) -> bool {
        self.widget_config
            .get(entity_id)
            .is_some_and(|w| w.require_confirm)
    }

    pub fn visibility(&self, entity_id: &str) -> Option<&VisibilityRule> {
        self.widget_config.get(entity_id)?.visibility.as_ref()
    }

    pub fn visibility_mut(&mut self, entity_id: &str) -> Option<&mut VisibilityRule> {
        self.widget_config.get_mut(entity_id)?.visibility.as_mut()
    }

    /// Every widget that currently has a visibility rule, as
    /// `(entity_id, rule)`.
    pub fn visibility_rules(&self) -> impl Iterator<Item = (&String, &VisibilityRule)> {
        self.widget_config
            .iter()
            .filter_map(|(id, w)| w.visibility.as_ref().map(|r| (id, r)))
    }

    /// Mutable access for writes, creating the entry on first use.
    pub fn widget_mut(&mut self, entity_id: &str) -> &mut WidgetConfig {
        self.widget_config.entry(entity_id.to_owned()).or_default()
    }

    /// Drop everything stored for a widget. The single call that used to
    /// need four (and was only ever making one of them).
    pub fn forget_widget(&mut self, entity_id: &str) {
        self.widget_config.remove(entity_id);
    }

    /// Fold the pre-consolidation parallel maps into `widget_config`.
    ///
    /// Only ever has anything to do the first time a config written by
    /// an older build is loaded; the legacy fields are never serialized,
    /// so the next save writes the new shape and this becomes a no-op.
    fn migrate_legacy_widget_maps(&mut self) {
        for (id, position) in std::mem::take(&mut self.legacy_positions) {
            self.widget_mut(&id).position = Some(position);
        }
        for (id, priority) in std::mem::take(&mut self.legacy_priorities) {
            self.widget_mut(&id).priority = priority;
        }
        for (id, name) in std::mem::take(&mut self.legacy_names) {
            self.widget_mut(&id).name = Some(name);
        }
        for (id, rule) in std::mem::take(&mut self.legacy_visibility) {
            self.widget_mut(&id).visibility = Some(rule);
        }

        // Drop state for entities that are no longer pinned. The parallel
        // maps leaked: `ToggleWidget` removed the entity from `widgets`
        // and cleaned none of them, so real configs carry position and
        // name entries for widgets deleted long ago. Migrating that
        // forward would just re-house the leak in the new shape.
        //
        // Scoped to the migration on purpose. Pruning on every load would
        // race any runtime path that records state for an entity before
        // it lands in `widgets`; here the whole file is already in hand
        // and `widgets` is authoritative.
        self.widget_config
            .retain(|entity_id, _| self.widgets.contains(entity_id));
    }

    fn project_dirs() -> Result<ProjectDirs> {
        ProjectDirs::from("dev", "snapdash", "Snapdash")
            .context("Cannot determine app config directory")
    }

    pub fn config_path() -> Result<PathBuf> {
        let proj = Self::project_dirs()?;
        Ok(proj.config_dir().join("config.json"))
    }

    pub async fn load() -> anyhow::Result<Self> {
        let path = Self::config_path()?;
        let bytes = match tokio::fs::read(&path).await {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e).with_context(|| format!("Failed to read {:?}", path)),
        };

        match serde_json::from_slice::<Self>(&bytes) {
            Ok(mut cfg) => {
                cfg.migrate_legacy_widget_maps();
                Ok(cfg)
            }
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

        // Prune widgets the user never customised so config.json doesn't
        // accumulate empty objects for every widget ever opened.
        let mut to_write = self.clone();
        to_write.widget_config.retain(|_, w| !w.is_empty());

        let json = serde_json::to_vec_pretty(&to_write).context("Failed to serialize config")?;
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

fn default_theme_name() -> String {
    DEFAULT_THEME.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget_visibility::VisibilityCondition;

    /// A config written by a pre-consolidation build must load with
    /// every per-widget setting intact.
    #[test]
    fn migrates_legacy_parallel_maps() {
        let legacy = serde_json::json!({
            "ha_url": "http://ha.local:8123",
            "ha_token_present": true,
            "widgets": ["light.kitchen", "sensor.temp"],
            "widget_positions": { "light.kitchen": { "x": 10.0, "y": 20.0 } },
            "widget_priorities": { "light.kitchen": "high" },
            "widget_names": { "sensor.temp": "Outside" },
            "widget_visibility": {
                "sensor.temp": {
                    "trigger": "sensor.temp",
                    "condition": { "kind": "is_available" }
                }
            }
        });

        let mut cfg: Config = serde_json::from_value(legacy).expect("legacy config deserializes");
        cfg.migrate_legacy_widget_maps();

        assert_eq!(
            cfg.position("light.kitchen"),
            Some(WidgetPosition { x: 10.0, y: 20.0 })
        );
        assert_eq!(cfg.priority("light.kitchen"), Priority::High);
        assert_eq!(cfg.name_override("sensor.temp"), Some("Outside"));
        assert!(cfg.visibility("sensor.temp").is_some());

        // Untouched widgets stay at their defaults.
        assert_eq!(cfg.priority("sensor.temp"), Priority::default());
        assert_eq!(cfg.name_override("light.kitchen"), None);
    }

    /// Real pre-consolidation configs carry entries for widgets the user
    /// unpinned long ago, because `ToggleWidget` dropped the entity from
    /// `widgets` and left the four maps alone. Migration must not carry
    /// that leak into the new shape.
    #[test]
    fn migration_drops_state_for_unpinned_widgets() {
        let legacy = serde_json::json!({
            "ha_url": "http://ha.local:8123",
            "ha_token_present": true,
            "widgets": ["sensor.active_power"],
            "widget_positions": {
                "sensor.active_power": { "x": 10.0, "y": 20.0 },
                "sensor.tx_137": { "x": 99.0, "y": 99.0 }
            },
            "widget_names": { "sensor.tx_137": "Long gone" }
        });

        let mut cfg: Config = serde_json::from_value(legacy).unwrap();
        cfg.migrate_legacy_widget_maps();

        assert_eq!(
            cfg.position("sensor.active_power"),
            Some(WidgetPosition { x: 10.0, y: 20.0 }),
            "pinned widget keeps its state"
        );
        assert_eq!(cfg.position("sensor.tx_137"), None);
        assert_eq!(cfg.name_override("sensor.tx_137"), None);
        assert!(!cfg.widget_config.contains_key("sensor.tx_137"));
    }

    /// The legacy maps are drained, not merely copied - a second save
    /// must not resurrect them.
    #[test]
    fn legacy_maps_are_not_written_back() {
        let legacy = serde_json::json!({
            "ha_url": "http://ha.local:8123",
            "ha_token_present": false,
            "widgets": ["light.kitchen"],
            "widget_priorities": { "light.kitchen": "low" }
        });

        let mut cfg: Config = serde_json::from_value(legacy).unwrap();
        cfg.migrate_legacy_widget_maps();

        let round_tripped = serde_json::to_value(&cfg).unwrap();
        assert!(round_tripped.get("widget_priorities").is_none());
        assert_eq!(
            round_tripped["widget_config"]["light.kitchen"]["priority"],
            serde_json::json!("low")
        );
    }

    /// Removing a widget drops all of its state, which is what the four
    /// parallel maps failed to do.
    #[test]
    fn forget_widget_drops_every_setting() {
        let mut cfg = Config::default();
        let w = cfg.widget_mut("light.kitchen");
        w.priority = Priority::High;
        w.name = Some("Kitchen".into());
        w.position = Some(WidgetPosition { x: 1.0, y: 2.0 });
        w.visibility = Some(VisibilityRule {
            trigger: "light.kitchen".into(),
            condition: VisibilityCondition::IsAvailable,
        });

        cfg.forget_widget("light.kitchen");

        assert_eq!(cfg.priority("light.kitchen"), Priority::default());
        assert_eq!(cfg.name_override("light.kitchen"), None);
        assert_eq!(cfg.position("light.kitchen"), None);
        assert!(cfg.visibility("light.kitchen").is_none());
        assert!(cfg.widget_config.is_empty());
    }

    /// The confirmation gate is off unless explicitly enabled, and
    /// survives a round trip through config.json (#85).
    #[test]
    fn require_confirm_defaults_off_and_round_trips() {
        let mut cfg = Config::default();
        assert!(!cfg.require_confirm("switch.pump"));

        cfg.widget_mut("switch.pump").require_confirm = true;
        assert!(cfg.require_confirm("switch.pump"));

        let json = serde_json::to_value(&cfg).unwrap();
        let reloaded: Config = serde_json::from_value(json).unwrap();
        assert!(reloaded.require_confirm("switch.pump"));
        assert!(!reloaded.require_confirm("switch.other"));
    }

    /// An un-customised widget serializes to nothing at all.
    #[test]
    fn default_entries_are_pruned_on_save() {
        let mut cfg = Config::default();
        cfg.widget_mut("sensor.untouched");
        cfg.widget_mut("light.kitchen").priority = Priority::High;

        let mut to_write = cfg.clone();
        to_write.widget_config.retain(|_, w| !w.is_empty());

        assert!(!to_write.widget_config.contains_key("sensor.untouched"));
        assert!(to_write.widget_config.contains_key("light.kitchen"));
    }
}
