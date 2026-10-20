use base64::{Engine, engine::general_purpose::STANDARD};
use hagency_updater::*;
use minisign::{KeyPair, sign};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Cursor, Write},
};

struct Release {
    keys: KeyPair,
    payload: Vec<u8>,
    manifest: Manifest,
}
impl Release {
    fn new() -> Self {
        let keys = KeyPair::generate_unencrypted_keypair().unwrap();
        let payload = b"MZ verified installer test fixture".to_vec();
        Self { manifest: Manifest {
            version: "0.7.0".parse().unwrap(), notes: "Release notes".into(),
            platforms: BTreeMap::from([(PLATFORM.into(), Package {
                url: "https://github.com/hagency-org/hagency-client/releases/download/v0.7.0/hagency-windows-amd64.exe".into(),
                signature: Self::sign(&keys, &payload), size: payload.len() as u64,
                sha256: format!("{:x}", Sha256::digest(&payload)), format: "nsis".into(),
            })]),
        }, keys, payload }
    }
    fn sign(keys: &KeyPair, bytes: &[u8]) -> String {
        STANDARD.encode(
            sign(
                Some(&keys.pk),
                &keys.sk,
                Cursor::new(bytes),
                Some("test release"),
                None,
            )
            .unwrap()
            .to_string(),
        )
    }
    fn key(&self) -> minisign_verify::PublicKey {
        public_key(&STANDARD.encode(self.keys.pk.to_box().unwrap().to_string())).unwrap()
    }
    fn signed(&self) -> (Vec<u8>, String) {
        let bytes = serde_json::to_vec(&self.manifest).unwrap();
        let signed = Self::sign(&self.keys, &bytes);
        (bytes, signed)
    }
    fn stage(&self, root: &std::path::Path) -> Staged {
        let directory = root.join(self.manifest.version.to_string());
        std::fs::create_dir_all(&directory).unwrap();
        let (bytes, signed) = self.signed();
        for (name, data) in [
            ("manifest.json", bytes.as_slice()),
            ("manifest.sig", signed.as_bytes()),
            ("installer.exe", self.payload.as_slice()),
        ] {
            atomic_write(&directory.join(name), data).unwrap();
        }
        atomic_write(&root.join("staged.json"), br#""0.7.0""#).unwrap();
        Staged::load(root, &self.key(), &"0.6.0".parse().unwrap())
            .unwrap()
            .unwrap()
    }
}

#[test]
fn official_packager_signature_format_verifies_manifest_and_streamed_installer() {
    let release = Release::new();
    let (bytes, signed) = release.signed();
    let manifest = parse_manifest(&bytes, &signed, &release.key(), &"0.6.0".parse().unwrap())
        .unwrap()
        .unwrap();
    verify_package(
        Cursor::new(&release.payload),
        &release.key(),
        manifest.package().unwrap(),
    )
    .unwrap();
}
#[test]
fn rejects_tampered_manifest_key_and_installer() {
    let release = Release::new();
    let (mut bytes, signed) = release.signed();
    bytes[1] ^= 1;
    assert!(parse_manifest(&bytes, &signed, &release.key(), &"0.6.0".parse().unwrap()).is_err());
    let (bytes, signed) = release.signed();
    assert!(
        parse_manifest(
            &bytes,
            &signed,
            &Release::new().key(),
            &"0.6.0".parse().unwrap()
        )
        .is_err()
    );
    assert!(
        verify_package(
            Cursor::new(b"malicious executable"),
            &release.key(),
            release.manifest.package().unwrap()
        )
        .is_err()
    );
}
#[test]
fn refuses_downgrade_same_version_preview_wrong_platform_and_url() {
    let mut release = Release::new();
    for current in ["0.7.0", "0.8.0"] {
        let (bytes, signed) = release.signed();
        assert!(
            parse_manifest(&bytes, &signed, &release.key(), &current.parse().unwrap())
                .unwrap()
                .is_none()
        );
    }
    release.manifest.version = "0.8.0-beta.1".parse().unwrap();
    let (bytes, signed) = release.signed();
    assert!(
        parse_manifest(&bytes, &signed, &release.key(), &"0.6.0".parse().unwrap())
            .unwrap()
            .is_none()
    );
    release.manifest.version = "0.7.0".parse().unwrap();
    release.manifest.platforms.get_mut(PLATFORM).unwrap().url =
        "http://example.com/installer.exe".into();
    let (bytes, signed) = release.signed();
    assert!(parse_manifest(&bytes, &signed, &release.key(), &"0.6.0".parse().unwrap()).is_err());
    release.manifest.platforms.clear();
    let (bytes, signed) = release.signed();
    assert!(parse_manifest(&bytes, &signed, &release.key(), &"0.6.0".parse().unwrap()).is_err());
}
#[test]
fn interrupted_oversized_and_failed_disk_writes_never_complete() {
    assert!(copy_download(Cursor::new(b"partial"), Vec::new(), 100, |_, _| {}).is_err());
    assert!(copy_download(Cursor::new(b"oversized"), Vec::new(), 2, |_, _| {}).is_err());
    struct FullDisk;
    impl Write for FullDisk {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("disk full"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert!(copy_download(Cursor::new(b"data"), FullDisk, 4, |_, _| {}).is_err());
    let mut saved = Vec::new();
    copy_download(Cursor::new(b"data"), &mut saved, 4, |a, b| assert!(a <= b)).unwrap();
    assert_eq!(saved, b"data");
}
#[test]
fn staged_update_survives_restart_and_is_reverified_before_install() {
    let release = Release::new();
    let root = tempfile::tempdir().unwrap();
    let staged = release.stage(root.path());
    assert!(staged.verified_file(&release.key()).is_ok());
    std::fs::write(staged.directory.join("installer.exe"), b"corrupt").unwrap();
    let restored = Staged::load(root.path(), &release.key(), &"0.6.0".parse().unwrap())
        .unwrap()
        .unwrap();
    assert!(restored.verified_file(&release.key()).is_err());
    assert!(
        Staged::load(root.path(), &release.key(), &"0.7.0".parse().unwrap())
            .unwrap()
            .is_none()
    );
}
#[test]
fn staged_manifest_cannot_escape_cache_or_change_version() {
    let release = Release::new();
    let root = tempfile::tempdir().unwrap();
    let staged = release.stage(root.path());
    atomic_write(&root.path().join("staged.json"), br#""../../elsewhere""#).unwrap();
    assert!(Staged::load(root.path(), &release.key(), &"0.6.0".parse().unwrap()).is_err());
    atomic_write(&root.path().join("staged.json"), br#""0.7.0""#).unwrap();
    std::fs::write(staged.directory.join("manifest.json"), b"{}").unwrap();
    assert!(Staged::load(root.path(), &release.key(), &"0.6.0".parse().unwrap()).is_err());
}
#[test]
fn preferences_are_machine_local_durable_and_rate_limit_checks() {
    let root = tempfile::tempdir().unwrap();
    let mut prefs = Preferences::load(root.path()).unwrap();
    assert!(prefs.auto_check && prefs.auto_download);
    prefs.last_check = 100_000;
    assert!(!prefs.due(100_030));
    assert!(prefs.due(200_000));
    prefs.auto_check = false;
    assert!(!prefs.due(200_000));
    prefs.save(root.path()).unwrap();
    assert_eq!(Preferences::load(root.path()).unwrap(), prefs);
    prefs.auto_download = false;
    prefs.save(root.path()).unwrap();
    assert!(!Preferences::load(root.path()).unwrap().auto_download);
}
#[cfg(windows)]
#[test]
fn verified_installer_cannot_be_replaced_while_held_for_execution() {
    let release = Release::new();
    let root = tempfile::tempdir().unwrap();
    let staged = release.stage(root.path());
    let held = staged.verified_file(&release.key()).unwrap();
    assert!(std::fs::write(staged.directory.join("installer.exe"), b"changed").is_err());
    assert!(std::fs::remove_file(staged.directory.join("installer.exe")).is_err());
    drop(held);
    assert!(std::fs::write(staged.directory.join("installer.exe"), b"changed").is_ok());
}
