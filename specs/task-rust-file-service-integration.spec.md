spec: task
name: "Connect development FileService to the real Bootstrap MCP and encrypted Matrix publisher"
inherits: project
satisfies: [REQ-RUST-MIGRATION-EXECUTION, REQ-THREAD-SCOPED-SESSIONS, REQ-MATRIX-DM-PRIVACY, REQ-THREE-LAYER-COMPLETION]
tags: [active, rust, files, bootstrap, mcp]
---

## Intent

Implement the ADR101 application slice accepted after root and independent review.
This contract is design-only at creation. Its selectors are planned executable
acceptance, not tests that currently exist or passed. Do not add them to main's
migration inventory before implementation and actual strict lifecycle evidence.
Depend on integrated ADR096, ADR097, ADR098 and ADR100 without replacing their
current or historical authority checks.

## Constraints

### Must
- Opt the fixed TaskMcp enabled-tools and actual helper catalog into the same two file tools only through the private host profile while keeping backend authority independent.
- Reuse the existing fixed private development profile and the original single Collector DomainStore and post-Started WorkspaceAccess.
- Reuse existing RelativeFile project identifier and FileDeliveryRequest validation for source fields and byte bounds.
- Initialize the fresh SDK only through actual current Collector refresh then run bounded historical resume and media readiness before compatible claim or child launch.
- Keep one attempt per start and require file worker readiness plus original workspace registration before the existing launch ACK.
- Admit at most two retained file jobs across queue active and unknown states with one fixed source worker and no per-request blocking task.
- Allow durable admission while another job awaits network but report Unknown and retain ownership if blocked synchronous disk work prevents response within the existing five-second client deadline.
- Validate current original capability binding root and byte limit after queues before capture and again after source copy.
- Return queued only after durable file admission and retain the first preparation independently of caller lifetime.
- Bind exact source selection metadata snapshot hash stage operation receipt and frozen encrypted destination without exposing private fields.
- Preserve actual Snapshot Encrypted Prepared Restored upload and publication custody through asynchronous waits and caller loss.
- Require qualified staging and exact one-shot upload and file publication authority with current revalidation at existing SDK and HTTP boundaries.
- Distinguish historical exact-capability GET authorization from current send_file authentication and never reuse historical authority for source or sends.
- Preserve Unknown for lost admission incomplete staging possible writes missing owners and corrupt recovery without rearming or changing keys.
- Use actual native serve native MCP real local TLS and encrypted recipient fixtures without injecting capability Started registration or SQL transport availability.
- Keep original shutdown owners until acknowledged release and production capabilities false.

### Must Not
- Do not create another configuration credential source SDK engine Collector or mutable source authority registry.
- Do not accept runtime roots routes URLs keys descriptors MIME approval assertions or capability fields in tool arguments.
- Do not exceed four MiB or the configured lower source limit or widen existing MCP writer execution network or shutdown deadlines.
- Do not recapture reencrypt upload or publish automatically after replay unknown outcome or restart.
- Do not infer source custody from paths inferred delivery from upload acceptance or canonical Done from file status.
- Do not make GET send retry reconcile repeatedly expose private metadata or authorize another operation.
- Do not treat Windows unqualified durability known refusal skipped tests or unimplemented selectors as positive delivery evidence.

## Boundaries

### Allowed Changes
- ./Cargo.lock
- crates/hagency/Cargo.toml
- crates/hagency-execution/src/host.rs
- crates/hagency-runtime/src/codex/session/task_mcp.rs
- crates/hagency/src/lib.rs
- crates/hagency/src/bootstrap.rs
- crates/hagency/src/bootstrap/config.rs
- crates/hagency/src/bootstrap/driver.rs
- crates/hagency/src/bootstrap/workspace.rs
- crates/hagency/src/file_service.rs
- crates/hagency/src/file_service/types.rs
- crates/hagency/src/file_service/worker.rs
- crates/hagency/src/file_service/job.rs
- crates/hagency/src/file_service/pipeline.rs
- crates/hagency/src/file_service/recovery.rs
- crates/hagency/src/runner.rs
- crates/hagency/src/runner/files.rs
- crates/hagency/src/task_client.rs
- crates/hagency/src/task_client/files.rs
- crates/hagency/src/task_client/transport.rs
- crates/hagency/src/mcp.rs
- crates/hagency/src/mcp/catalog.rs
- crates/hagency/src/mcp/file_catalog.rs
- crates/hagency/tests/file_service.rs
- crates/hagency/tests/file_service/fixture.rs
- crates/hagency/tests/file_service/admission.rs
- crates/hagency/tests/file_service/scope.rs
- crates/hagency/tests/file_service/recovery.rs
- crates/hagency/tests/file_service/shutdown.rs
- crates/hagency/tests/fixtures/file_mcp_peer.rs
- crates/hagency/tests/bootstrap/fixture.rs
- crates/hagency/tests/mcp.rs
- crates/hagency/tests/task_client/mcp.rs
- crates/hagency/tests/http.rs
- knowledge/decisions/adr-101-native-file-service-integration.md
- specs/task-rust-file-service-integration.spec.md
- docs/progress.md
- docs/agent-knowledge.md

### Forbidden
- Existing JavaScript live services core domain schema media codec journals Matrix SDK transport runtime sandbox and guardian implementation are outside this application contract.
- Expanding an underlying API requires a separately reviewed prerequisite or an explicit contract amendment before source edits.

## Acceptance Criteria

<!-- lint-ack: bdd-rule-grouping — Independent integration and original-process observation scenarios retain one explicit application slice. -->

Scenario: Actual native bootstrap and first MCP file call deliver to the original encrypted conversation
  Level: integration
  Test Double: real native executable MCP fixed offline runtime peer local TLS and encrypted recipient SDK
  Targets: crates/hagency/src/file_service.rs
  Retired-Test: native_file_service_executable
  Given legitimate queued canonical work and the one fixed private development profile with files enabled
  When real serve authenticates Collector claims work registers the retained workspace and launches the pinned peer
  Then its first native MCP send_file reaches the same App writer and Collector after registration
  And actual decrypted group-thread and null-root DM file events preserve sender relation metadata and original bytes
  And get_file_delivery becomes delivered only after the actual Matrix event acknowledgement while canonical task remains not Done
  And platforms lacking qualified staging explicitly refuse before POST with separate negative qualification evidence

Scenario: Durable admission and caller loss preserve one original bounded job
  Level: integration
  Test Double: real writer source file and local TLS with controlled response and capture checkpoints
  Targets: crates/hagency/src/file_service.rs
  Test: native_file_service_admission
  Given two available service slots and an active pipeline waiting on real HTTP
  When a second request arrives or an HTTP caller disappears before or after durable admission
  Then the original job and first preparation survive and queued is returned only after acknowledged reservation
  And the second admission can complete during network wait without waiting for publication
  And blocked synchronous capture may cause caller Unknown while the actual worker retains custody and later records its original outcome

Scenario: Current source authority and historical status authority cannot be exchanged
  Level: integration
  Test Double: actual runner HTTP headers original writer retained root and legitimate retirement mutations
  Targets: crates/hagency/src/file_service.rs
  Test: native_file_service_authority
  Given current original expired retired foreign and same-localpart impostor credentials
  When send_file and exact get_file_delivery are called through the actual routes
  Then only a current original Started workspace can admit source capture and external publication
  And an original historical credential can read only its own safe receipt without passing current execution authentication
  And traversal links oversized changed sources stale queue scope and private-room promotion refuse before their next external effect

Scenario: Stable request replay and finite retained capacity do not duplicate capture or effects
  Level: integration
  Test Double: real source mutation two retained uncertain operations and counted TLS requests
  Targets: crates/hagency/src/file_service.rs
  Test: native_file_service_replay_bounds
  Given an admitted original call and two retained active or unknown slots
  When the source changes exact and conflicting requests replay or a third new request arrives
  Then exact replay retains the original bytes or reports missing original custody as unknown and changed content conflicts
  And completed safe job removal leaves exact replay answerable through the permanent domain receipt without new capture
  And a third request cannot bypass the job journal or domain bounds by caller cancellation or reopening an owner
  And no replay issues a replacement preparation or duplicate POST or PUT

Scenario: Stage and SDK recovery preserve incomplete facts across actual process death
  Level: integration
  Test Double: actual protected journals SDK receipt local TLS and a fresh native service process
  Targets: crates/hagency/src/file_service.rs
  Retired-Test: native_file_service_restart
  Given original committed source metadata and private SDK Complete with first domain Delivered deliberately not committed
  When every original process owner is gone and real startup opens the same protected state
  Then bounded historical resume commits the original Delivered without old claim Send or another POST or PUT
  And an actual provisioned original task-client context may read only that historical result
  And missing corrupt partial directory-unconfirmed and Possible journals remain unavailable or unknown without replacement initialization

Scenario: Cancellation and lost network evidence keep original uncertainty
  Level: integration
  Test Double: actual paused binary POST encrypted PUT and real negative room observations
  Targets: crates/hagency/src/file_service.rs
  Retired-Test: native_file_service_uncertainty
  Given a current pipeline before and after each possible external write boundary
  When cancellation retirement deadline expiry or a truncated acknowledgement occurs
  Then known pre-effect refusal stays distinct from possible-write uncertainty
  And retained original operations survive caller future loss and no automatic unknown retry occurs
  And a null-root private operation never appears in a promoted group

Scenario: Borrowed close retains the only physical SDK media and process owners
  Level: integration
  Test Double: actual Bootstrap service worker owned runtime journal locks and controlled pending operations
  Targets: crates/hagency/src/file_service.rs
  Test: native_file_service_shutdown
  Given admitting blocked active and unresolved jobs with the original execution Report
  When shutdown quiesces admission and retires workspace access
  Then new work refuses and close acknowledges only after both file and execution owners have qualified release
  And unknown source publication process or writer outcomes keep actual retained owners and locks without a second Collector
  And a consumed SDK-close timeout is sticky unknown and never becomes success through another empty-owner close

Scenario: MCP and default service boundaries remain explicit and bounded
  Level: integration
  Test Double: actual native MCP SDK client and real loopback HTTP malformed requests
  Targets: crates/hagency/src/file_service.rs
  Test: native_file_service_protocol
  Given disabled and enabled development configurations and closed tool schemas
  When unknown duplicate oversized or authority-bearing fields and valid file operations are submitted
  Then arguments remain selection-only and safe fixed results omit keys routes paths captions and underlying errors
  And the existing helper context request IDs frame limits and deadlines remain unchanged
  And default serve remains passive with production capability flags false

Scenario: The safe result names which unknown it is
  Test: native_file_service_protocol_missing_failure_stays_unknown
  Given receipt-derived and local-custody projections of the same delivery
  When either produces FileStatus OutcomeUnknown
  Then the fixed error code names its producer within the closed two-code set and any invented code is refused
  And inspect and replay still report the receipt-derived base code because they rebuild through the durable receipt

Scenario: Original executable failures retain bounded child evidence
  Level: integration
  Test Double: actual native children with valid and refused private configuration
  Targets: crates/hagency/tests/file_service/fixture.rs
  Retired-Test: native_file_service_original_observation
  Given the original executable fixture and its own child process
  When an HTTP wait or assertion fails before cleanup or the child actually exits
  Then fixed variant phase request-count and child-state evidence survives before original panic cleanup
  And only bounded fixed stderr categories are printed without payload credentials identifiers or changed deadlines
  And observation cannot report an exit before the actual original child exits
  And stderr follows the original retained read handle across path replacement and output failure cannot replace the original panic

## Decisions

Before implementation parse and lint this proposed contract and review the exact
interface manifest. Change to accepted ADR and active task only when that review
permits code. Run all nine real selectors with complete cross-crate prerequisites,
then affected hagency MCP bootstrap HTTP and task-client tests. Run warnings-denied
Clippy on all affected hagency execution and runtime targets and native plus Windows GNU checks; cross
compilation is not actual Windows qualification. Strict lifecycle uses --code .
and every changed path, including ./Cargo.lock if changed. Record pass fail skip
uncertain and platform refusal branches separately. Do not publish unimplemented
selectors or infer a positive network pass from an early durability refusal.

Original Linux CI85427cb failed the executable and uncertainty scenarios inside
an unlabelled Fake.next wait. The original branch and child status remain unknown;
a later local three-test pass does not explain those failures. The authorized
follow-up changes only the three executable fixture files, this decision/task and
append-only coordination docs. It preserves actual deadlines, results and cleanup
while capturing bounded fixed evidence from the same original child before panic
cleanup. The new refusal-observation test reuses the existing fifteen-second
fixture startup watchdog; no original deadline changes. No production path or
Matrix test helper is changed.

## Out of Scope

- receive_file cache paths image previews plaintext rooms arbitrary file destinations or a production file tool.
- Continuous scheduling live runtime or service deployment and full runtime sandbox qualification.
- New domain schema raw capability restoration pruning and automatic uncertain external retries.

2026-10-07: These Retired selectors depend on removed account/task CLI or the former direct-Palpo/Fleet production process. Library SDK coverage remains separate; current OwnerHost forbids these product commands. See [replacement coverage](../docs/design/2026-10-07-retired-product-test-coverage.zh-CN.md).
