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
    // On Linux we install a custom `.style()` that clears the surface to
    // `Color::TRANSPARENT` so the shadow-margin area around the card stays
    // transparent (see `Snapdash::style` and `ui::platform`). On macOS and
    // Windows the OS-level rounded-corner hack in iced_winit plus the
    // card-filling-the-window layout make the default opaque theme clear
    // color invisible, so we keep the iced default.
    let builder = daemon(
        app::Snapdash::boot,
        app::Snapdash::update,
        app::Snapdash::view,
    )
    .subscription(app::Snapdash::subscription);

    #[cfg(target_os = "linux")]
    let builder = builder.style(app::Snapdash::style);

    builder.run()
}
