//! Release gate: verify the manifest and installer with the embedded public key.
use anyhow::{Context, Result};
use hagency_updater::{parse_manifest, public_key, verify_package};
use std::{fs, path::PathBuf};
fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("Expected asset directory")?,
    );
    let key = public_key(&std::env::var("HAGENCY_UPDATER_PUBLIC_KEY")?)?;
    let bytes = fs::read(root.join("windows-update.json"))?;
    let signed = fs::read_to_string(root.join("windows-update.json.sig"))?;
    let manifest = parse_manifest(&bytes, &signed, &key, &"0.0.0".parse()?)?
        .context("Expected stable release")?;
    verify_package(
        fs::File::open(root.join("hagency-windows-amd64.exe"))?,
        &key,
        manifest.package()?,
    )?;
    println!("Verified Windows update {}", manifest.version);
    Ok(())
}
