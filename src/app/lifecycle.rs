//! Iced runtime wiring: how `Snapdash` boots and what it subscribes to.
//! Kept separate from `snapdash.rs` (the message-handling core) so each
//! file has one job and `lib.rs`'s `daemon(...)` plumbing has an obvious
//! home.

use std::time::Duration;

use iced::{Subscription, Task, window};

use crate::config::Config;
use crate::update;

use super::{FocusDirection, Message, Snapdash};

impl Snapdash {
    /// Initial state plus the tasks that must run before the first frame:
    /// load the on-disk config and fire the first GitHub release check in
    /// parallel.
    pub fn boot() -> (Self, Task<Message>) {
        crate::update::installer::cleanup_stale_artefacts();

        let state = Self::new();

        let load_task = Task::perform(
            async { Config::load().await.map_err(|e| e.to_string()) },
            Message::ConfigLoad,
        );

        let check_update = Task::perform(update::get_latest_version(), Message::LastVersionChecked);

        let tasks = Task::batch([load_task, check_update]);

        (state, tasks)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // RedrawRequested fires on every frame, so we only subscribe while
        // an entity is animating its pulse. Otherwise we'd burn CPU on
        // idle widgets.

        let animation_frames = if self
            .windows
            .values()
            .any(|win| win.entity.pulse.is_animating())
        {
            iced::time::every(Duration::from_millis(16)).map(Message::AnimationFrame)
        } else {
            Subscription::none()
        };

        let move_events = iced::event::listen_raw(|event, _status, id| match event {
            iced::Event::Window(window::Event::Moved(point)) => Some(Message::WidgetMoved {
                id,
                position: point,
            }),
            _ => None,
        });

        // The WS subscription is keyed on the connection config, so
        // mutating `self.ha.connection` (e.g. token change) tears the old
        // stream down and starts a new one. `None` keeps it idle.
        let ha = if let Some(connection) = &self.ha.connection {
            Subscription::run_with(connection.clone(), crate::ha::ws::connect).map(Message::HaEvent)
        } else {
            Subscription::none()
        };

        let keyboard_events = iced::event::listen_raw(|event, _status, id| {
            let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
                modifiers,
                ..
            }) = event
            else {
                return None;
            };

            let direction = if modifiers.shift() {
                FocusDirection::Previous
            } else {
                FocusDirection::Next
            };

            Some(Message::FocusMove {
                window_id: id,
                direction,
            })
        });

        let check_for_update =
            iced::time::every(Duration::from_hours(1)).map(|_| Message::CheckForUpdate);

        Subscription::batch([
            window::close_events().map(Message::WindowClosed),
            animation_frames,
            move_events,
            keyboard_events,
            ha,
            check_for_update,
        ])
    }
}
