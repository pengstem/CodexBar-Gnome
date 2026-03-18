---
summary: "Linux config file layout for CodexBar-Gnome."
read_when:
  - "Editing provider enablement or source settings."
  - "Changing config migration or defaults."
---

# Configuration

CodexBar-Gnome reads a single JSON config file:

- `~/.config/codexbar-gnome/config.json`

On first install, the backend copies forward a legacy file from:

- `~/.codexbar/config.json`

Permissions should remain `0600`.

## Supported shape

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

## Active fields

- `id`: currently `codex` or `claude`
- `enabled`: whether the provider should appear in the backend payload
- `source`: currently only `oauth` and `auto` are supported by the Linux backend

## Notes

- `auto` is treated as OAuth-only on the Linux path today.
- Unsupported legacy fields are ignored by the current backend.
- Keep the file minimal; do not store unrelated app-era settings here.
