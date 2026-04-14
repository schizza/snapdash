# Contributing to Snapdash

Thanks for your interest in contributing! 🎉
This document explains the project workflow, conventions, and
how to get your first PR merged smoothly.

## Table of contents

- [Code of Conduct](#code-of-conduct)
- [Getting started](#getting-started)
- [Branching model](#branching-model)
- [Commit convention](#commit-convention)
- [Coding style](#coding-style)
- [Submitting a PR](#submitting-a-pr)
- [Reporting bugs](#reporting-bugs)
- [Security issues](#security-issues)

## Code of Conduct

This project adheres to the [Contributor Covenant Code code-of-conduct](CODE_OF_CONDUCT.md). By participating, you agree to
uphold its principles. Report unacceptable behavior to <opensource@schizza.cz>.

## Getting started

### Prerequisites

- **Rust 1.85+** (2024 edition) — install via [rustup](https://rustup.rs/)
- **Git**
- **Platform toolchain**:
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Windows: Visual Studio Build Tools (C++ workload)
  - Linux: `build-essential`, `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev` (package names vary by distro)

### Clone and build

```bash
git clone https://github.com/schizza/snapdash.git
cd snapdash

# Debug build (fast compile, unoptimized)
cargo build
cargo run

# Release build (slow compile, optimized)
cargo build --release
``

The first build downloads ~50 dependencies and the forked iced
  from https://github.com/schizza/iced. Subsequent builds are incremental.

Running checks locally

Before pushing, always run:
```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo build --release
```

CI runs the same checks on every PR. Run them locally to avoid
  PR round-trips.

## Branching model

Snapdash uses a two-branch workflow:

- main — stable, tagged releases only. Direct commits forbidden.
- dev — integration branch. All PRs target here.
- Feature / fix branches — short-lived, one per PR:
  - feat/sparkline-charts
  - fix/macos-dragging-jitter
  - docs/readme-cleanup
  - chore/bump-tokio
  - refactor/settings-tabs

### Typical lifecycle

feat/my-feature  →  PR  →  dev  →  (eventual release PR)  → main  →  tag v0.x.y

PRs to main are allowed only for:

- release/* branches (release preparation)
- hotfix/* branches (critical fixes shipped between normal releases)

### Commit convention

Snapdash follows <https://www.conventionalcommits.org/>. Format:

<type>[optional scope]: <short description>

[optional longer body]

[optional footer(s)]

Examples:

```
feat: add sparkline chart to entity widget
fix(ha-ws): reconnect after Home Assistant restart
docs: clarify token setup in README
chore: bump tokio to 1.49
refactor(ui): extract icon button styling
feat(ui)!: redesign settings window layout
```

### Types used

┌──────────┬──────────────────────────────────────┐
│   Type   │               Purpose                │
├──────────┼──────────────────────────────────────┤
│ feat     │ New user-visible feature             │
├──────────┼──────────────────────────────────────┤
│ fix      │ Bug fix                              │
├──────────┼──────────────────────────────────────┤
│ docs     │ Documentation only                   │
├──────────┼──────────────────────────────────────┤
│ refactor │ Code change without behavior change  │
├──────────┼──────────────────────────────────────┤
│ perf     │ Performance improvement              │
├──────────┼──────────────────────────────────────┤
│ test     │ Tests only                           │
├──────────┼──────────────────────────────────────┤
│ chore    │ Tooling, CI, deps, no source changes │
└──────────┴──────────────────────────────────────┘

## Breaking changes

Use ! after type/scope or a BREAKING CHANGE: footer:

feat(config)!: rename `ha_url` to `homeassistant_url`

**BREAKING CHANGE: existing config.json files must be migrated
manually.**

This automatically gets the breaking-change label and triggers
  a major version bump on release.

### Why this matters

The commit type is parsed by:

- release-drafter — categorizes changes in release notes
- auto-labeler — applies PR labels automatically
- version-resolver — determines major/minor/patch bump

Sloppy commit messages mean sloppy release notes.

## Coding style

### Rust style

- cargo fmt uses default rustfmt config (no custom rustfmt.toml).
- Follow idiomatic Rust. Clippy lints are enforced (-D warnings in CI).
- Avoid unwrap() in library code. Use ?, expect("reason"), or pattern match.
- Prefer Result<T, E> over panics for recoverable errors.
- Small PRs > big PRs. If your change spans multiple concerns, split it.

## Comments

- Write comments only when the why is non-obvious. Don't comment what the code does — well-named identifiers show that.
- Use /// doc comments for public items (pub fn, pub struct, ...).
- Prefer English in comments (the codebase is public).

## Error handling

- Internal errors use `anyhow::Result<T>` with `.context(...)` for context.
- User-facing errors flow through `set_status(..., LogType::Error)` and log to `debug.log`.
- Never silently swallow errors `(.ok() / let _ = ...)` unless deliberate and obvious.

## Async code

- One runtime: tokio with multi_thread flavor.
- Don't block the UI thread. Long-running work → `Task::perform(async {...}, Message::...)`.
- Don't create ad-hoc `tokio::spawn` from inside an iced update handler — use Task instead.

## Submitting a PR

1. Fork the repo and clone your fork.
2. Create a branch from dev: `git checkout -b feat/my-thing dev`.
3. Make your changes. Keep commits logically atomic.
4. Run checks: `cargo fmt && cargo clippy -- -D warnings && cargo build --release`.
5. Push and open a PR targeting `dev` **(not main)**.
6. Fill in the PR template — it's there for a reason.
7. Respond to review — maintainers will tag you.
   Address comments in the same branch (new commits, don't force-push
   unless necessary).
8. Squash merge — we squash-merge PRs into dev to keep history
  linear. Your commit title becomes the squashed commit
  message, so make it descriptive.

## First-time contributors

Look for issues labeled <https://github.com/schzza/snapdash/issues?q=is%3Aopen+is%3Aissue+label%3Agood-first-issue>.
These are scoped, well-defined, and good entry points.

## Reporting bugs

Use the `.github/ISSUE_TEMPLATE/bug_report.yml`. Include:

- OS + version
- Snapdash version
- Steps to reproduce
- Relevant debug.log excerpts (anonymized)

## Security issues

Do not open public issues for security vulnerabilities.
See `SECURITY.md` for responsible disclosure process.

## Questions?

- General questions → <https://github.com/schizza/snapdash/discussions>
- Bug / feature → <https://github.com/schizza/snapdash/issues>
- Security → <opensource@schizza.cz>

Thanks again for contributing! 🙌
