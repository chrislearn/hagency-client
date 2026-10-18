# Hagency additions and verification

The `agent_chat` build adds long agent-reply folding, receiving-side MSC4357
streaming, shared mobile/desktop settings, and English/Simplified Chinese labels.
Mobile entry: Me → Settings → General → Hagency. Existing approval namespaces,
verdict bindings, role badges, companion bridge invites and workflow commands
remain supported. This is not an implementation of every Hagency dashboard API.

Agent Operations has a scoped runtime and native panel for attention/tasks/queue/
workspaces, cancellation, workspace inspection, and authorized outcome resolution.
The producer is pinned to `hagency-org/hagency` commit
`4a8a8ac25e43e645345fc25987261e0d7a644fcd`; its canonical fixture manifest is still
`development`, with no released source commit. The production bootstrap gate stays
closed. Explicit `agent_ops_dev` builds enable same-host development testing.
Operator device enrollment, the exact encrypted approval-room membership, a loopback
origin and an independently supplied server fingerprint are required. See
[setup and protocol notes](../../docs/agent-chat.md#scoped-agent-operations).

Verification on macOS, using isolated Palpo fixture accounts and a temporary real
Hagency backend with its runner pump stopped:

- 181 Rust library tests pass with `agent_ops_dev`; two opt-in integration tests are
  excluded from the ordinary suite. The Agent Operations integration test is run
  explicitly and passes. The normal build's library suite also passes (181).
- Real Rust-to-Hagency HTTP interoperability covers pinned signed exchange,
  proof-of-possession requests, scoped snapshots, invalidation, cancellation,
  inspection, Continue resolution, mark-inspected, duplicate mutation replay and
  authorization revocation. No runner or external agent is started.
- The pinned producer's 17 Agent Operations backend tests pass.
- Native production journeys cover folding/expansion, Unicode live edits, rich-text
  completion, unchanged Matrix message bodies, English/Chinese settings and the
  release gate, plus shared preference persistence across mobile/desktop layouts.
- Native development journeys reject non-loopback endpoints and using the project
  room as the owner room, send an actually encrypted session request through Palpo,
  and cancel the bootstrap on closing the panel.
- Default-feature compilation passes. Translation validation finds 788 matching
  catalog entries and 900 explicit translated call sites, with no missing keys.

Native input traces, actual screenshots, binary hashes and per-run logs live under
`evidence/live/hagency/` (ignored by Git). The receipt files are
`native-hagency.json` and `native-hagency-ops.json`. No personal account is driven
or inspected by these tests. These checks use mobile-size and desktop macOS
windows; they are not an iOS device test or a visual similarity score. The encrypted
Matrix bootstrap and the real backend HTTP lifecycle are tested separately; a full
bridge-mediated encrypted grant lifecycle against a released deployment remains
an integration acceptance step.

Reproduce:

```bash
cargo test --locked --features agent_chat --lib
cargo test --locked --features agent_ops_dev --lib
# In the pinned temporary Hagency checkout: npm ci && npm run build:router
ROBRIX_HAGENCY_SOURCE=/path/to/pinned/hagency ROBRIX_TEST_NODE=/path/to/node \
  cargo test --locked --features agent_ops_dev --lib \
  real_backend_signed_sessions_mutations_and_revocation -- --ignored
cargo build --locked --features agent_ops_dev
python3 tools/wechat-ux/live/native_hagency_ops.py
cargo build --locked --features agent_chat
python3 tools/wechat-ux/live/native_hagency.py
python3 tools/wechat-ux/check_i18n.py
```

The native scripts require the existing isolated test-server fixture and SSH
access configured by `tools/wechat-ux/live/provision_palpo.py`; set
`ROBRIX_TEST_SSH=user@your-isolated-test-host` when provisioning. They only create or
use `@robrix_ux_` accounts. Never point them at a personal profile or deployment.
