//! Scoped local storage with host authorization before reads and commits.
use std::{path::{Path, PathBuf}, io::{Read, Write}, collections::BTreeMap, sync::Mutex, marker::PhantomData};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::{document::*, assets::{Asset, prepare_image, MAX_FILE}, editing::EditorState, host::{ArticleHost, ConsentGrant, Capability}};
static STORE: Mutex<()> = Mutex::new(());
pub const APP_ID: &str = "org.octosense.article-editor";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Library<P = (), O = ()> {
    pub schema: u32,
    pub documents: Vec<Document>,
    pub assets: BTreeMap<String, Asset>,
    pub publications: Vec<P>,
    pub outbox: Vec<O>,
    #[serde(default)]
    pub legacy_source: Option<String>,
    /// Unapplied source belongs to a local draft, never to a published document.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub source_drafts: BTreeMap<String, String>,
    /// Host-local file origins for resolving explicit relative-link clicks.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub source_locations: BTreeMap<String, String>,
}
impl<P, O> Default for Library<P, O> {
    fn default() -> Self {
        Self {
            schema: 2,
            documents: Vec::new(),
            assets: BTreeMap::new(),
            publications: Vec::new(),
            outbox: Vec::new(),
            legacy_source: None,
            source_drafts: BTreeMap::new(),
            source_locations: BTreeMap::new(),
        }
    }
}
impl<P, O> Library<P, O> {
    pub fn source_for(&self, document_id: &str) -> Option<&str> {
        self.source_drafts.get(document_id).map(String::as_str).or_else(|| {
            // The old, single-draft format migrated into the first document.
            self.documents.first().filter(|d| d.id == document_id)
                .and(self.legacy_source.as_deref())
        })
    }

    pub fn clear_source(&mut self, document_id: &str) {
        self.source_drafts.remove(document_id);
        if self.documents.first().is_some_and(|d| d.id == document_id) {
            self.legacy_source = None;
        }
    }

    fn validate_sources(&self) -> Result<(), String> {
        if self.source_drafts.len() > 100 || self.source_drafts.iter().any(|(id, source)| {
            !self.documents.iter().any(|d| d.id == *id) || source.len() > MAX_BODY
        }) {
            return Err("Article source exceeds its storage limits.".into());
        }
        Ok(())
    }
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid storage path")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = path.with_extension(format!("{}.tmp", new_id()));
    let mut options = std::fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temporary).map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())
    })(); // Both draft and outbox use the same atomic replacement contract.
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}

/// Existing Robrix paths and schema are preserved, including opaque host metadata.
/// P/O belong to the host: the shared library never interprets Matrix room IDs.
pub struct LocalStore<'a, H: ArticleHost + ?Sized, P = (), O = ()> {
    host: &'a H,
    grant: &'a ConsentGrant,
    metadata: PhantomData<(P, O)>,
}
impl<'a, H: ArticleHost + ?Sized, P: Serialize + DeserializeOwned, O: Serialize + DeserializeOwned> LocalStore<'a, H, P, O> {
    pub fn new(host: &'a H, grant: &'a ConsentGrant) -> Self { Self { host, grant, metadata: PhantomData } }
    fn check(&self, capability: Capability) -> Result<(), String> { self.host.authorize(self.grant, capability) }
    fn directory(&self) -> PathBuf {
        self.host.data_root().join("mini-apps")
            .join(blake3::hash(self.grant.owner().as_bytes()).to_hex().as_str()).join(APP_ID)
    }
    fn load_inner(&self) -> Result<Library<P, O>, String> {
        match std::fs::File::open(self.directory().join("library-v2.json")) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(8_000_001).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
                if bytes.len() > 8_000_000 { return Err("Article library exceeds its storage limit.".into()); }
                let library: Library<P, O> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                if library.schema != 2 || library.documents.len() > 100 { return Err("Unsupported article library".into()); }
                for document in &library.documents { document.validate()?; }
                library.validate_sources()?;
                Ok(library)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut library = Library::default();
                let old = match std::fs::File::open(self.directory().join("draft.json")) {
                    Ok(file) => {
                        let mut bytes = Vec::new();
                        file.take(160_001).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
                        if bytes.len() > 160_000 { return Err("Draft exceeds storage limit".into()); }
                        let draft: EditorState = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                        draft.validate()?; draft
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => EditorState::default(),
                    Err(e) => return Err(e.to_string()),
                };
                if !old.title.is_empty() || !old.markdown.is_empty() {
                    match Document::from_markdown(&old.title, &old.markdown) {
                        Ok(document) => library.documents.push(document),
                        Err(_) => {
                            library.documents.push(Document { title: old.title, ..Document::default() });
                            library.legacy_source = Some(old.markdown);
                        }
                    }
                }
                Ok(library)
            }
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn load(&self) -> Result<Library<P, O>, String> {
        self.check(Capability::ReadDrafts)?;
        let _lock = STORE.lock().map_err(|_| "Article storage unavailable")?;
        let library = self.load_inner()?;
        self.check(Capability::ReadDrafts)?;
        Ok(library)
    }
    pub fn update<T>(&self, edit: impl FnOnce(&mut Library<P, O>) -> Result<T, String>) -> Result<T, String> {
        self.check(Capability::WriteDrafts)?;
        self.check(Capability::ReadDrafts)?;
        let _lock = STORE.lock().map_err(|_| "Article storage unavailable")?;
        let mut library = self.load_inner()?;
        let value = edit(&mut library)?;
        if library.documents.len() > 100 { return Err("Keep at most 100 article drafts on this device.".into()); }
        for document in &library.documents { document.validate()?; }
        library.validate_sources()?;
        let bytes = serde_json::to_vec(&library).map_err(|e| e.to_string())?;
        if bytes.len() > 8_000_000 { return Err("Article library exceeds its storage limit.".into()); }
        self.check(Capability::WriteDrafts)?;
        atomic_write(&self.directory().join("library-v2.json"), &bytes)?;
        Ok(value)
    }
    pub fn save_document(&self, document: &Document) -> Result<(), String> {
        document.validate()?;
        self.update(|library| {
            if let Some(old) = library.documents.iter_mut().find(|d| d.id == document.id) { *old = document.clone(); }
            else { library.documents.push(document.clone()); }
            Ok(())
        })
    }
    /// Commit the visual draft and its unapplied source together. A successful
    /// import passes None to clear only this document's saved source.
    pub fn save_document_with_source(&self, document: &Document, source: Option<&str>) -> Result<(), String> {
        document.validate()?;
        self.update(|library| {
            if let Some(old) = library.documents.iter_mut().find(|d| d.id == document.id) { *old = document.clone(); }
            else { library.documents.push(document.clone()); }
            library.clear_source(&document.id);
            if let Some(source) = source {
                library.source_drafts.insert(document.id.clone(), source.to_owned());
            }
            Ok(())
        })
    }
    fn asset_path(&self, id: &str) -> Result<PathBuf, String> {
        if !valid_id(id) { return Err("Invalid image reference".into()); }
        Ok(self.directory().join("assets").join(id))
    }
    pub fn asset_bytes(&self, id: &str) -> Result<Vec<u8>, String> {
        self.check(Capability::ReadAssets)?;
        let file = std::fs::File::open(self.asset_path(id)?)
            .map_err(|_| "An article image is missing. Replace it before publishing.")?;
        let mut bytes = Vec::new();
        file.take(MAX_FILE + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE || blake3::hash(&bytes).to_hex().as_str() != id {
            return Err("Article image failed its integrity check.".into());
        }
        self.check(Capability::ReadAssets)?;
        Ok(bytes)
    }
    /// Bytes must be supplied by the trusted host's image picker/asset service.
    /// Documents and scripts cannot choose filesystem paths through this API.
    pub fn import_image(&self, bytes: &[u8], name: &str) -> Result<Asset, String> {
        self.check(Capability::ImportAssets)?;
        self.check(Capability::WriteDrafts)?;
        let (asset, output) = prepare_image(bytes, name)?;
        self.check(Capability::ImportAssets)?;
        // Authorization is checked again inside update before metadata commits.
        self.update(|library| {
            if library.assets.len() >= 300 && !library.assets.contains_key(&asset.id) { return Err("The image library is full.".into()); }
            self.check(Capability::ImportAssets)?;
            atomic_write(&self.asset_path(&asset.id)?, &output)?;
            library.assets.insert(asset.id.clone(), asset.clone());
            Ok(())
        })?;
        Ok(asset)
    }
}
