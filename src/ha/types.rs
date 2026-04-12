use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub entity_id: String,
    pub state: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, Value>,

    #[serde(default)]
    pub last_changed: Option<String>,
    #[serde(default)]
    pub last_updated: Option<String>,
}

#[derive(Clone, Debug)]
pub enum HaEvent {
    Connected,
    Disconnected(String),
    InitialState(Vec<EntityState>),
    StateChanged { new_state: EntityState },
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HaConnectionConfig {
    pub url: String,
    pub token: String,
}
