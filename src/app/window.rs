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
