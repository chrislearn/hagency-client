# Desktop durable knowledge

## 2026-10-13 — execution device configuration

`hagency-desktop` uses the sibling `hagency-client/native/hagency` dependency. The CI checkout pin in `.github/actions/setup-hagency-client/action.yml` must include the NativeOwner configuration wrappers whenever Desktop calls them. A local sibling build alone cannot verify that checkout pin.

NativeOwner contracts: `AgentConfigurationStatus` returns `{configuration: metadata}`; `ExecutionEnvironment` returns raw `{providers, configuration}`; `AgentConfiguration` wraps only the approved Local/Provider/RuntimeProvider/PrepareAgentWorkspace commands and returns the raw operation result. The server schema is defined in `../hagency-server/docs/design/2026-10-13-cross-device-agent-configuration.zh-CN.md` and its review.

Remote model and Agent policy edits target the assigned device; provider catalogs, references, reasoning choices and workspaces must never fall back to the viewing device. Room/requester policy, reset, provider login/logout/disconnect, service controls and approvals remain local. Metadata polls use a separate UI request fence, preserve mutation feedback, and cannot clear a save's busy state.

`local_revision_conflict` is the client policy CAS error. `invalid_local_policy` also covers target workspace canonicalization failures. `configuration_outcome_unknown` and malformed configuration responses do not prove that an attempted write failed; retain the original idempotency key. A confirmed success or terminal validation failure ends that intent, so an A → B → A model sequence creates a new final write rather than replaying the first receipt.
