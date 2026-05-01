pub mod app;
pub mod config;
pub mod ha;
pub mod logger;
pub mod theme;
pub mod ui;
pub mod update;
pub mod widget_size;

use iced::daemon;

/// Inter Variable — body / UI font. SIL OFL, see assets/fonts/Inter-LICENSE.txt.
/// Bundled so every platform renders the same neogrotesk shapes (closest
/// open-source substitute for SF Pro). Avoids OS-default font drift between
/// macOS (San Francisco), Windows (Segoe UI), and Linux (whatever fontconfig
/// resolves).
const INTER_VARIABLE: &[u8] = include_bytes!("../assets/fonts/InterVariable.ttf");

/// Lucide icon font. ISC, see assets/fonts/Lucide-LICENSE.txt. Used for the
/// gear/settings icon (and any future icons) so we don't fall through to the
/// per-OS Symbols/Emoji font which renders ⚙ as a colored emoji on Windows
/// and as outlines on macOS/Linux.
pub const LUCIDE_FONT: &[u8] = include_bytes!("../assets/fonts/lucide.ttf");

/// Build and run the Snapdash daemon.
///
/// On Linux we install a custom `.style()` that clears the surface to
/// `Color::TRANSPARENT` so the shadow-margin area around the card stays
/// transparent (see `Snapdash::style` and `ui::platform`). On macOS/Windows
/// the native rounded-corner/shadow path clips the card-filling window before
/// any cleared pixel is visible, so we keep the iced default.
pub fn run() -> iced::Result {
    let _gurad = logger::init();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "snapdash starting");

    let builder = daemon(
        app::Snapdash::boot,
        app::Snapdash::update,
        app::Snapdash::view,
    )
    .subscription(app::Snapdash::subscription)
    .default_font(iced::Font::with_name("Inter Variable"))
    .font(INTER_VARIABLE)
    .font(LUCIDE_FONT);

    #[cfg(target_os = "linux")]
    let builder = builder.style(app::Snapdash::style);

    builder.run()
}
