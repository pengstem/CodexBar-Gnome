---
summary: "Status-page polling behavior for the Linux backend."
read_when:
  - "Changing status checks or handling degraded network behavior."
---

# Status

The Linux backend polls provider status pages separately from usage.

Current URLs:

- Codex: `https://status.openai.com/api/v2/status.json`
- Claude: `https://status.claude.com/api/v2/status.json`

## Behavior

- Status failures must not force the provider into an overall error state if live usage succeeds.
- When the status fetch fails, the payload should return:
  - `indicator: "unknown"`
  - a short degraded description
  - the public status URL

This keeps the GNOME UI usable even when the status endpoint is flaky.
