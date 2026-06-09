//! Per-widget visibility rules. A widget can be gated by another HA
//! entity's state — useful when a sensor only makes sense while some
//! device is running (e.g. show "washer time remaining" only while
//! `binary_sensor.washer_running` is `on`).
//!
//! Default: a widget without a rule is always visible. With a rule,
//! visibility = `rule.evaluate(entities)`. An unknown trigger entity
//! evaluates to `false` — hide until confirmed.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ha::types::EntityState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibilityCondition {
    StateEquals {
        value: String,
    },
    StateNotEquals {
        value: String,
    },
    /// Entity is reporting a real value — not `unknown` / `unavailable`.
    IsAvailable,
    NumericGt {
        threshold: String, // String -> easier UI binding
    },
    NumericLt {
        threshold: String,
    },
}

impl VisibilityCondition {
    /// Return the editable raw string value (if the variant has one).
    pub fn raw_value(&self) -> Option<&str> {
        match self {
            Self::StateEquals { value } | Self::StateNotEquals { value } => Some(value),
            Self::IsAvailable => None,
            Self::NumericGt { threshold } | Self::NumericLt { threshold } => Some(threshold),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisibilityRule {
    /// entity_id of the trigger. May equal the widget's own entity_id
    /// (self-triggering — common case for "show sensor X only while it's
    /// reporting something useful").
    pub trigger: String,
    pub condition: VisibilityCondition,
}

impl VisibilityRule {
    /// `true` → widget should be visible. Trigger entity not in `entities`
    /// → `false` (hide-until-known). Decoupled from `HaState` so it's
    /// trivially unit-testable.
    pub fn evaluate(&self, entities: &HashMap<String, EntityState>) -> bool {
        let Some(state) = entities.get(&self.trigger) else {
            return false;
        };
        let raw = state.state.as_str();
        match &self.condition {
            VisibilityCondition::StateEquals { value } => raw == value,
            VisibilityCondition::StateNotEquals { value } => raw != value,
            VisibilityCondition::IsAvailable => raw != "unknown" && raw != "unavailable",
            VisibilityCondition::NumericGt { threshold } => {
                let Ok(t) = threshold.parse::<f64>() else {
                    return false;
                };
                raw.parse::<f64>().map(|n| n > t).unwrap_or(false)
            }
            VisibilityCondition::NumericLt { threshold } => {
                let Ok(t) = threshold.parse::<f64>() else {
                    return false;
                };
                raw.parse::<f64>().map(|n| n < t).unwrap_or(false)
            }
        }
    }
}

/// UI-facing handle on a condition's *variant* — used by the picker in
/// the widget settings dialog so changing the condition type doesn't
/// require constructing a default variant in the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConditionKind {
    IsAvailable,
    StateEquals,
    StateNotEquals,
    NumericGt,
    NumericLt,
}

impl ConditionKind {
    pub const ALL: &[Self] = &[
        Self::IsAvailable,
        Self::StateEquals,
        Self::StateNotEquals,
        Self::NumericGt,
        Self::NumericLt,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::IsAvailable => "Is available",
            Self::StateEquals => "State equals",
            Self::StateNotEquals => "State is not",
            Self::NumericGt => "Numeric value >",
            Self::NumericLt => "Numeric value <",
        }
    }

    /// Rebuild a `VisibilityCondition` of this variant, carrying the raw
    /// string forward where applicable. Numeric ↔ text switch keeps the
    /// string (user may have to retype, but no surprise data loss).
    pub fn with_value(self, raw: String) -> VisibilityCondition {
        match self {
            Self::IsAvailable => VisibilityCondition::IsAvailable,
            Self::StateEquals => VisibilityCondition::StateEquals { value: raw },
            Self::StateNotEquals => VisibilityCondition::StateNotEquals { value: raw },
            Self::NumericGt => VisibilityCondition::NumericGt { threshold: raw },
            Self::NumericLt => VisibilityCondition::NumericLt { threshold: raw },
        }
    }

    pub fn from_condition(c: &VisibilityCondition) -> Self {
        match c {
            VisibilityCondition::IsAvailable => Self::IsAvailable,
            VisibilityCondition::StateEquals { .. } => Self::StateEquals,
            VisibilityCondition::StateNotEquals { .. } => Self::StateNotEquals,
            VisibilityCondition::NumericGt { .. } => Self::NumericGt,
            VisibilityCondition::NumericLt { .. } => Self::NumericLt,
        }
    }
}

impl std::fmt::Display for ConditionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make(entity_id: &str, state: &str) -> EntityState {
        EntityState {
            entity_id: entity_id.to_owned(),
            state: state.to_owned(),
            attributes: BTreeMap::new(),
            last_changed: None,
            last_updated: None,
        }
    }

    fn map(states: &[(&str, &str)]) -> HashMap<String, EntityState> {
        states
            .iter()
            .map(|(id, s)| ((*id).to_owned(), make(id, s)))
            .collect()
    }

    fn rule(trigger: &str, condition: VisibilityCondition) -> VisibilityRule {
        VisibilityRule {
            trigger: trigger.into(),
            condition,
        }
    }

    #[test]
    fn state_equals_matches() {
        let entities = map(&[("binary_sensor.washer", "on")]);
        let r = rule(
            "binary_sensor.washer",
            VisibilityCondition::StateEquals { value: "on".into() },
        );
        assert!(r.evaluate(&entities));
    }

    #[test]
    fn state_not_equals_matches_otherwise() {
        let entities = map(&[("light.x", "on")]);
        let r = rule(
            "light.x",
            VisibilityCondition::StateNotEquals {
                value: "off".into(),
            },
        );
        assert!(r.evaluate(&entities));
    }

    #[test]
    fn unknown_trigger_evaluates_false() {
        let entities = HashMap::new();
        let r = rule("ghost", VisibilityCondition::IsAvailable);
        assert!(!r.evaluate(&entities));
    }

    #[test]
    fn is_available_excludes_unknown_and_unavailable() {
        let r = rule("s", VisibilityCondition::IsAvailable);
        assert!(!r.evaluate(&map(&[("s", "unknown")])));
        assert!(!r.evaluate(&map(&[("s", "unavailable")])));
        assert!(r.evaluate(&map(&[("s", "42")])));
    }

    #[test]
    fn numeric_thresholds() {
        let entities = map(&[("s", "60")]);
        assert!(
            rule(
                "s",
                VisibilityCondition::NumericGt {
                    threshold: "30".into()
                }
            )
            .evaluate(&entities)
        );
        assert!(
            !rule(
                "s",
                VisibilityCondition::NumericGt {
                    threshold: "90.0".into()
                }
            )
            .evaluate(&entities)
        );
        assert!(
            rule(
                "s",
                VisibilityCondition::NumericLt {
                    threshold: "90".into()
                }
            )
            .evaluate(&entities)
        );
    }

    #[test]
    fn non_numeric_state_fails_numeric_gracefully() {
        // "running" can't parse as f64 → NumericGt evaluates to false
        // (rule hides the widget instead of crashing or flashing on).
        let entities = map(&[("s", "running")]);
        let r = rule(
            "s",
            VisibilityCondition::NumericGt {
                threshold: "0.0".into(),
            },
        );
        assert!(!r.evaluate(&entities));
    }
}
