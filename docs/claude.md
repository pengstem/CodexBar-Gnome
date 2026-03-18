---
summary: "Claude provider behavior on the Linux GNOME path."
read_when:
  - "Changing Claude auth or usage fetching."
  - "Debugging Claude provider payloads."
---

# Claude

The Linux backend currently supports Claude through local OAuth credentials only.

## Auth source

- `~/.claude/.credentials.json`

The backend requires:

- `claudeAiOauth.accessToken`
- `claudeAiOauth.scopes` containing `user:profile`

## Usage endpoint

- `https://api.anthropic.com/api/oauth/usage`

Headers:

- `Authorization: Bearer <token>`
- `anthropic-beta: oauth-2025-04-20`
- `User-Agent: claude-code/2.1.0`

## Output mapping

- `five_hour` → session bar
- `seven_day` → weekly bar
- `seven_day_sonnet` or `seven_day_opus` → model cap row
- `extra_usage` → credits-style balance section when enabled

## Current Linux behavior

- OAuth-backed only
- No browser cookie import
- If local credentials are missing, disable Claude in config instead of leaving it noisy
