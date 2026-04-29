use std::time::Duration;

use iced::{Subscription, Task, window};

use crate::config::Config;
use crate::update;

use super::{FocusDirection, Message, Snapdash};

impl Snapdash {
    pub fn boot() -> (Self, Task<Message>) {
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
        let redraw_events_needed = self.windows.values().any(|win| win.entity.pulse > 0.0);

        let redraw_events = if redraw_events_needed {
            iced::event::listen_raw(|event, _status, id| match event {
                iced::Event::Window(window::Event::RedrawRequested(_)) => {
                    Some(Message::WindowRedraw(id))
                }
                _ => None,
            })
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
            redraw_events,
            move_events,
            keyboard_events,
            ha,
            check_for_update,
        ])
    }
}

