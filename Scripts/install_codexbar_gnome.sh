#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_MANIFEST="${ROOT_DIR}/linux/backend/Cargo.toml"
BACKEND_BINARY="${ROOT_DIR}/linux/backend/target/release/codexbar-gnome-backend"
LOCAL_BIN_DIR="${HOME}/.local/bin"
LOCAL_BINARY="${LOCAL_BIN_DIR}/codexbar-gnome-backend"
EXTENSION_UUID="codexbar-gnome"
EXTENSION_SOURCE_DIR="${ROOT_DIR}/linux/gnome-extension"
EXTENSION_TARGET_DIR="${HOME}/.local/share/gnome-shell/extensions/${EXTENSION_UUID}"
NEW_CONFIG_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/codexbar-gnome"
NEW_CONFIG_PATH="${NEW_CONFIG_DIR}/config.json"
LEGACY_CONFIG_PATH="${HOME}/.codexbar/config.json"

log() {
  printf '%s\n' "$*"
}

ensure_binary_tools() {
  command -v cargo >/dev/null 2>&1 || {
    echo "cargo is required to build the backend." >&2
    exit 1
  }
  command -v glib-compile-schemas >/dev/null 2>&1 || {
    echo "glib-compile-schemas is required to install the GNOME extension." >&2
    exit 1
  }
}

build_backend() {
  log "==> Building Rust backend"
  cargo build --release --manifest-path "${BACKEND_MANIFEST}"
}

install_backend() {
  log "==> Installing backend to ${LOCAL_BINARY}"
  mkdir -p "${LOCAL_BIN_DIR}"
  install -m 0755 "${BACKEND_BINARY}" "${LOCAL_BINARY}"
}

install_extension() {
  log "==> Installing GNOME extension to ${EXTENSION_TARGET_DIR}"
  mkdir -p "${EXTENSION_TARGET_DIR}"
  rm -rf "${EXTENSION_TARGET_DIR}/"*
  cp -R "${EXTENSION_SOURCE_DIR}/." "${EXTENSION_TARGET_DIR}/"
  glib-compile-schemas "${EXTENSION_TARGET_DIR}/schemas"
}

ensure_config() {
  mkdir -p "${NEW_CONFIG_DIR}"

  if [[ -f "${NEW_CONFIG_PATH}" ]]; then
    chmod 600 "${NEW_CONFIG_PATH}"
    return
  fi

  if [[ -f "${LEGACY_CONFIG_PATH}" ]]; then
    log "==> Migrating legacy config to ${NEW_CONFIG_PATH}"
    install -m 0600 "${LEGACY_CONFIG_PATH}" "${NEW_CONFIG_PATH}"
    return
  fi

  log "==> Writing starter config to ${NEW_CONFIG_PATH}"
  cat > "${NEW_CONFIG_PATH}" <<'JSON'
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
JSON
  chmod 600 "${NEW_CONFIG_PATH}"
}

print_next_steps() {
  cat <<EOF
==> Install complete

Backend:
  ${LOCAL_BINARY}

Config:
  ${NEW_CONFIG_PATH}

Extension UUID:
  ${EXTENSION_UUID}

Next steps:
  1. Enable the extension:
     gnome-extensions enable ${EXTENSION_UUID}
  2. If GNOME Shell does not pick it up immediately on Wayland, log out and back in.
  3. Open extension preferences:
     gnome-extensions prefs ${EXTENSION_UUID}
EOF
}

main() {
  ensure_binary_tools
  build_backend
  install_backend
  install_extension
  ensure_config
  print_next_steps
}

main "$@"
