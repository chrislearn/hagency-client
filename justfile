set positional-arguments

default:
    @just --list

# Install the static console's build dependencies.
init-dev:
    npm ci --prefix mockup

# Build from source and watch Rust/console changes; private state lives in .run/.
dev *args:
    node native/scripts/dev.mjs "$@"

# Obtain a fresh local console sign-in link. Do not redirect it into logs.
console state=".run/dev-state" listen="127.0.0.1:13300":
    target/debug/hagency console-access --state-dir "$1" --listen "$2"
