#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
# Only the desktop executable needs an ad-hoc app bundle. CLI and test
# executables keep their own command line and lifecycle.
if [[ "${1##*/}" == hagency ]]; then
    exec bash "$root/crates/hagency-app/packaging/run-macos.sh" "$@"
fi
exec "$@"
