//! Window-management types and helpers. A Snapdash session can open
//! several windows (settings, per-entity widgets, release notes) and they
//! are addressed by `window::Id` from iced; `WindowKind` tags what each
//! id is for so the view layer can dispatch correctly.

use std::collections::HashMap;

use iced::window;

use crate::ha::EntityState;

#[derive(Debug, Clone)]
pub struct PulseSpring {
    value: f32,
    velocity: f32,
    last_tick: Option<iced::time::Instant>,
}

impl Default for PulseSpring {
    fn default() -> Self {
        Self {
            value: 0.0,
            velocity: 0.0,
            last_tick: None,
        }
    }
}

impl PulseSpring {
    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn trigger(&mut self) {
        self.value = 1.0;
        self.velocity = 0.0;
        self.last_tick = None;
    }

    pub fn is_animating(&self) -> bool {
        self.value > 0.001 || self.velocity.abs() > 0.001
    }

    pub fn tick(&mut self, now: iced::time::Instant) {
        let dt = self
            .last_tick
            .map(|last| (now - last).as_secs_f32())
            .unwrap_or(1.0 / 60.0)
            .clamp(0.0, 0.05);

        self.last_tick = Some(now);

        let stiffness = 90.0;
        let damping = 18.0;
        let acceleration = -stiffness * self.value - damping * self.velocity;

        self.velocity += acceleration * dt;
        self.value += self.velocity * dt;

        if self.value <= 0.001 && self.velocity.abs() <= 0.01 {
            self.value = 0.0;
            self.velocity = 0.0;
            self.last_tick = None;
        }
    }
}

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
    pub pulse: PulseSpring, // TODO: Replace with Animation/spring. Currently just easy "animation paramter" (0..1), později nahradit Animation/spring
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
