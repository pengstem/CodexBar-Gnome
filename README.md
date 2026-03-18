# CodexBar-Gnome

CodexBar-Gnome is a Linux GNOME top bar usage monitor for AI coding tools. The active product path is:

- `linux/backend`: Rust backend that reads local auth/config and polls provider APIs
- `linux/gnome-extension`: GNOME Shell extension UI
- `Scripts/install_codexbar_gnome.sh`: local install/update path

Current focus:

- GNOME Shell frontend
- Codex as the primary supported provider
- Claude support when local CLI credentials exist

## Install

```bash
./Scripts/install_codexbar_gnome.sh
gnome-extensions enable codexbar-gnome
```

If GNOME Shell does not pick the extension up immediately on Wayland, log out and back in once.

## Config

Primary config file:

- `~/.config/codexbar-gnome/config.json`

Legacy config is copied forward automatically from:

- `~/.codexbar/config.json`

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
      "enabled": false,
      "source": "oauth"
    }
  ]
}
```

## Development

Backend:

```bash
cargo test --manifest-path linux/backend/Cargo.toml
cargo build --release --manifest-path linux/backend/Cargo.toml
~/.local/bin/codexbar-gnome-backend poll --provider all --pretty
```

Repo checks:

```bash
pnpm check
```

## Notes

- This repo is being actively collapsed into a Linux GNOME-first codebase.
- Only `Sources/CodexBar` and `Sources/CodexBarCore` remain as temporary reference material while GNOME parity lands.
