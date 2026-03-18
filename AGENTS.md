# Repository Guidelines

## Project Structure
- `linux/backend`: Rust backend for live provider polling, config loading, and JSON payload generation.
- `linux/gnome-extension`: GNOME Shell frontend. Treat this as the primary user-facing surface.
- `Scripts/install_codexbar_gnome.sh`: local install/update path into `~/.local/bin` and the GNOME extensions dir.
- `docs`: Linux-only operational docs. Keep them short and current.
- `Sources/CodexBar` and `Sources/CodexBarCore`: temporary reference material from the old app. Do not treat them as the active product path.

## Build, Test, Run
- Backend tests: `cargo test --manifest-path linux/backend/Cargo.toml`
- Backend build: `cargo build --release --manifest-path linux/backend/Cargo.toml`
- Repo check: `pnpm check`
- Install/reload locally:
  - `./Scripts/install_codexbar_gnome.sh`
  - `gnome-extensions disable codexbar-gnome && gnome-extensions enable codexbar-gnome`
- Live backend probe:
  - `~/.local/bin/codexbar-gnome-backend poll --provider codex --pretty`

## Coding Style
- Prefer small, explicit Rust and JS changes over framework-heavy abstractions.
- Keep GNOME extension code direct and readable; avoid unnecessary indirection.
- Treat the old Swift code as reference only. If you borrow behavior from it, translate the behavior, not the structure.

## Testing Guidelines
- Always run `cargo test --manifest-path linux/backend/Cargo.toml` after backend changes.
- Always run `pnpm check` before handoff.
- When changing GNOME schema or extension metadata, ensure schema compilation still succeeds.
- When changing installed extension behavior, verify the extension becomes active after reload and that GNOME Shell logs stay clean.

## Cleanup Policy
- The active repo direction is Linux GNOME-first.
- Remove dead macOS/release/upstream maintenance code rather than preserving unused workflows.
- Keep only the minimum old Swift/macOS files still needed as frontend or behavior reference material.

## Agent Notes
- Do not reintroduce macOS-first scripts, Sparkle flows, or `.app` packaging assumptions.
- Keep provider data siloed: never mix identity/plan fields between providers.
- Codex is the primary supported provider right now; Claude is optional and should degrade cleanly when local credentials are absent.
