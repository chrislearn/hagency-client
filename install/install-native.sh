#!/bin/sh
# Per-user OwnerHost installer. The release binary embeds its owner console.
# Pasion personal login and explicit owner Agent start happen after installation.
set -eu
usage() {
  echo "usage: $0 --install-dir DIR --state-dir DIR [--listen 127.0.0.1:13300] [--no-open]" >&2
}
INSTALL_DIR=""
STATE_DIR=""
LISTEN="127.0.0.1:13300"
NO_OPEN=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --install-dir|--state-dir|--listen)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      case "$1" in
        --install-dir) INSTALL_DIR=$2 ;;
        --state-dir) STATE_DIR=$2 ;;
        --listen) LISTEN=$2 ;;
      esac
      shift 2 ;;
    --no-open) NO_OPEN=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "refused: unknown argument $1" >&2; usage; exit 2 ;;
  esac
done
[ -n "$INSTALL_DIR" ] && [ -n "$STATE_DIR" ] || { usage; exit 2; }
BIN="$INSTALL_DIR/hagency"
[ -x "$BIN" ] || { echo "refused: missing release binary" >&2; exit 1; }
# The binary validates the fresh/current owner format and refuses legacy state
# before installing anything. It preserves current state and never imports it.
# Linux uses systemd --user; macOS uses the installing user's LaunchAgent.
if [ "$NO_OPEN" -eq 1 ]; then
  exec "$BIN" service install --state-dir "$STATE_DIR" --listen "$LISTEN" --no-open
else
  exec "$BIN" service install --state-dir "$STATE_DIR" --listen "$LISTEN"
fi
