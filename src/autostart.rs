//! Cross-platform autostart preference wiring. Asks the OS to launch
//! Snapdash at user login.
//!
//! - macOS: writes a LaunchAgent plist under ~/Library/LaunchAgents.
//!   First registration may surface in System Settings → Login Items
//!   pending user approval. After that the agent persists.
//! - Linux: drops a `.desktop` file in ~/.config/autostart per the XDG
//!   spec. Works on GNOME, KDE Plasma, XFCE, Cinnamon. Tiling WMs
//!   (sway, i3) need a separate xdg-autostart-launcher to honour it.
//! - Windows: writes a string value to
//!   HKCU\Software\Microsoft\Windows\CurrentVersion\Run.

use std::path::PathBuf;

use anyhow::{Context, Result};
use auto_launch::AutoLaunchBuilder;

const APP_NAME: &str = "snapdash";

/// Builds an AutoLaunch handle for the running binary. Resolves the
/// path that should be registered with the OS — the .app bundle on
/// macOS when present, otherwise the raw binary.
fn handle() -> Result<auto_launch::AutoLaunch> {
    let app_path = registration_path()?;
    let path_string = app_path
        .to_str()
        .with_context(|| format!("non-UTF8 app path: {}", app_path.display()))?;

    AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(path_string)
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .build()
        .context("build AutoLaunch handler")
}

/// Path the OS should launch.
///
/// On macOS, if we're running from inside a `.app` bundle, register the
/// bundle (so OS launches `open Snapdash.app` correctly). Otherwise
/// (dev `cargo run`, sideloaded binary) register `current_exe()`.
fn registration_path() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    if let Some(bundle) = crate::update::installer::detect_app_bundle() {
        return Ok(bundle);
    }

    std::env::current_exe().context("resolve current_exe")
}

pub fn enable() -> Result<()> {
    handle()?.enable().context("enable autostart")
}

pub fn disable() -> Result<()> {
    handle()?.disable().context("disable autostart")
}

/// Whether the OS currently has a registration for our app. Best-effort:
/// any error from the OS layer (missing config dir, malformed plist)
/// counts as "not enabled" so callers don't panic on edge cases.
pub fn is_enabled() -> bool {
    handle()
        .and_then(|h| h.is_enabled().context("query OS"))
        .unwrap_or(false)
}

/// Reconciles the user's stored preference with the OS-level
/// registration. Called once after config loads — if the user wants
/// autostart but the OS has forgotten about us (manual deletion of the
/// plist/.desktop, app moved between filesystems), re-register
/// silently. Logs but doesn't fail boot on errors.
pub fn validate_state(want_enabled: bool) {
    let os_enabled = is_enabled();

    if want_enabled && !os_enabled {
        match enable() {
            Ok(_) => tracing::info!("autostart re-registered after drift"),
            Err(e) => tracing::warn!(error = %e, "autostart re-register failed"),
        }
    } else if !want_enabled && os_enabled {
        // User stored "off" but OS still has us listed — cleanup so
        // we don't silently autostart against the user's preference.
        match disable() {
            Ok(_) => tracing::info!("autostart cleared (config says off)"),
            Err(e) => tracing::warn!(error = %e, "autostart clear failed"),
        }
    }
}
