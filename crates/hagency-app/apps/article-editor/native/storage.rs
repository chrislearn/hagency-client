//! Rinx's Matrix publication metadata and shared article storage adapter.
use std::{path::Path, collections::BTreeMap, io::Read};
use serde::{Serialize, Deserialize};
use ruma::{OwnedRoomId, OwnedEventId};
use super::{document::*, model::Grant, host::RobrixArticleHost};
pub use article_core::assets::{Asset, MAX_FILE};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteAsset {
    pub asset: Asset,
    pub source: ruma::events::room::MediaSource,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    pub id: String,
    pub document_id: String,
    pub room: OwnedRoomId,
    pub room_name: String,
    pub root: OwnedEventId,
    pub events: Vec<OwnedEventId>,
    pub version: u64,
    pub withdrawn: bool,
    pub modified: u64,
    pub document: Document,
    pub assets: BTreeMap<String, RemoteAsset>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationKind {
    Publish,
    Update,
    Withdraw,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub id: String,
    pub kind: OperationKind,
    pub document: Document,
    pub room: OwnedRoomId,
    pub room_name: String,
    pub publication: Option<String>,
    pub root: Option<OwnedEventId>,
    pub version: u64,
    pub uploaded: BTreeMap<String, RemoteAsset>,
    pub redacted: Vec<OwnedEventId>,
    pub confirmed: Option<OwnedEventId>,
    pub finished: bool,
}

pub type Library = article_core::storage::Library<Publication, Operation>;
type Store<'a> = article_core::storage::LocalStore<'a, RobrixArticleHost<'a>, Publication, Operation>;
pub fn load(root: &Path, grant: &Grant) -> Result<Library, String> {
    Store::new(&RobrixArticleHost::new(root), &grant.lease).load()
}
pub fn update<T>(root: &Path, grant: &Grant, edit: impl FnOnce(&mut Library) -> Result<T, String>) -> Result<T, String> {
    Store::new(&RobrixArticleHost::new(root), &grant.lease).update(edit)
}
pub fn save_document_with_source(root: &Path, grant: &Grant, document: &Document, source: Option<&str>) -> Result<(), String> {
    Store::new(&RobrixArticleHost::new(root), &grant.lease).save_document_with_source(document, source)
}
pub fn read_import(grant: &Grant, path: &Path, name: &str) -> Result<Document, String> {
    use article_core::host::Capability;
    grant.authorize(Capability::WriteDrafts)?;
    let mut bytes = Vec::new();
    std::fs::File::open(path).map_err(|e| e.to_string())?
        .take((MAX_BODY + 1) as u64).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    let mut document = article_core::markup::import_file(name, &bytes)?;
    let origin=path.canonicalize().map_err(|e|e.to_string())?;
    for reference in article_core::render::image_references(&document) {
        let Ok((image,name))=read_relative_image(&origin,&reference) else {continue;};
        let host=RobrixArticleHost::new(crate::app_data_dir());
        let asset=Store::new(&host,&grant.lease).import_image(&image.png,&name)?;
        document.resource_bindings.insert(reference,asset.id);
    }
    grant.authorize(Capability::WriteDrafts)?;
    update(crate::app_data_dir(),grant,|library| {
        library.documents.push(document.clone());
        library.source_locations.insert(document.id.clone(),origin.to_string_lossy().into_owned());
        Ok(())
    })?;
    Ok(document)
}

fn read_relative_image(origin:&Path,reference:&str)->Result<(article_makepad::content::PreviewImage,String),String> {
    let path=resolve_relative(origin,reference)?;
    let mut bytes=Vec::new();
    std::fs::File::open(&path).and_then(|f|f.take(MAX_FILE+1).read_to_end(&mut bytes)).map_err(|e|e.to_string())?;
    let image=article_makepad::content::prepare_image(&bytes)?;
    Ok((image,path.file_name().and_then(|s|s.to_str()).unwrap_or("image").into()))
}

fn resolve_relative(origin: &Path, reference: &str) -> Result<std::path::PathBuf,String> {
    if url::Url::parse(reference).is_ok() || reference.starts_with('#') || reference.starts_with("//") {
        return Err("Expected a relative file reference.".into());
    }
    let base=url::Url::from_file_path(origin).map_err(|_|"Invalid import origin")?;
    let target=base.join(reference).map_err(|e|e.to_string())?.to_file_path().map_err(|_|"Invalid file reference")?;
    let target=target.canonicalize().map_err(|e|e.to_string())?;
    let folder=origin.parent().ok_or("Missing article folder")?;
    if !target.starts_with(folder) { return Err("Relative file is outside the imported article folder.".into()); }
    Ok(target)
}

pub fn read_relative(grant:&Grant,document_id:&str,reference:&str)->Result<Document,String> {
    let library=load(crate::app_data_dir(),grant)?;
    let origin=library.source_locations.get(document_id).ok_or("Relative link has no imported destination.")?;
    let target=resolve_relative(Path::new(origin),reference)?;
    if let Some((id,_))=library.source_locations.iter().find(|(_,p)|Path::new(p)==target) {
        if let Some(doc)=library.documents.iter().find(|d|&d.id==id) {return Ok(doc.clone());}
    }
    let name=target.file_name().and_then(|s|s.to_str()).ok_or("Invalid file name")?;
    read_import(grant,&target,name)
}
pub fn asset_bytes(root: &Path, grant: &Grant, id: &str) -> Result<Vec<u8>, String> {
    Store::new(&RobrixArticleHost::new(root), &grant.lease).asset_bytes(id)
}
pub fn import_image(root: &Path, grant: &Grant, path: &Path, name: &str) -> Result<Asset, String> {
    use article_core::host::{ArticleHost, Capability};
    let host = RobrixArticleHost::new(root);
    host.authorize(&grant.lease, Capability::ImportAssets)?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Store::new(&host, &grant.lease).import_image(&bytes, name)
}

/// Imports image bytes the host already holds, such as a pasted image.
pub fn import_image_bytes(root: &Path, grant: &Grant, bytes: &[u8], name: &str) -> Result<Asset, String> {
    use article_core::host::{ArticleHost, Capability};
    let host = RobrixArticleHost::new(root);
    host.authorize(&grant.lease, Capability::ImportAssets)?;
    Store::new(&host, &grant.lease).import_image(bytes, name)
}

#[cfg(test)]
mod native_import_tests {
    use super::*;
    #[test]
    fn relative_imports_decode_urls_normalize_images_and_stay_in_chosen_folder() {
        let root=std::env::temp_dir().join(format!("rinx-relative-{}",new_id()));
        std::fs::create_dir_all(root.join("article/images")).unwrap();
        let origin=root.join("article/source.md");
        std::fs::write(&origin,b"![Local](images/local%20image.svg)").unwrap();
        let origin=origin.canonicalize().unwrap();
        std::fs::write(root.join("article/images/local image.svg"),br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="40" height="20" fill="#00cc44"/></svg>"##).unwrap();
        std::fs::write(root.join("outside.md"),b"outside").unwrap();
        let (image,name)=read_relative_image(&origin,"images/local%20image.svg").unwrap();
        assert_eq!(name,"local image.svg");assert_eq!((image.width,image.height),(40,20));assert!(image.png.starts_with(b"\x89PNG"));
        assert_eq!(resolve_relative(&origin,"source.md#heading").unwrap(),origin);
        assert!(resolve_relative(&origin,"../outside.md").is_err());
        assert!(resolve_relative(&origin,"https://example.org/image.png").is_err());
        #[cfg(unix)] {
            std::os::unix::fs::symlink(root.join("outside.md"),root.join("article/escape.md")).unwrap();
            assert!(resolve_relative(&origin,"escape.md").is_err());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
