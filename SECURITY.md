# Security Policy

## Supported versions

Snapdash is in early development. Only the **latest release** receives security
fixes.

| Version | Supported |
|---|---|
| Latest (main) | ✅ |
| Older | ❌ |

Once v1.0.0 ships, the support window will be formalized (latest minor + one back, etc.).

## Reporting a vulnerability

**Please do not report security vulnerabilities through public GitHub issues,
discussions, or pull requests.**

Instead, please report them via one of these channels:

1. **Preferred**: [GitHub's private vulnerability reporting](https://github.com/schizza/snapdash/security/advisories/new) (Security tab on the repo).
2. **Alternative**: Email `opensource@schizza.cz` with subject starting with `[SECURITY]`.

### What to include

- A description of the vulnerability and its potential impact.
- Steps to reproduce (proof of concept if possible).
- Affected versions / platforms.
- Your suggested severity (Low / Medium / High / Critical) — optional.
- Any mitigations or workarounds you've identified.

### What to expect

- **Acknowledgement** within 72 hours.
- **Initial assessment** within 7 days.
- **Coordinated disclosure** — we'll work with you on a fix timeline before public
disclosure.
- **Credit** in the release notes / advisory (unless you prefer anonymity).

We follow a **90-day disclosure window** by default: if a fix isn't possible within
90 days, we'll discuss extension or early disclosure with you.

## Scope

### In scope

- Snapdash binary and source code
- Home Assistant WebSocket / REST client code
- Credential storage (OS keychain integration)
- Build / release infrastructure in `.github/workflows/`

### Out of scope

- **Dependencies** — report to the upstream project (iced, tokio, etc.). If you believe a vulnerability is better mitigated in Snapdash itself, please include that reasoning.
- **Home Assistant** itself — report to [HA security](https://www.home-assistant.io/security/).
- **Physical attacks** requiring local privileged access.
- **Social engineering** of Snapdash users.

## Known issues

Current limitations (not vulnerabilities, but worth noting):

- **Binaries are not code-signed.** macOS and Windows will warn about unidentified
developer. We plan to sign releases once funding allows. Users should verify
downloads via GitHub Releases (not third-party mirrors).
- **No auto-update mechanism yet** — users must manually check for updates.
- **TLS certificate validation** is handled by `rustls` (via `reqwest` and
`tokio-tungstenite`). Standard system trust store + webpki roots. No custom cert
pinning.

## Thanks

Snapdash is a small project maintained primarily by one person. Responsible
disclosure is appreciated and credited. 🙏
