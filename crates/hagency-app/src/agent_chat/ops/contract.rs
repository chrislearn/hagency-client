//! Pinned upstream fixtures; verified before any development-mode handshake.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
pub fn verify() -> Result<()> {
    let manifest: serde_json::Value = serde_json::from_str(include_str!("contract/manifest.json"))?;
    ensure!(
        manifest["contract"] == super::SCHEMA,
        "Agent contract mismatch"
    );
    let artifacts = [
        (
            "cancel-dispatch-response.json",
            include_bytes!("contract/cancel-dispatch-response.json").as_slice(),
        ),
        (
            "client-session-grant.json",
            include_bytes!("contract/client-session-grant.json").as_slice(),
        ),
        (
            "client-session-request.json",
            include_bytes!("contract/client-session-request.json").as_slice(),
        ),
        (
            "client-session.json",
            include_bytes!("contract/client-session.json").as_slice(),
        ),
        (
            "contract.schema.json",
            include_bytes!("contract/contract.schema.json").as_slice(),
        ),
        (
            "error-precondition-failed.json",
            include_bytes!("contract/error-precondition-failed.json").as_slice(),
        ),
        (
            "grant-exchange-request.json",
            include_bytes!("contract/grant-exchange-request.json").as_slice(),
        ),
        (
            "grant-proof-material.json",
            include_bytes!("contract/grant-proof-material.json").as_slice(),
        ),
        (
            "invalid-empty-capability.json",
            include_bytes!("contract/invalid-empty-capability.json").as_slice(),
        ),
        (
            "invalid-terminal-resolution-with-recovery.json",
            include_bytes!("contract/invalid-terminal-resolution-with-recovery.json").as_slice(),
        ),
        (
            "invalidation.json",
            include_bytes!("contract/invalidation.json").as_slice(),
        ),
        (
            "outcome-inspection.json",
            include_bytes!("contract/outcome-inspection.json").as_slice(),
        ),
        (
            "protocol-limits.json",
            include_bytes!("contract/protocol-limits.json").as_slice(),
        ),
        (
            "resolve-outcome-request-accept-completed.json",
            include_bytes!("contract/resolve-outcome-request-accept-completed.json").as_slice(),
        ),
        (
            "resolve-outcome-request-continue.json",
            include_bytes!("contract/resolve-outcome-request-continue.json").as_slice(),
        ),
        (
            "resolve-outcome-request-keep-blocked.json",
            include_bytes!("contract/resolve-outcome-request-keep-blocked.json").as_slice(),
        ),
        (
            "resolve-outcome-response.json",
            include_bytes!("contract/resolve-outcome-response.json").as_slice(),
        ),
        (
            "session-proof-material.json",
            include_bytes!("contract/session-proof-material.json").as_slice(),
        ),
        (
            "snapshot.json",
            include_bytes!("contract/snapshot.json").as_slice(),
        ),
        (
            "stable-errors.json",
            include_bytes!("contract/stable-errors.json").as_slice(),
        ),
    ];
    let entries = manifest["artifacts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid contract manifest"))?;
    ensure!(entries.len() == artifacts.len(), "Incomplete contract");
    for (path, bytes) in artifacts {
        let entry = entries
            .iter()
            .find(|entry| entry["path"] == path)
            .ok_or_else(|| anyhow::anyhow!("Missing contract artifact"))?;
        ensure!(
            entry["sha256"] == format!("{:x}", Sha256::digest(bytes)),
            "Contract artifact changed"
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn pinned_artifacts_match_producer_digests() {
        super::verify().unwrap();
    }
    #[test]
    fn production_gate_remains_closed() {
        assert_eq!(super::super::available(), cfg!(feature = "agent_ops_dev"));
    }
}
