# SnapDash

SnapDash is a Rust-powered cross-platform desktop widget for Home Assistant. It gives you a clean, always-visible snapshot of your sensors and entities, with real-time updates, customizable cards, and lightweight charts.

Built with **Rust** for stability, performance, and long-running reliability —
SnapDash is designed to run quietly in the background without leaks, lag, or surprises.

## Features

- Real-time updates via Home Assistant WebSocket API
- Fully customizable cards and layouts
  - in progress
- Sensor history and lightweight charts
  - in progress
- Cross-platform: macOS, Windows, Linux
- Frameless widget window
- Secure token storage (OS keychain)
- Low resource usage, safe for 24/7 operation

## Why Rust?

Because SnapDash is meant to be boring in the best possible way.

Rust lets us build a widget that doesn’t slowly eat memory, doesn’t spike the CPU, and doesn’t need babysitting. You start it, pin it to your desktop, and it just keeps doing its job.

## Status

🚧 Early development / MVP stage

## Roadmap

- [X] Widget window + basic card layout
- [X] Home Assistant authentication & entity picker
- [X] Real-time updates (WebSocket)
- [ ] Multiple cards support
- [ ] Local history & 24h charts
- [ ] Tray menu & autostart

## Tech Stack

- Rust (core, Home Assistant client, data handling)
- Iced with custom hacks

## Getting your Home Assistant Long-Lived Access Token

SnapDash uses a **long-lived access token** to talk to your Home Assistant instance.
You only need to create it once and paste it into the Settings window.

1. Open your Home Assistant UI in a browser.
2. Click your **user profile**:
   - bottom-left corner (your username / avatar),
   - or go to **Settings → People → [Your user]** and open the profile.
3. Scroll down to the section **Long-Lived Access Tokens**.
4. Click **Create Token**.
5. Enter any name, e.g. `SnapDash`, and confirm.
6. Home Assistant will show you the token **once**:
   - copy it immediately,
   - paste it into the **Home Assistant token** field in SnapDash Settings.
7. After saving:
   - SnapDash stores the token in the OS keychain (not in `config.json`),
   - you can safely close the window; you don’t need to see the token again.

If you lose the token or suspect it was compromised:

- go back to **Long-Lived Access Tokens** in your HA profile,
- **delete** the old token,
- create a new one and update it in SnapDash Settings.

## Configuration

SnapDash using simple  JSON config that is stored in your user profile.
If configiguration is broken, application will start with defauls and write new config file.

### Where is `config.json`?

**macOS**

```text
  ~/Library/Application Support/dev.snapdash.Snapdash/config.json
```

**Windows**

```text
%APPDATA%\dev.snapdash.Snapdash\config.json

or

C:\Users\<username>\AppData\Roaming\dev.snapdash.Snapdash\config.json
```

**Linux**

```text
~/.config/snapdash/config.json
```

### Where is my log file?

**macOS**

```text
  ~/Library/Application Support/dev.snapdash.Snapdash/debug.log
```

**Windows**

```text
%APPDATA%\dev.snapdash.Snapdash\debug.log

or:

C:\Users\<username>\AppData\Roaming\dev.snapdash.Snapdash\debug.log
```

**Linux**

```text
~/.config/snapdash/debug.log

or

~/.local/share/snapdash/debug.log
```

## Where to look when things go wrong

**Config issues**

- Check `config.json` in the paths listed above.
- If SnapDash reports “Invalid JSON… Using default config”, the file is corrupted;
  delete it or fix the JSON syntax and reconfigure via Settings.

- **Token / authentication**
  - On macOS/Windows the token is stored only in the OS keychain; if connection fails:
    - remove and re-save the token in Settings,
    - or clear the `snapdash` / `Snapdash` entry in the Keychain / Credential Manager.

- **No widget windows visible**
  - On the very first run without a config:
    - SnapDash loads a default config and should automatically open the Settings window.
  - If you have an entity in `widgets` but no window appears:
    - check the log and any errors from the HA WebSocket.

## License

MIT
