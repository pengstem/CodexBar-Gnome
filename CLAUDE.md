# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

CodexBar-Gnome is a GNOME Shell top-bar usage monitor for AI coding tools (Codex/Claude). It shows remaining API capacity via panel indicators and dropdown menus.

Two runtime components:
- **Rust backend** (`linux/backend/`) -- CLI that polls provider usage APIs, reads local OAuth credentials, and outputs a JSON payload to stdout. HTTP is delegated to an embedded Python3 urllib script (no Rust HTTP crate).
- **GNOME Shell extension** (`linux/gnome-extension/`) -- GJS/Gtk4/libadwaita panel indicator that spawns the backend on a timer, parses JSON, and renders usage bars. Targets GNOME 47-49.

`Sources/` contains legacy macOS Swift code kept as behavioral reference only -- never treat it as the active product.

## Build, Test, Run

```bash
# Build backend (release)
cargo build --release --manifest-path linux/backend/Cargo.toml

# Run backend tests
cargo test --manifest-path linux/backend/Cargo.toml

# Lint (validates extension metadata, schema compilation)
./Scripts/lint.sh lint

# Full check (tests + lint)
pnpm check

# Install backend + extension locally
./Scripts/install_codexbar_gnome.sh

# Reload extension after changes
gnome-extensions disable codexbar-gnome && gnome-extensions enable codexbar-gnome

# Probe backend manually
~/.local/bin/codexbar-gnome-backend poll --provider codex --pretty
```

All `pnpm` scripts are defined in `package.json` as thin wrappers: `pnpm build`, `pnpm check`, `pnpm lint`, `pnpm start` (install), `pnpm backend:poll`.

## Architecture

### Backend data flow
1. Reads config from `~/.config/codexbar-gnome/config.json` (auto-migrates from legacy `~/.codexbar/`)
2. Reads OAuth credentials: Codex from `~/.codex/auth.json`, Claude from `~/.claude/.credentials.json`
3. Polls usage APIs (`chatgpt.com/backend-api/wham/usage`, `api.anthropic.com/api/oauth/usage`)
4. Polls status pages (`status.openai.com`, `status.claude.com`)
5. Emits `AppPayload` JSON to stdout

### Backend source layout
- `main.rs` -- CLI entry (poll/watch/config-path subcommands)
- `config.rs` -- config file loading + legacy migration
- `payload.rs` -- JSON output types (`AppPayload`, `ProviderSnapshot`)
- `status.rs` -- status page polling
- `util.rs` -- HTTP via python3 subprocess, JWT decode, helpers
- `providers/mod.rs` -- provider registry + dispatch
- `providers/codex.rs` -- Codex OAuth usage fetcher
- `providers/claude.rs` -- Claude OAuth usage fetcher

### Extension source layout
- `extension.js` -- panel button, dropdown menu, backend process management
- `prefs.js` -- preferences window (Adw/Gtk4)
- `stylesheet.css` -- dark-theme card styling
- `schemas/` -- GSettings schema XML (5 keys: backend-path, default-provider, refresh-interval, show-percentage, keep-last-good-value)

## Conventions

- Keep provider data siloed -- never mix identity/plan fields between Codex and Claude.
- Codex is the primary provider; Claude is optional and must degrade cleanly when credentials are absent.
- Prefer small, explicit changes over framework-heavy abstractions.
- Always run `cargo test` after backend changes and `pnpm check` before handoff.
- When changing GNOME schema or metadata, verify schema compilation succeeds.
- No Rust HTTP library -- HTTP goes through python3 subprocess (see `util.rs:http_get`).
- Rust edition 2021, dependencies: `base64`, `serde`, `serde_json` only.

## CI

CI (`ci.yml`) currently runs legacy Swift tests on macOS and builds a Swift CLI on Linux. It does **not** run `cargo test` or GNOME lint -- those must be run locally via `pnpm check`.
