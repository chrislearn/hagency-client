#!/bin/bash
# Open a persistent personal-account profile, separate from test fixtures.
set -euo pipefail
project_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$project_dir"
if [[ $(uname -s) != Darwin ]]; then
    echo 'This launcher is for macOS.' >&2
    exit 1
fi
export HAGENCY_APP_DATA_DIR="${HAGENCY_APP_DATA_DIR:-${RINX_DATA_DIR:-$HOME/Library/Application Support/Hagency Live}}"
mkdir -p "$HAGENCY_APP_DATA_DIR"
chmod 700 "$HAGENCY_APP_DATA_DIR"
# Real account credentials must never pass through the test input bridge.
unset MAKEPAD_REMOTE MAKEPAD_HIDE_WINDOWS MAKEPAD_NO_FOCUS MAKEPAD_FOCUS
cargo build --locked --features agent_chat
build_dir=$(cargo metadata --no-deps --locked --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
mkdir -p "$build_dir/matrix-account"
cp "$build_dir/debug/hagency" "$build_dir/matrix-account/hagency"
exec packaging/run-macos.sh "$build_dir/matrix-account/hagency"
