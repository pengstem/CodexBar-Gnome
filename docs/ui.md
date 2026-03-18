---
summary: "Current GNOME Shell frontend goals and visual language."
read_when:
  - "Changing the extension dropdown layout or panel icon."
  - "Cleaning up old frontend reference code."
---

# UI

The active frontend is the GNOME Shell extension in `linux/gnome-extension`.

## Panel language

- Compact provider chip
- Two horizontal meters:
  - top = session window
  - bottom = weekly window
- Optional percentage text

## Dropdown card

- Provider header with badge, name, identity, and hero percentage
- Usage section with session / weekly / model-cap rows
- Codex credits section when present
- Facts section for plan, source, status, updated time
- Dedicated error banner instead of flattening errors into normal rows

## Reference policy

The old Swift UI is reference material only. If you need to borrow ideas:

- preserve the visual hierarchy
- preserve the two-bar mental model
- do not preserve the old macOS implementation structure
