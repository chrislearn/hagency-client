//! Signed Windows NSIS updates and desktop release notifications.
pub mod releases;
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    time::Duration,
};

pub const ENDPOINT: &str =
    "https://github.com/hagency-org/hagency-client/releases/latest/download/windows-update.json";
pub const MAX_MANIFEST: u64 = 64 * 1024;
pub const MAX_INSTALLER: u64 = 2 * 1024 * 1024 * 1024;
pub const PLATFORM: &str = "windows-x86_64";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: Version,
    pub notes: String,
    pub platforms: BTreeMap<String, Package>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub url: String,
    pub signature: String,
    pub size: u64,
    pub sha256: String,
    pub format: String,
}

fn decoded(value: &str) -> Result<String> {
    String::from_utf8(STANDARD.decode(value.trim())?).context("Invalid signing data")
}

pub fn public_key(encoded: &str) -> Result<PublicKey> {
    PublicKey::decode(&decoded(encoded)?).context("Invalid update public key")
}

fn signature(encoded: &str) -> Result<Signature> {
    Signature::decode(&decoded(encoded)?).context("Invalid update signature")
}

fn verify_reader(mut reader: impl Read, key: &PublicKey, signed: &str) -> Result<(u64, String)> {
    let sig = signature(signed)?;
    let mut verifier = key.verify_stream(&sig)?;
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        ensure!(total <= MAX_INSTALLER, "Update exceeds the download limit");
        verifier.update(&buffer[..count]);
        hash.update(&buffer[..count]);
    }
    verifier
        .finalize()
        .context("Update signature verification failed")?;
    Ok((total, format!("{:x}", hash.finalize())))
}

pub fn parse_manifest(
    bytes: &[u8],
    signed: &str,
    key: &PublicKey,
    current: &Version,
) -> Result<Option<Manifest>> {
    ensure!(
        bytes.len() as u64 <= MAX_MANIFEST,
        "Update manifest is too large"
    );
    key.verify(bytes, &signature(signed)?, false)
        .context("Manifest signature verification failed")?;
    let manifest: Manifest = serde_json::from_slice(bytes)?;
    ensure!(
        manifest.notes.len() <= 16 * 1024,
        "Release notes are too large"
    );
    // The stable channel never installs a prerelease or replays an older version.
    if !manifest.version.pre.is_empty() || manifest.version <= *current {
        return Ok(None);
    }
    let package = manifest.package()?;
    let expected = format!(
        "https://github.com/hagency-org/hagency-client/releases/download/v{}/hagency-windows-amd64.exe",
        manifest.version
    );
    ensure!(
        package.url == expected && package.format == "nsis",
        "Unsupported update package"
    );
    ensure!(
        package.size > 0 && package.size <= MAX_INSTALLER,
        "Invalid update size"
    );
    ensure!(
        package.sha256.len() == 64 && package.sha256.bytes().all(|c| c.is_ascii_hexdigit()),
        "Invalid update digest"
    );
    signature(&package.signature)?;
    Ok(Some(manifest))
}

impl Manifest {
    pub fn package(&self) -> Result<&Package> {
        self.platforms
            .get(PLATFORM)
            .context("No Windows x64 update is available")
    }
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::blocking::Client,
    pub key: PublicKey,
}

impl Client {
    pub fn new(encoded_key: &str) -> Result<Self> {
        Ok(Self {
            key: public_key(encoded_key)?,
            http: reqwest::blocking::Client::builder()
                .https_only(true)
                .connect_timeout(Duration::from_secs(15))
                .timeout(Duration::from_secs(30 * 60))
                .redirect(reqwest::redirect::Policy::limited(5))
                .user_agent(concat!("Hagency-Updater/", env!("CARGO_PKG_VERSION")))
                .build()?,
        })
    }
    fn small(&self, url: &str, limit: u64) -> Result<Vec<u8>> {
        let response = self
            .http
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()?
            .error_for_status()?;
        let mut bytes = Vec::new();
        response.take(limit + 1).read_to_end(&mut bytes)?;
        ensure!(bytes.len() as u64 <= limit, "Update response is too large");
        Ok(bytes)
    }
    pub fn check(&self, current: &Version) -> Result<Option<(Manifest, Vec<u8>, String)>> {
        let bytes = self.small(ENDPOINT, MAX_MANIFEST)?;
        let signed = String::from_utf8(self.small(&format!("{ENDPOINT}.sig"), 8192)?)?;
        Ok(parse_manifest(&bytes, &signed, &self.key, current)?
            .map(|manifest| (manifest, bytes, signed)))
    }
    pub fn download(
        &self,
        root: &Path,
        manifest: &Manifest,
        bytes: &[u8],
        signed: &str,
        progress: impl Fn(u64, u64),
    ) -> Result<Staged> {
        let package = manifest.package()?;
        std::fs::create_dir_all(root)?;
        let directory = root.join(manifest.version.to_string());
        std::fs::create_dir_all(&directory)?;
        let mut pending = tempfile::NamedTempFile::new_in(&directory)?;
        let response = self.http.get(&package.url).send()?.error_for_status()?;
        if let Some(length) = response.content_length() {
            ensure!(length == package.size, "Update size changed");
        }
        copy_download(response, pending.as_file_mut(), package.size, progress)?;
        pending.as_file_mut().sync_all()?;
        pending.as_file_mut().rewind()?;
        verify_package(pending.as_file_mut(), &self.key, package)?;
        let path = directory.join("installer.exe");
        pending.persist(&path).map_err(|e| e.error)?;
        atomic_write(&directory.join("manifest.json"), bytes)?;
        atomic_write(&directory.join("manifest.sig"), signed.as_bytes())?;
        // Publish the pointer last, after the signed package and metadata are durable.
        atomic_write(
            &root.join("staged.json"),
            &serde_json::to_vec(&manifest.version)?,
        )?;
        Ok(Staged {
            directory,
            manifest: manifest.clone(),
        })
    }
}

pub fn copy_download(
    mut source: impl Read,
    mut destination: impl Write,
    expected: u64,
    progress: impl Fn(u64, u64),
) -> Result<()> {
    ensure!(
        expected > 0 && expected <= MAX_INSTALLER,
        "Invalid update size"
    );
    let mut count = 0;
    let mut buffer = [0; 64 * 1024];
    loop {
        let length = source.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        count += length as u64;
        ensure!(count <= expected, "Update is larger than advertised");
        destination.write_all(&buffer[..length])?;
        progress(count, expected);
    }
    ensure!(count == expected, "Update download was interrupted");
    Ok(())
}

pub fn verify_package(reader: impl Read, key: &PublicKey, package: &Package) -> Result<()> {
    let (size, digest) = verify_reader(reader, key, &package.signature)?;
    ensure!(
        size == package.size && digest.eq_ignore_ascii_case(&package.sha256),
        "Update checksum does not match"
    );
    Ok(())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let directory = path.parent().context("Missing update directory")?;
    std::fs::create_dir_all(directory)?;
    let mut pending = tempfile::NamedTempFile::new_in(directory)?;
    pending.write_all(bytes)?;
    pending.as_file_mut().sync_all()?;
    pending.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= limit, "Update state is too large");
    Ok(bytes)
}

#[derive(Clone, Debug)]
pub struct Staged {
    pub directory: PathBuf,
    pub manifest: Manifest,
}

impl Staged {
    pub fn load(root: &Path, key: &PublicKey, current: &Version) -> Result<Option<Self>> {
        let pointer = root.join("staged.json");
        if !pointer.try_exists()? {
            return Ok(None);
        }
        let version: Version = serde_json::from_slice(&bounded_file(&pointer, 128)?)?;
        if version <= *current {
            return Ok(None);
        }
        let directory = root.join(version.to_string());
        let bytes = bounded_file(&directory.join("manifest.json"), MAX_MANIFEST)?;
        let signed = String::from_utf8(bounded_file(&directory.join("manifest.sig"), 8192)?)?;
        let Some(manifest) = parse_manifest(&bytes, &signed, key, current)? else {
            return Ok(None);
        };
        ensure!(version == manifest.version, "Staged update version changed");
        Ok(Some(Self {
            directory,
            manifest,
        }))
    }
    /// Keep this handle alive through installer launch on Windows. Deny writes
    /// and deletion so verification and execution refer to the same file.
    pub fn verified_file(&self, key: &PublicKey) -> Result<File> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(1); // FILE_SHARE_READ
        }
        let mut file = options.open(self.directory.join("installer.exe"))?;
        verify_package(&mut file, key, self.manifest.package()?)?;
        file.rewind()?;
        Ok(file)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub auto_check: bool,
    pub auto_download: bool,
    pub last_check: u64,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            auto_check: true,
            auto_download: true,
            last_check: 0,
        }
    }
}
impl Preferences {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("preferences.json");
        if !path.try_exists()? {
            return Ok(Self::default());
        }
        Ok(serde_json::from_slice(&bounded_file(&path, 4096)?)?)
    }
    pub fn save(&self, root: &Path) -> Result<()> {
        atomic_write(&root.join("preferences.json"), &serde_json::to_vec(self)?)
    }
    pub fn due(&self, now: u64) -> bool {
        self.auto_check
            && (now < self.last_check || now.saturating_sub(self.last_check) >= 24 * 60 * 60)
    }
}

pub fn stable_version(value: &str) -> Result<Version> {
    let version = Version::parse(value)?;
    if !version.pre.is_empty() {
        bail!("Automatic updates are unavailable in preview builds");
    }
    Ok(version)
}
