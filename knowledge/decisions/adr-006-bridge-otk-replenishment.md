---
kind: decision
id: ADR-006
title: "Reconcile missing bridge OTK counts at a bounded cadence"
status: Accepted
liveness: auto
tags: [matrix, e2ee, approval, olm]
---

## Context

An encrypted approval verdict uses Megolm for the room event and Olm to deliver
the Megolm session key to the bridge device. Palpo can omit a usable
`signed_curve25519` count even when the bridge needs to reconcile its one-time
key pool. The Matrix crypto state machine treats an absent count as unknown, so
it may not replenish an exhausted pool. Conversely, rewriting every absent
count as zero causes 50 new keys to be uploaded on every sync and can leave
undecryptable orphaned public keys on the server.

## Decision

The bridge does not alter Matrix `/sync` count semantics. A missing
`device_one_time_keys_count` field remains missing, and numeric counts pass
through unchanged.

When a sync does not contain a usable signed Curve25519 count, the bridge
periodically sends an empty `/keys/upload` request, whose response contains the
authoritative current count. It feeds that count into matrix-sdk-crypto using
the SDK's supported sync-update API. Reconciliation is limited to once every
five minutes; key generation, signing, storage, and any required upload remain
inside matrix-sdk-crypto. Logs may include counts but never key material.

## Consequences

Good, because fresh Robrix2 devices can establish Olm sessions with the bridge
and approval verdicts no longer depend on an accidentally pre-existing session.

Good, because key generation, signing, storage, and upload remain owned by
matrix-sdk-crypto rather than custom cryptographic code.

Good, because a homeserver that omits counts cannot trigger an upload on every
sync round.

Bad, because affected homeservers require one additional bounded count probe.

## Alternatives Considered

- Disable encryption for approval rooms: rejected because request details and
  verdicts are private owner communications. A temporary plaintext diagnostic
  room may be selected only through two explicit non-production settings to
  isolate protocol behavior from E2EE behavior; it is not a deployment mode or
  an automatic fallback.
- Implement custom Olm key generation: rejected because cryptographic lifecycle
  must remain inside the audited SDK.
- Treat every absent count as zero: rejected after live testing produced 17
  consecutive uploads and grew the server pool from 150 to 950 keys.
- Retain undecryptable events only: insufficient, because retention cannot
  recover a key that was never shareable.

## Amendment (2026-10-02): the native service does not implement this yet

This decision was implemented only in the JavaScript Matrix bridge
(`bridge-matrix.js`: `MATRIX_OTK_COUNT_RECONCILE_MS`, an empty `/keys/upload`
probe, then `updateSyncData`), which was removed when the repository became the
native Rust service. The decision itself stands; the native implementation is
owed.

What the native service does today:

- It passes `/sync` to matrix-sdk-crypto unchanged (`receive_sync_response` in
  `native/hagency-matrix/src/sdk.rs`), so absent counts stay absent, as decided
  above.
- It uploads device keys, one-time keys and a fallback key once, during
  enrollment (`native/hagency-matrix/src/sdk/enrollment.rs`). After that it
  never sends the SDK's later `KeysUpload` requests, so one-time keys are not
  replenished even when the homeserver does report a count. ADR-102 ("no ...
  OTK replenishment") and ADR-043 ("one-time-key/device maintenance" as a
  remaining gate) record the same gap.

Consequence: each new owner device or session claims one of the original
one-time keys. Once they are gone, new Olm sessions use the fallback key, which
is never rotated. Sessions still establish, but with a reused key and weaker
forward secrecy, for every long-lived agent and the approval bot.

To close the gap, the native Matrix owner must send matrix-sdk-crypto's
post-enrollment key uploads (including fallback-key rotation) through its
existing custody, and add this ADR's bounded count probe for homeservers that
omit `device_one_time_keys_count`. Tracked in hagency-org/hagency-rs#19.
