//! Window-management types and helpers. A Snapdash session can open
//! several windows (settings, per-entity widgets, release notes) and they
//! are addressed by `window::Id` from iced; `WindowKind` tags what each
//! id is for so the view layer can dispatch correctly.

use std::collections::HashMap;

use iced::window;

use crate::ha::EntityState;

#[derive(Debug, Clone, PartialEq)]
pub enum WindowKind {
    Settings,
    Entity { entity_id: String },
    ReleaseNotes,
}

#[derive(Debug, Clone)]
pub struct WindowState {
    pub kind: WindowKind,
    pub entity: EntityWindowState,
}

#[derive(Debug, Default, Clone)]
pub struct EntityWindowState {
    pub entity_id: String,
    pub last: Option<EntityState>,
    pub pulse: f32, // TODO: Replace with Animation/spring. Currently just easy "animation paramter" (0..1), později nahradit Animation/spring
    pub hovered: bool,
}

/// Look up the window id for a given `kind`, optionally matching on the
/// inner entity id (used by `WindowKind::Entity`). Returns `None` when
/// the window isn't currently open — callers use this to decide between
/// focusing the existing window and spawning a new one.
pub fn find_window_id(
    windows: &HashMap<window::Id, WindowState>,
    kind: WindowKind,
    name: Option<&str>,
) -> Option<window::Id> {
    windows
        .iter()
        .find(|(_, v)| {
            if v.kind != kind {
                return false;
            }

            match name {
                None => true,
                Some(exp) => exp == v.entity.entity_id,
            }
        })
        .map(|(&id, _)| id)
}
