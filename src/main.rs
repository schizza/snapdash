#![windows_subsystem = "windows"]

mod app;
mod config;
mod ha;
mod logger;
mod secrets;
mod theme;
mod ui;

use iced::daemon;

fn main() -> iced::Result {
    daemon(
        app::Snapdash::boot,
        app::Snapdash::update,
        app::Snapdash::view,
    )
    .style(app::Snapdash::style)
    .subscription(app::Snapdash::subscription)
    .run()
}
