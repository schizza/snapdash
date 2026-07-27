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
    ThemeGallery,
    WidgetSettings { entity_id: String },
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
    /// When this widget's confirmation gate was armed, or `None` when it
    /// is not awaiting a confirmation (#85).
    ///
    /// Carries the instant rather than a flag because being armed has a
    /// bounded life: the card shows the prompt instead of the value, and
    /// a widget that stopped mirroring the house indefinitely is the one
    /// failure a dashboard must not have.
    ///
    /// Deliberately runtime-only, never persisted. An armed widget that
    /// survived a restart would be a loaded gun with no visible cause.
    pub armed_at: Option<std::time::Instant>,
    /// Set while the widget is grown out of its size preset to show its
    /// continuous controls (#87), `None` when it sits at preset size.
    pub expansion: Option<Expansion>,
    /// Swallows exactly one `Moved` event, for when Snapdash moved the
    /// window itself rather than the user dragging it.
    ///
    /// Expanding near the bottom of the screen lifts the window, and
    /// collapsing puts it back. Neither is the user repositioning their
    /// widget, so neither may be written to config.json: without this a
    /// widget would wander up the screen every time it was opened.
    pub ignore_next_move: bool,
}

/// How long an armed widget waits before disarming itself.
pub const ARM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// A widget grown out of its size preset to reveal its continuous
/// controls, and what it takes to put it back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Expansion {
    /// Extra height the controls occupy.
    pub grown_by: f32,
    /// How far the window was moved up to make room, `0.0` when there
    /// was room to grow downwards. Collapsing gives this back, so the
    /// widget ends up exactly where the user left it.
    pub lifted_by: f32,
}

impl EntityWindowState {
    /// `true` when this widget has been waiting for a confirmation
    /// longer than [`ARM_TIMEOUT`], so it should go back to showing its
    /// value. Never true for a widget that is not armed.
    pub fn arm_expired(&self, now: std::time::Instant) -> bool {
        self.armed_at
            .is_some_and(|at| now.duration_since(at) >= ARM_TIMEOUT)
    }

    pub fn is_expanded(&self) -> bool {
        self.expansion.is_some()
    }
}

/// Vertical room left for the Dock, the taskbar or a panel.
///
/// A guess, not a measurement: the fork's `monitor_size` reports the
/// monitor's full resolution rather than its work area, so there is
/// nothing better to subtract. Tracked in #90 together with the
/// multi-monitor origin, which the same call also drops.
const EDGE_RESERVE: f32 = 80.0;

/// Decide how a widget at `origin` grows by `grown_by` pixels without
/// running off the bottom of the screen.
///
/// Returns how far the window has to be lifted first: `0.0` when it can
/// simply grow downwards. The lift never exceeds `origin.y`, so a widget
/// near the top of a short screen grows down and overflows rather than
/// being pushed off the top edge, where it could not be dragged back.
///
/// `monitor` is `None` when the platform would not say, in which case
/// the widget grows downwards and the compositor decides what that
/// looks like.
pub fn lift_needed(origin_y: f32, base_height: f32, grown_by: f32, monitor: Option<f32>) -> f32 {
    let Some(monitor_height) = monitor else {
        return 0.0;
    };

    let bottom_limit = monitor_height - EDGE_RESERVE;
    let overflow = (origin_y + base_height + grown_by) - bottom_limit;

    overflow.clamp(0.0, origin_y.max(0.0))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn widget(armed_at: Option<Instant>) -> EntityWindowState {
        EntityWindowState {
            entity_id: "switch.pump".into(),
            armed_at,
            ..Default::default()
        }
    }

    /// Room below: the widget grows downwards and stays put.
    #[test]
    fn a_widget_with_room_below_does_not_move() {
        assert_eq!(lift_needed(200.0, 110.0, 90.0, Some(1080.0)), 0.0);
    }

    /// Near the bottom edge it has to come up, and by exactly as much as
    /// it would otherwise overflow, not by its whole growth.
    #[test]
    fn a_widget_near_the_bottom_is_lifted_by_the_overflow() {
        // 1080 tall, 80 reserved for the Dock, so the usable bottom is
        // 1000. A 110-tall widget at y=940 already ends at 1050 and
        // growing 90 more would put it at 1140, so it overflows by 140.
        assert_eq!(lift_needed(940.0, 110.0, 90.0, Some(1080.0)), 140.0);
    }

    /// The lift must never push the title row off the top, because a
    /// widget whose drag handle is off-screen cannot be brought back.
    #[test]
    fn the_lift_never_pushes_a_widget_off_the_top() {
        // Barely any room above, and far too little below.
        let lift = lift_needed(20.0, 110.0, 200.0, Some(300.0));
        assert_eq!(lift, 20.0, "capped at the distance to the top edge");
    }

    /// No monitor size means no basis for a decision, so grow downwards
    /// and let the compositor deal with it rather than guessing.
    #[test]
    fn an_unknown_monitor_grows_downwards() {
        assert_eq!(lift_needed(940.0, 110.0, 90.0, None), 0.0);
    }

    #[test]
    fn a_widget_that_is_not_armed_never_expires() {
        assert!(!widget(None).arm_expired(Instant::now()));
    }

    #[test]
    fn arming_survives_its_window_and_ends_after_it() {
        let now = Instant::now();
        let w = widget(Some(now));

        assert!(!w.arm_expired(now), "just armed");
        assert!(
            !w.arm_expired(now + ARM_TIMEOUT - std::time::Duration::from_millis(1)),
            "still inside the window, the user may be deciding"
        );
        // The user armed the widget and walked away. Nothing else will
        // ever answer the prompt, and until it clears the card is showing
        // a question instead of the state of the house.
        assert!(w.arm_expired(now + ARM_TIMEOUT));
    }
}
