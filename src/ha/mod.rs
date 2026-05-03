//! Home Assistant integration: WebSocket connection, REST helpers, secure
//! token storage, and the runtime state the rest of the app reads from.

pub mod rest;
pub mod token;
pub mod types;
pub mod ws;

use std::collections::HashMap;

pub use types::{EntityState, HaConnectionConfig, HaEvent};

/// Runtime state for the HA integration. Owned by `Snapdash` as a single
/// field so connection, draft input, and the entity store stay grouped.
#[derive(Debug, Default)]
pub struct HaState {
    pub connected: bool,
    /// `Some` when a connection task should be running; `None` keeps the
    /// subscription idle. Mutating this drives the WS subscription on/off.
    pub connection: Option<HaConnectionConfig>,
    /// Token entered in Settings but not yet persisted to the keychain.
    pub token_draft: String,
    /// Sets from ws.rs if coonection is refused by HA
    pub auth_failed: bool,
    pub entities: HashMap<String, EntityState>,
}
