#![windows_subsystem = "windows"]

mod app;
mod config;
mod ha;
mod logger;
mod secrets;
mod theme;
mod ui;

use iced::{daemon, time, window};
use std::time::Duration;

fn main() -> iced::Result {
    daemon(
        app::Snapdash::boot,
        app::Snapdash::update,
        app::Snapdash::view,
    )
    .subscription(|_state: &app::Snapdash| {
        iced::Subscription::batch([
            window::open_events().map(app::Message::WindowActuallyOpened),
            window::close_events().map(app::Message::WindowClosed),
            time::every(Duration::from_millis(100)).map(app::Message::Tick),
        ])
    })
    .run()
}


