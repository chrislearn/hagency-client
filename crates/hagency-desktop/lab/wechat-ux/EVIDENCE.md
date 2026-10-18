# Evidence contract

`tools/wechat-ux/ux.py gate` is an evidence validator. Native capture, interaction
execution, server assertions and visual judgment must happen before it is run.
It neither implements the screens nor assigns its own similarity scores.

All file paths below are relative to the directory containing `acceptance.json`,
including paths inside nested inspection/journey JSON. Absolute paths and paths
escaping that directory are rejected. Hashes are SHA-256 of exact bytes. Keep
reference images, native images, raw logs and JSON records together in a new
immutable evidence directory per run. `evidence/` is ignored by Git.

## Acceptance receipt

The following is a schema illustration, **not passing evidence**. Expand it to
every screen and journey in `flow-catalog.json` for both `en` and `cn`. A missing
entry, duplicate entry, old source hash, mismatched locale, incomplete review,
reference masquerading as native output, or missing required trace fails.

```json
{
  "schema_version": 1,
  "claim": "generated-target",
  "execution_mode": "fixture",
  "fixture": "morgan-alice-v1",
  "catalog_sha256": "<ux.py fingerprint>",
  "api_map_sha256": "<ux.py fingerprint>",
  "source_sha256": "<ux.py fingerprint>",
  "screens": [{
    "screen": "chats",
    "locale": "en",
    "pixels": [812, 1552],
    "reference": {
      "path": "chats/en/reference.png",
      "sha256": "<sha256>",
      "kind": "generated_target",
      "locale": "en"
    },
    "native": {"path": "chats/en/native.png", "sha256": "<sha256>"},
    "inspection": {"path": "chats/en/inspection.json", "sha256": "<sha256>"},
    "review": {
      "reviewer": "<person or independent visual reviewer>",
      "notes": "<observed differences and why they satisfy the target>",
      "reference_sha256": "<same reference hash>",
      "native_sha256": "<same native hash>",
      "design_match": null,
      "verdict": "pending",
      "criteria": {
        "layout": false, "typography": false, "colors": false,
        "icons": false, "density": false, "states": false
      }
    }
  }],
  "journeys": [{
    "journey": "chat-send", "locale": "en",
    "evidence": {"path": "journeys/chat-send-en.json", "sha256": "<sha256>"}
  }]
}
```

Allowed reference kinds are `generated_target`, `makepad_wechat_capture`, and
`wechat_capture`. The `wechat` claim permits only the last and additionally
requires nonempty `app_version`, `os_version` and `device` on each reference.
Those are caller-declared provenance; preserve the original capture receipt.

`generated-target` can compare with the generated atlas or the open-source mock,
but it must never be described as measured parity with a shipping WeChat version.
Fixture mode permits mocked data/side effects as requested. It still requires
actual native user input and behavior evidence. Live mode requires real backend
effects and an API map with no incomplete capabilities.

Use identical fixture, locale, canvas dimensions, scale, safe area and keyboard
conditions for reference/candidate pairs. Save any original-to-comparison crop
transform separately. The gate never silently scales a candidate to match.

## Native inspection

An instrumented Robrix runner supplies the following, referencing **raw evidence**
from the same build/run. `snapshot`, `tree`, `layout`, `input_trace` and
`build_receipt` each use the `{path,sha256}` file-record shape shown above.

```json
{
  "screen": "chats", "locale": "en",
  "run_id": "<unique native run/Studio request identity>",
  "source_sha256": "<current source fingerprint>",
  "native_sha256": "<screenshot hash>",
  "input_method": "native",
  "snapshot": {"path": "chats/en/snapshot.json", "sha256": "<sha256>"},
  "tree": {"path": "chats/en/tree.json", "sha256": "<sha256>"},
  "layout": {"path": "chats/en/layout.json", "sha256": "<sha256>"},
  "input_trace": {"path": "chats/en/input.jsonl", "sha256": "<sha256>"},
  "build_receipt": {"path": "build.json", "sha256": "<sha256>"},
  "checks": {
    "native_controls": false, "no_screen_raster": false,
    "no_clipping": false, "text_legible": false,
    "enabled_state": false, "hit_targets": false
  }
}
```

The build receipt must contain the same `run_id` and `source_sha256`, plus the
64-character `binary_sha256` of the executable launched for that run. Record the
Makepad/SDK/compiler revisions and build command alongside those fields. The
runner must establish the association between that binary and the inspected
process. A bridge screenshot plus a user-supplied binary path cannot attest this.

Derive each native check from inspection and input:

- `native_controls`: actual named widget children in tree/snapshot, not just host.
- `no_screen_raster`: native text/controls; only declared isolated artwork is raster.
- `no_clipping`: visible/clipped bounds, keyboard and safe-area regions agree.
- `text_legible`: no unresolved glyphs, overflow or accidental truncation.
- `enabled_state`: disabled controls cannot activate; selected state matches route.
- `hit_targets`: input at actual queried bounds reaches the correct control,
  without another invisible element covering it.

Keep upstream AppCard gate reports when evaluating Kit components, but obtain
fresh Robrix inspection after integration. The JSON schema does not authenticate
the truth of a declared boolean; raw evidence needs review. Do not hand-fill true
values merely to satisfy this validator.

## Journey trace

```json
{
  "journey": "chat-send", "locale": "en",
  "run_id": "<same run as build receipt>",
  "source_sha256": "<current source fingerprint>",
  "input_method": "native",
  "mock_backend": true,
  "visited_states": ["chats", "conversation", "compose", "sending", "conversation"],
  "input_trace": {"path": "journeys/send-input.jsonl", "sha256": "<sha256>"},
  "assertion_log": {"path": "journeys/send-assertions.jsonl", "sha256": "<sha256>"},
  "backend_log": {"path": "journeys/send-fixture-effects.jsonl", "sha256": "<sha256>"},
  "build_receipt": {"path": "build.json", "sha256": "<sha256>"},
  "checks": {
    "expected_outcome": false, "back_state": false,
    "error_paths": false, "no_duplicate_effects": false
  }
}
```

`visited_states` must equal that journey's catalog sequence. Additional branches
and measurements belong in the input/assertion logs. Each locale needs its own
trace. Use `mock_backend: false` for live mode and record server-confirmed effects.
Do not include access tokens, passwords or real personal content in the logs.

For sending/retry/upload/DM/invite/profile changes, record request identity,
room/user/event identity, expected result, observed backend result and duplicate
count. A clicked button or success-looking bubble is not server confirmation.
Pure navigation journeys can omit backend logs; other journeys require them.
Fixture logs explicitly record simulated effects and remain fixture evidence.

For `layout-input`, include each required viewport, safe-area insets, orientation,
keyboard mode, focused widget, composer bounds, timeline anchor and outcome.
Exercise CJK IME composition and emoji graphemes, not just English `type_text`.
For long lists, record before/after anchor event ID and offset around pagination
and new incoming messages. Performance should include frame/input timings on the
tested device, without claiming parity on devices that were not measured.

## Review rubric and repair

| Score | Interpretation |
| --- | --- |
| 10 | Indistinguishable for the agreed fixture/scope except approved identity differences |
| 9 | Same hierarchy, density, geometry, type rhythm, icon language and state affordances; only minor documented differences |
| 8 | Noticeable row/spacing/typography/icon/layout differences, even if functional |
| <8 | Different visual structure, missing states, clipped controls or large drift |

Record each visible difference, affected native ID and proposed repair. Use
upstream pixel/ink/geometry diagnostics as supporting measurements. A global
SSIM/MAE score can miss a broken small send button and cannot certify UX.
Functional failures cannot be traded against attractive screenshots.

Rebuild, recapture and re-review changed source. Never reuse a review against a
new native screenshot. The gate binds catalog, API map, app source, screenshot,
inspection and review hashes. It deliberately returns no numeric score while
any required evidence is missing or failed.

The starting pack has no native acceptance receipt. Its saved failing result is
expected. Passing the Python unit tests only demonstrates properties of the
validator; it says nothing about Robrix's visual similarity.
