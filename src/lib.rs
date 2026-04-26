pub mod app;
pub mod config;
pub mod ha;
pub mod logger;
pub mod secrets;
pub mod theme;
pub mod ui;
pub mod update;

use iced::daemon;

/// Build and run the Snapdash daemon.
///
/// On Linux and Windows we install a custom `.style()` that clears the
/// surface to `Color::TRANSPARENT` so the shadow-margin area around the
/// card stays transparent (see `Snapdash::style` and `ui::platform`).
/// On macOS the OS-level rounded-corner hack in iced_winit plus the
/// card-filling-the-window layout make the default opaque theme clear
/// color invisible, so we keep the iced default.
pub fn run() -> iced::Result {
    let _gurad = logger::init();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "snapdash starting");

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
