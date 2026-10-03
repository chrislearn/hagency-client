//! Startup captures actual original files; HTTP never resolves a filesystem path.
use super::Error;
use cap_fs_ext::DirExt;
use cap_std::{ambient_authority, fs::Dir};
use hagency_files::{Limits, RelativeFile, Snapshot, Workspace};
use hyper::body::Bytes;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    assets: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    size: usize,
    sha256: String,
    mime: String,
}
pub(super) struct Asset {
    pub(super) bytes: Bytes,
    pub(super) mime: String,
    /// The validated file handle for a console folder; none for the
    /// embedded console, whose bytes are part of the binary.
    _proof: Option<Snapshot>,
}
pub(super) struct Assets {
    values: BTreeMap<String, Asset>,
    _manifest: Option<Snapshot>,
}

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_console.rs"));
}
/// ADR-189: whether this binary carries the console build.
pub(super) fn embedded_available() -> bool {
    !embedded::FILES.is_empty()
}

fn root(path: &Path) -> Result<Dir, Error> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| Error::Assets)?
            .join(path)
    };
    let mut anchor = PathBuf::new();
    let mut parts = Vec::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir if parts.is_empty() => {
                anchor.push(component.as_os_str())
            }
            Component::Normal(name) => parts.push(name),
            Component::CurDir => {}
            _ => return Err(Error::Assets),
        }
    }
    if !anchor.is_absolute() || parts.is_empty() {
        return Err(Error::Assets);
    }
    let mut dir = Dir::open_ambient_dir(anchor, ambient_authority()).map_err(|_| Error::Assets)?;
    // Ancestors may be spelled through a symlink: the host's own worktree
    // alias (`/Users/x/home/hl -> hl.noindex`, macOS `/tmp -> /private/tmp`)
    // is a legitimate way to name the bundle, and following it cannot change
    // which bytes the final handle proves. The asset directory ITSELF must be
    // a real directory, never a link: a link can be repointed at any moment,
    // so the proof handle would stop naming the bytes we validated.
    let (last, ancestors) = parts.split_last().ok_or(Error::Assets)?;
    for name in ancestors {
        dir = dir.open_dir(name).map_err(|_| Error::Assets)?;
    }
    dir = dir.open_dir_nofollow(last).map_err(|_| Error::Assets)?;
    // Existing helper validates owner and private permissions from the actual handle.
    hagency_store::private::check_handle(
        &dir.try_clone().map_err(|_| Error::Assets)?.into_std_file(),
    )
    .map_err(|_| Error::Assets)?;
    Ok(dir)
}
fn snapshot(dir: &Dir, path: &str, limit: usize) -> Result<Snapshot, Error> {
    Workspace::from_directory(
        dir.try_clone().map_err(|_| Error::Assets)?,
        Limits::new(limit.max(1), 1).map_err(|_| Error::Assets)?,
    )
    .map_err(|_| Error::Assets)?
    .snapshot(&RelativeFile::new(path).map_err(|_| Error::Assets)?)
    .map_err(|_| Error::Assets)
}
/// A document is the front door (`index.html`) or a rail page
/// (`<route>/index.html`). Derived from the path SHAPE, never a hand list:
/// the hand list silently refused a page the build shipped, and a refused page
/// fails `load` outright — the service will not start on that bundle at all
/// (board #88). The length, character and segment rules are the same ones the
/// static branch below enforces, so a traversal or a bogus byte can never name
/// a document.
fn document(path: &str) -> bool {
    let Some(route) = path.strip_suffix("index.html") else {
        return false;
    };
    // The front door is exactly `index.html`; a page is `<route>/index.html`
    // with the route one or more plain, non-empty segments.
    if !route.is_empty() && !route.ends_with('/') {
        return false;
    }
    path.len() <= 512
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
}
fn mime(path: &str) -> Option<&'static str> {
    if document(path) {
        return Some("text/html; charset=utf-8");
    }
    if !path.starts_with("_next/static/")
        || path.len() > 512
        || path
            .split('/')
            .any(|v| v.is_empty() || v == "." || v == "..")
        || !path
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"/_-.".contains(&v))
    {
        return None;
    }
    match path.rsplit('.').next()? {
        "js" => Some("text/javascript; charset=utf-8"),
        "css" => Some("text/css; charset=utf-8"),
        "woff2" => Some("font/woff2"),
        "woff" => Some("font/woff"),
        _ => None,
    }
}
fn key(path: &str) -> String {
    // Derived, not hand-mapped: the front door is `/console/`, and a page
    // `<route>/index.html` serves at `/console/<route>/` (board #88).
    if path == "index.html" {
        "/console/".into()
    } else if document(path) {
        format!("/console/{}", &path[..path.len() - "index.html".len()])
    } else {
        format!("/console/{path}")
    }
}
fn manifest(bytes: &[u8]) -> Result<Manifest, Error> {
    let input: Manifest = serde_json::from_slice(bytes).map_err(|_| Error::Assets)?;
    if input.version != 1 || input.assets.is_empty() || input.assets.len() > 512 {
        return Err(Error::Assets);
    }
    Ok(input)
}
impl Assets {
    /// ADR-189: the console compiled into the binary, checked against its own
    /// manifest with the same rules as a console folder.
    pub(super) fn embedded() -> Result<Self, Error> {
        use sha2::{Digest, Sha256};
        let files: BTreeMap<&str, &[u8]> = embedded::FILES.iter().copied().collect();
        let input = manifest(files.get("manifest.json").ok_or(Error::Assets)?)?;
        let mut total = 0usize;
        let mut values = BTreeMap::new();
        for entry in input.assets {
            let expected = mime(&entry.path).ok_or(Error::Assets)?;
            if entry.mime != expected || entry.size > 4 * 1024 * 1024 || entry.sha256.len() != 64 {
                return Err(Error::Assets);
            }
            total = total
                .checked_add(entry.size)
                .filter(|n| *n <= 32 * 1024 * 1024)
                .ok_or(Error::Assets)?;
            let bytes = *files.get(entry.path.as_str()).ok_or(Error::Assets)?;
            let digest: String = Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            if bytes.len() != entry.size || digest != entry.sha256 {
                return Err(Error::Assets);
            }
            let asset = Asset {
                bytes: Bytes::from_static(bytes),
                mime: entry.mime,
                _proof: None,
            };
            if values.insert(key(&entry.path), asset).is_some() {
                return Err(Error::Assets);
            }
        }
        if !values.contains_key("/console/usage/") {
            return Err(Error::Assets);
        }
        Ok(Self {
            values,
            _manifest: None,
        })
    }
    pub(super) fn load(path: &Path) -> Result<Self, Error> {
        let dir = root(path)?;
        let manifest_file = snapshot(&dir, "manifest.json", 128 * 1024)?;
        let input = manifest(manifest_file.bytes())?;
        let mut total = 0usize;
        let mut values = BTreeMap::new();
        for entry in input.assets {
            let expected = mime(&entry.path).ok_or(Error::Assets)?;
            if entry.mime != expected || entry.size > 4 * 1024 * 1024 || entry.sha256.len() != 64 {
                return Err(Error::Assets);
            }
            total = total
                .checked_add(entry.size)
                .filter(|n| *n <= 32 * 1024 * 1024)
                .ok_or(Error::Assets)?;
            let proof = snapshot(&dir, &entry.path, entry.size)?;
            let digest: String = proof.digest().iter().map(|b| format!("{b:02x}")).collect();
            if proof.len() != entry.size || digest != entry.sha256 {
                return Err(Error::Assets);
            }
            let key = key(&entry.path);
            let asset = Asset {
                bytes: Bytes::copy_from_slice(proof.bytes()),
                mime: entry.mime,
                _proof: Some(proof),
            };
            if values.insert(key, asset).is_some() {
                return Err(Error::Assets);
            }
        }
        if !values.contains_key("/console/usage/") {
            return Err(Error::Assets);
        }
        Ok(Self {
            values,
            _manifest: Some(manifest_file),
        })
    }
    pub(super) fn get(&self, path: &str) -> Option<&Asset> {
        // The exact key wins: a static chunk is stored as the URL it is
        // requested at, and must never be rewritten.
        if let Some(asset) = self.values.get(path) {
            return Some(asset);
        }
        // `/console` and `/console/<route>` are the no-trailing-slash spellings
        // of a document key (`/console/`, `/console/<route>/`). Only a
        // document is stored with a trailing slash, so the retry cannot shadow
        // a static asset. Derived from the path shape, so a newly shipped page
        // needs no edit here (board #88).
        let with_slash = if path == "/console" {
            "/console/".to_owned()
        } else if path.starts_with("/console/") {
            format!("{path}/")
        } else {
            return None;
        };
        self.values.get(&with_slash)
    }
}
