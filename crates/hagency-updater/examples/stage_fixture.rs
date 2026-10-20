//! Offline installer-handoff fixture. Generates an ephemeral test key in memory.
use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use hagency_updater::{Manifest, PLATFORM, Package, atomic_write};
use minisign::{KeyPair, sign};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Cursor, path::PathBuf};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(args.len() == 2, "Expected fixture root and NSIS installer");
    let root = PathBuf::from(&args[0]);
    let installer = PathBuf::from(&args[1]);
    let key = KeyPair::generate_unencrypted_keypair()?;
    let sign_bytes = |bytes: &[u8]| -> Result<String> {
        Ok(STANDARD.encode(
            sign(
                Some(&key.pk),
                &key.sk,
                Cursor::new(bytes),
                Some("offline fixture only"),
                None,
            )?
            .to_string(),
        ))
    };
    let payload = fs::read(installer)?;
    let version = "0.99.0".parse()?;
    let manifest = Manifest { version, notes: "Offline NSIS integration fixture".into(), platforms: BTreeMap::from([(PLATFORM.into(), Package {
        url: "https://github.com/hagency-org/hagency-client/releases/download/v0.99.0/hagency-windows-amd64.exe".into(),
        signature: sign_bytes(&payload)?, size: payload.len() as u64, sha256: format!("{:x}", Sha256::digest(&payload)), format: "nsis".into(),
    })]) };
    let bytes = serde_json::to_vec(&manifest)?;
    let directory = root.join("updates/0.99.0");
    atomic_write(&directory.join("installer.exe"), &payload)?;
    atomic_write(&directory.join("manifest.json"), &bytes)?;
    atomic_write(
        &directory.join("manifest.sig"),
        sign_bytes(&bytes)?.as_bytes(),
    )?;
    atomic_write(
        &root.join("updates/staged.json"),
        &serde_json::to_vec(&manifest.version)?,
    )?;
    atomic_write(
        &root
            .parent()
            .context("Missing fixture parent")?
            .join("fixture-public-key.txt"),
        STANDARD.encode(key.pk.to_box()?.to_string()).as_bytes(),
    )?;
    println!("Created offline update fixture");
    Ok(())
}
