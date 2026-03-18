---
summary: "CodexBar-Gnome Linux backend + GNOME Shell extension setup."
read_when:
  - "Installing or updating the Linux GNOME top bar fork."
  - "Troubleshooting the Rust backend or GNOME extension."
---

# CodexBar-Gnome

CodexBar-Gnome is the Linux GNOME top bar fork of CodexBar. It ships two pieces:

- `linux/backend`: Rust backend that reads local auth files and calls the official Codex/Claude OAuth usage APIs.
- `linux/gnome-extension`: GNOME Shell extension that renders the current provider in the top bar.

## Requirements

- GNOME Shell 47+
- `cargo`
- `glib-compile-schemas`
- Logged-in Codex CLI (`~/.codex/auth.json`) and/or Claude CLI (`~/.claude/.credentials.json`)

## Install

```bash
./Scripts/install_codexbar_gnome.sh
gnome-extensions enable codexbar-gnome
```

If you are on Wayland and the extension does not appear immediately, log out and back in.

## Config

Primary config path:

- `~/.config/codexbar-gnome/config.json`

Legacy fallback path:

- `~/.codexbar/config.json`

The installer copies the legacy file to the new path on first install. If neither file exists, it writes a starter
config with Codex and Claude enabled.

Minimal example:

```json
{
  "version": 1,
  "providers": [
    {
      "id": "codex",
      "enabled": true,
      "source": "oauth"
    },
    {
      "id": "claude",
      "enabled": true,
      "source": "oauth"
    }
  ]
}
```

## Backend CLI

```bash
~/.local/bin/codexbar-gnome-backend poll --provider all --pretty
~/.local/bin/codexbar-gnome-backend watch --provider codex --interval 120 --pretty
~/.local/bin/codexbar-gnome-backend config-path
```

## v1 Scope

- Providers: `codex`, `claude`
- Sources: `oauth` and `auto` (mapped to OAuth-only on Linux)
- No browser cookie import
- No Sparkle/Homebrew/macOS signing flow
- No Swift build requirement for the Linux path
