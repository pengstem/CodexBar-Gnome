---
summary: "Codex provider behavior on the Linux GNOME path."
read_when:
  - "Changing Codex auth or usage fetching."
  - "Debugging Codex provider payloads."
---

# Codex

The Linux backend uses local Codex auth and the live usage API.

## Auth source

- Primary: `~/.codex/auth.json`
- Token preference:
  - `tokens.access_token`
  - fallback: `OPENAI_API_KEY`
- If present, `tokens.account_id` is sent as `ChatGPT-Account-Id`.

## Usage endpoint

- Default: `https://chatgpt.com/backend-api/wham/usage`
- If `~/.codex/config.toml` defines `chatgpt_base_url`, the backend respects it.

## Output mapping

- `rate_limit.primary_window` → session bar
- `rate_limit.secondary_window` → weekly bar
- `credits.balance` → credits section
- `plan_type` or JWT plan fields → login method / plan label

## Current Linux behavior

- OAuth-backed only
- No browser cookie import
- No old PTY fallback in the active Linux runtime path
