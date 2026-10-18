#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
cargo run --locked --manifest-path "$root/crates/system-apps/Cargo.toml" -- "$root"
