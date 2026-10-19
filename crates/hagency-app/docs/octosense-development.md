# OctoSense development snapshot

The public fork is [OctoSense-org/robrix2](https://github.com/OctoSense-org/robrix2),
forked from [Project-Robius-China/robrix2](https://github.com/Project-Robius-China/robrix2).
The development branch is `dev/wechat-octoscript-miniapps`, based on local
`ui/robrix2-polish` commit `7c98054a` plus the current reviewed source, fixtures,
test tools and UX specifications. The fork's `main` retains the published
upstream state.

The snapshot includes the WeChat-style four tabs and navigation, room/Space
discovery, chat actions and forwarding, per-room search and attachments,
Matrix sign-in, Chinese/English catalogs, Apple PingFang integration, Moments,
web mini-app cards, and the current Hagency work. The production Hagency
operations gate remains closed; `agent_ops_dev` enables the development path.

Native account captures, credentials, session stores and local runtime evidence
are excluded by the existing lab ignore rules. Reference images in `lab/wechat-ux/source`
are generated references and Makepad WeChat mock captures with provenance and
the mock project's license. Historical validation summaries describe earlier
runs; they do not imply that those private evidence files are in this fork.

The original working checkout and index were left intact. Test provisioning in
this fork takes `ROBRIX_TEST_SSH` or `seed.py --ssh` instead of a developer's
hardcoded SSH address. Reusing an existing fixture does not require SSH.

## Validation on this snapshot

- `cargo test --offline --locked --features agent_chat --lib`: 181 passed,
  2 ignored (the explicit live Matrix and external Hagency backend tests).
- `cargo test --offline --locked --features agent_ops_dev --lib --quiet`:
  181 passed, the same 2 integration tests ignored.
- `python3 -m unittest discover -s tools/wechat-ux -p 'test_*.py'`: 26 passed.
- `python3 tools/wechat-ux/check_i18n.py`: 788 catalog entries, 900 translated
  call sites, no missing entries.
- Python syntax checks for the two adjusted fixture scripts and `git diff --check`
  passed. Reviewed credential-pattern matches were test placeholders,
  translations and runtime interpolation; no private evidence was staged.

These are development checks, not fresh native visual acceptance or an iOS
device qualification. The two ignored integration tests were not rerun while
creating this fork.

## Mini-app security design

[ADR 0002](adr/0002-octoscript-mini-app-authority.md) records the reviewed
OctoSense ROM, App-Hub, Octoscript and published Makepad enforcement revisions.
It proposes account/instance-bound operations, host-owned credentials and an
offline Markdown/HTML editor with trusted publication confirmation. It includes
negative acceptance cases and the distinction between native account delegation
and a separate Matrix OpenID identity proof for an app's backend.
It also specifies received-card authentication: each recipient verifies the app,
continues with their own Matrix account, approves its permissions and, when
needed, signs into the app backend. Sender grants and sessions never transfer.

The built-in L0 article editor slice is now implemented. See
[the article-editor flow and validation](../lab/article-editor/README.md) for
source admission, recipient consent, native Markdown/Html rendering and Matrix
publishing. The broader arbitrary-app catalog/runtime and external backend
identity exchange remain proposed. The snapshot counts above describe the
initial fork; current article-editor test evidence is recorded with that flow.

The article studio v2 adds native formatted editing, images, themes, cover art,
full review and Matrix publication revisions/withdrawal. Its generated UX,
measured scene references, host bindings and validation are tracked in
[the v2 delivery](../lab/article-editor-v2/README.md). Work remains on `wechat-ui`;
this delivery does not merge `main`.
