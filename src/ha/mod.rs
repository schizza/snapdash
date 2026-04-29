pub mod rest;
pub mod token;
pub mod types;
pub mod ws;

use std::collections::HashMap;

pub use types::{EntityState, HaConnectionConfig, HaEvent};

#[derive(Debug, Default)]
pub struct HaState {
    pub connected: bool,
    pub connection: Option<HaConnectionConfig>,
    pub token_draft: String,
    pub entities: HashMap<String, EntityState>,
}
