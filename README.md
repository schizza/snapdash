<div align="center">

# SnapDash

**A pluggable desktop widget system - Home Assistant today, anything tomorrow.**

[![CI](https://github.com/schizza/snapdash/actions/workflows/ci.yml/badge.svg)](https://github.com/schizza/snapdash/actions/workflows/ci.yml)
  [![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
  [![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org/)
  [![Platforms](https://img.shields.io/badge/platforms-macOS_|_Windows_|_Linux-lightgrey.svg)]()

[Website](https://snapdash.schizza.cz) · [Releases](https://github.com/schizza/snapdash/releases) · [Issues](https://github.com/schizza/snapdash/issues) · [Roadmap](https://github.com/schizza/snapdash/projects)
</div>

---

Snapdash gives you a clean, always-visible snapshot of live data
— sensors, metrics, anything that streams.
Built in **Rust** for stability, performance, and 24/7 reliability, it sits quietly
on your desktop without leaks, lag, or surprises.

  Today it speaks **Home Assistant**. Tomorrow, anything you can wire up via plugins.

Built with **Rust** for stability, performance, and long-running reliability —
SnapDash is designed to run quietly in the background without leaks, lag, or surprises.

🚧 **Status: Early development / MVP** — first public release, expect rough edges.

| Widget | Settings |
|:---:|:---:|
| ![Screenshot](screenshots/settings.png) | ![Widget1](screenshots/widget_1.png) ![Widget2](screenshots/widget_2.png) |

## Features

- **Real-time** updates via Home Assistant WebSocket API
- **Frameless widgets** - pin individual sensors as floating macOS-style cards
- **Native look** - Mac Light / Mac Dark themes, smooth pulse animations on state change
- **Custom themes** - drop a JSON file in your themes folder to fully recolor the UI; share or download themes like Dracula, Nord, Catppuccin
- **Secure token storage** - credentials lives in OS keychain (macOD Keychain / Windows Credential Manager / Linux Secret Service), never in plain text
- **Cross-platform** - macOS, Windows, Linux
- **Lightweight** - low CPU / memory footprint, designed to run 24/7 in background
- **Pluggable (planned)** - Home Assistant is the first integration; plugin API for arbitrary data sources is on the roadmap
- **Fully customizable cards and layouts** - in progress
- **Sensor history and lightweight charts** - in progress

## Why Rust?

Because SnapDash is meant to be boring in the best possible way.

Rust lets us build a widget that doesn’t slowly eat memory, doesn’t spike the CPU, and doesn’t need babysitting. You start it, pin it to your desktop, and it just keeps doing its job.

## Installation

### Pre-built binaries

Grab the latest release for your platform from the [Releases page](https://github.com/schizza/snapdash/releases):

- **macOS**: `.tar.gz` (Apple Silicon) / `.dmg` will be added later
- **Windows**: `.zip` portable / `.msi` installer (will be added later)
- **Linux**: `.tar.gz` portable / `.AppImage`

⚠️ macOS binaries are **not yet code-signed**. Signing of application is in process now.
macOS will warn about an "unidentified developer" — open it via `Right-click → Open` once.

**"Snapdash is damaged and can't be opened" on macOS**

macOS blocks unsigned apps from the internet. Snapdash is not yet
signed with an Apple Developer ID - we are tracking this in
[#12](https://github.com/schizza/snapdash/issues/12).

**Remove the quarantine attributes** (one-time, recommended):

```bash
xattr -cr /Applications/Snapdash.app  # or where your Snapdash.app lives
```

Or right-click the app -> **Open** -> click **Open** in the dialog.
Either way, very the download first:

```bash
shasum -a 256 -c snapdash-vX.Y.Z-macos-aarch64.tar.gz.sha256
```

**Easier: install via Homebrew** (recommended):

```bash
brew tap schizza/tap
brew install --cask snapdash
```

Homebrew automatically removes the quarantione attributes and handles
updates via `brew upgrade`.

### Build from source

Requires **Rust 1.85** (2024 edition)

```bash
git clone https://github/schizza/snapdash.git
cd snapdash
cargo build --release

# Run directly
cargo run --release
```

## Quick start

1. Launch Snapdash - the **Settings** window opens automatically on first run.
2. Enter your Home Assistant URL (e. g. `http://localhost:8123`).
3. Paste your **Long-Lived Access Token** (see below).
4. Hit **Save** - Snapdash connects and lists your sensors.
5. Tick any sensor -> a floating widget appears for it.
6. Drag widget anywhere on screen. They remember their position.

## Getting a Home Assistant Long-Lived Token

1. Open your Home Assistant UI in a browser.
2. Click your **user profile** (avatar in bottom-left)
3. Go to **Security -> Long-Lived Access Tokens**
4. Click **Create token**, name it (e.g. `Snapdash`), confirm
5. Copy the token immediately - Home Assistant only shows it once.
6. Paste it into Snapdash Settings , confirm
7. Copy the token immediately - Home Assistant only shows it once.
8. Paste it into Snapdash Settings -> **Home Assistant token** field.

After saving, the token is moved to your OS keychain. The `config.json` file never contains the token.

If the token is compromised: delete it in HA, generate a new one, paste it into Snapdash Settings (the `🗑` button next to the token field also clears the keychain entry).

### Configuration

Snapdash uses a simple JSON config in your user profile.
If the config is corrupted, Snapdash falls back to defaults and writes a fresh file on next save.

### File locations

| OS | Config | Log |
| --- | --- | --- |
| **macOS** | `~/Library/Application Support/dev.snapdash.Snapdash/config.json` | `~/Library/Application Support/dev.snapdash.Snapdash/debug.log` |
| **Windows** | `%APPDATA%\dev.snapdash.Snapdash\config.json` | `%APPDATA%\dev.snapdash.Snapdash\debug.log` |
| **Linux** | `~/.config/snapdash/config.json` | `~/.local/share/snapdash/debug.log` |

## Custom themes

Beyond the built-in **Mac Light** and **Mac Dark**, Snapdash loads any
JSON theme you drop into its `themes/` folder. Write your own, or grab
a ready-made one (Dracula, Nord, Catppuccin, …) and place the file in:

| OS | Themes folder |
| --- | --- |
| **macOS** | `~/Library/Application Support/dev.snapdash.Snapdash/themes/` |
| **Windows** | `%APPDATA%\dev.snapdash.Snapdash\themes\` |
| **Linux** | `~/.config/snapdash/themes/` |

The folder is scanned at startup; every valid `*.json` shows up in
**Settings → Appearance → Theme**. A malformed theme is skipped (with a
warning in the log) — it never blocks the others or crashes the picker.

### Theme file structure

A theme is a single self-contained JSON file. Colors are hex strings,
either `#rrggbb` (opaque) or `#rrggbbaa` (with alpha):

```json
{
  "schema": 1,
  "name": "Dracula",
  "author": "Zeno Rocha (port)",
  "appearance": "dark",
  "palette": {
    "bg": "#1e1f29",
    "card": "#282a36",
    "card_2": "#21222c",
    "text_primary": "#f8f8f2",
    "text_secondary": "#e2e2dc",
    "text_body": "#f8f8f2",
    "text_dim": "#6272a4",
    "text_disabled": "#44475a",
    "border": "#44475a80",
    "border_hovered": "#6272a4",
    "accent": "#bd93f9",
    "accent_dim": "#a679e0",
    "accent_tint": "#bd93f926",
    "shadow": { "color": "#00000059", "offset_x": 0.0, "offset_y": 10.0, "blur_radius": 22.0 },
    "danger": "#ff5555",
    "success": "#50fa7b"
  }
}
```

**Top-level fields**

| Field | Required | Description |
| --- | --- | --- |
| `schema` | no (default `1`) | Theme format version — for forward compatibility |
| `name` | **yes** | Display name shown in the theme picker |
| `author` | no | Credited as "Name — by Author" in the picker |
| `appearance` | no (default `dark`) | `"light"` or `"dark"` — hint for grouping / OS-mode matching |
| `palette` | **yes** | The color set (all fields below are required) |

**Palette fields**

| Field | Used for |
| --- | --- |
| `bg` | Window / surface background behind cards |
| `card` | Primary card background |
| `card_2` | Secondary / nested card background |
| `text_primary` | Headings |
| `text_secondary` | Subheadings, labels |
| `text_body` | Normal body text |
| `text_dim` | Placeholders, hints, captions |
| `text_disabled` | Disabled controls |
| `border` | Default borders |
| `border_hovered` | Borders on hover |
| `accent` | Primary accent (selection, active state, links) |
| `accent_dim` | Dimmed accent variant |
| `accent_tint` | Very subtle accent fill (use low alpha) |
| `shadow` | Drop shadow: `{ color, offset_x, offset_y, blur_radius }` |
| `danger` | Errors, destructive actions |
| `success` | Connected / OK states |

Ready-made examples live in [`assets/themes/`](assets/themes/) — copy any
of them into your themes folder as a starting point, then tweak the
colors. Changing the selected theme in Settings applies instantly; no
restart needed.

## Troubleshooting

**Settings window doesn't open**
First run with no config auto-opens Settings. If it stays closed, check the log file.

**`Invalid JSON ... using defalut config` in log**
The config file got corrupted. Delete it or fix the JSON manually, then reconfigure via Settings.

**No widget windows appear despite saved entities**
Check the log for HA WebSocket errors - token expired, URL unreachable, network blocked.
Open Settings, hit **Save** again to force a reconnect.

**Token issues**
On macOS/Windows the token only lives in the keychain. To reset: in Settings, click `🗑` to clear, then paste a fresh token and save.

## Roadmap

- [X] Frameless widget windows + macOS-style theming
- [X] Home Assistant WebSocket integration with reconnect
- [X] Secure token storage in OS keychain
- [X] Real-time state updates with pulse animations
- [X] Multi-widget configuration via Settings
- [X] Custom JSON themes (downloadable / user-authored)
- [X] Cross-platform autostart
- [X] In-app auto-update
- [ ] Local history & 24h sparkline charts
- [ ] System tray menu
- [ ] Plugin API for non-HA data sources
- [ ] Linux-specific window hacks (XShape rounded corners)
- [ ] Code-signed releases (macOS notarization, Windows signing)
- [ ] Auto-update mechanism

See the [issue tracker](https://github.com/schizza/snapdash/issues) and [project board](https://github.com/schizza/snapdash/projects) for current work.

## Tech Stack

- **[Rust](https://www.rust-lang.org/)** (2024 edition) - core language
- **[Iced](https://github.com/iced-rs/iced)** (forked) - GPU-accelerated GUI via wgpu
- **[Tokio](https://github.com/tokio-rs/tokio)** - async runtime
- **[tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)** - WebSocket client
- **[reqwest](https://github.com/seanmonstar/reqwest)** - HTTP client (initial state fetch)
- **[keyring](https://github.com/hwchen/keyring-rs)** - cross-platform OS credential storage

## Contributing

Contributions welcome! Pleas read [CONTRIBUTING.md](CONTRIBUTING.MD) (TODO) and check open [issues](https://github.com/schizza/snapdash/issues) for places to start.

Bug reports and feature requests via the [issue tracker](https://github.com/schizza/snapdash/issues/new/choose).

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

See [NOTICE](NOTICE) for third-party attribution.
