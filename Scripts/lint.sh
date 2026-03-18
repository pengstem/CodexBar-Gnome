#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXTENSION_DIR="${ROOT_DIR}/linux/gnome-extension"
SCHEMAS_DIR="${EXTENSION_DIR}/schemas"

cmd="${1:-lint}"

case "$cmd" in
  lint)
    python3 - <<'PY'
from pathlib import Path
import json

metadata_path = Path("linux/gnome-extension/metadata.json")
metadata = json.loads(metadata_path.read_text())
required = ["uuid", "name", "description", "version", "shell-version", "settings-schema"]
missing = [key for key in required if key not in metadata]
if missing:
    raise SystemExit(f"metadata.json is missing required keys: {', '.join(missing)}")
print("metadata.json ok")
PY
    python3 - <<'PY'
from pathlib import Path

for path in [Path("linux/gnome-extension/extension.js"), Path("linux/gnome-extension/prefs.js")]:
    text = path.read_text()
    if "CodexBarGnome" not in text:
        raise SystemExit(f"{path} does not look like the active GNOME frontend")
print("GNOME extension sources ok")
PY
    tmp_dir="$(mktemp -d)"
    cp "${SCHEMAS_DIR}"/*.xml "${tmp_dir}/"
    glib-compile-schemas "${tmp_dir}"
    rm -rf "${tmp_dir}"
    printf '%s\n' "GNOME schema ok"
    ;;
  format)
    printf '%s\n' "No formatter is configured for the Linux-only path." >&2
    ;;
  *)
    printf 'Usage: %s [lint|format]\n' "$(basename "$0")" >&2
    exit 2
    ;;
esac
