use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(thiserror::Error, Debug, Clone)]
pub enum HaError {
    #[error("connection failed: {0}")]
    Connect(String),

    #[error("authentication rejected by server: {0}")]
    AuthInvalid(String),

    #[error("authentication failed after {attempts} attempts - check your token")]
    AuthExhausted { attempts: u8 },

    #[error("websocket closed")]
    Closed,

    #[error("connection went stale (no frames for {elapsed:?})")]
    Stale { elapsed: Duration },

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("timeout waiting for: {what}")]
    Timeout { what: &'static str },

    #[error("failed to send: {what}")]
    SendFailed { what: &'static str },
}

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
    Disconnected(HaError),
    InitialState(Vec<EntityState>),
    StateChanged { new_state: EntityState },
    AuthFailed(HaError),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HaConnectionConfig {
    pub url: String,
    pub token: String,
}
