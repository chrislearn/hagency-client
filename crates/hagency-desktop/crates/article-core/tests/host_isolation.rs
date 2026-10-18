use std::{path::{Path, PathBuf}, sync::Mutex, time::Duration};
use article_core::{
    assets::{prepare_image, crop_cover, MAX_FILE},
    document::{Document, Cover, new_id},
    editing::EditHistory,
    host::{ArticleHost, Capabilities, Capability, SessionAuthority},
    storage::LocalStore,
};

struct LocalHost {
    root: PathBuf,
    account: Mutex<Option<String>>,
    authority: SessionAuthority,
}
impl LocalHost {
    fn new() -> Self {
        Self { root: std::env::temp_dir().join(format!("article-host-{}", new_id())),
            account: Mutex::new(Some("local:writer".into())), authority: SessionAuthority::default() }
    }
    fn consent(&self) -> article_core::host::ConsentGrant {
        self.authority.issue(self.active_account().unwrap(), Capabilities::editor(), Duration::from_secs(3600))
    }
}
impl Drop for LocalHost { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); } }
impl ArticleHost for LocalHost {
    fn active_account(&self) -> Option<String> { self.account.lock().unwrap().clone() }
    fn data_root(&self) -> &Path { &self.root }
    fn authority(&self) -> &SessionAuthority { &self.authority }
}
type Store<'a> = LocalStore<'a, LocalHost, serde_json::Value, serde_json::Value>;

#[test]
fn two_hosts_and_accounts_do_not_share_authority_or_data() {
    let first = LocalHost::new();
    let second = LocalHost::new();
    let grant = first.consent();
    let doc = Document::from_markdown("私有文章", "Hello **世界**").unwrap();
    Store::new(&first, &grant).save_document(&doc).unwrap();
    assert_eq!(Store::new(&first, &grant).load().unwrap().documents, vec![doc]);
    assert!(Store::new(&second, &grant).load().is_err()); // same account name, different issuer
    assert!(Store::new(&second, &second.consent()).load().unwrap().documents.is_empty());
    *first.account.lock().unwrap() = Some("local:reader".into());
    assert!(Store::new(&first, &grant).load().is_err());
    assert!(Store::new(&first, &first.consent()).load().unwrap().documents.is_empty());
}

#[test]
fn reader_grant_cannot_read_drafts_import_or_publish() {
    let host = LocalHost::new();
    let grant = host.authority.issue("local:writer".into(), Capabilities::reader(), Duration::from_secs(60));
    assert!(host.authorize(&grant, Capability::ReadPublished).is_ok());
    for cap in [Capability::ReadDrafts, Capability::WriteDrafts, Capability::ReadAssets, Capability::ImportAssets, Capability::Publish] {
        assert!(host.authorize(&grant, cap).is_err());
    }
    assert!(Store::new(&host, &grant).load().is_err());
    assert!(Store::new(&host, &grant).import_image(b"not an image", "image").is_err());
    assert!(!host.root.exists());
}

#[test]
fn logout_expiry_and_revoke_invalidate_clones() {
    let host = LocalHost::new();
    let old = host.consent();
    let clone = old.clone();
    host.authority.invalidate();
    assert!(Store::new(&host, &old).load().is_err());
    assert!(Store::new(&host, &clone).load().is_err());
    let grant = host.consent();
    grant.clone().revoke();
    assert!(Store::new(&host, &grant).load().is_err());
    let expired = host.authority.issue("local:writer".into(), Capabilities::editor(), Duration::ZERO);
    assert!(Store::new(&host, &expired).load().is_err());
}

#[test]
fn account_switch_during_update_does_not_commit() {
    let host = LocalHost::new();
    let grant = host.consent();
    let store = Store::new(&host, &grant);
    let original = Document::from_markdown("Original", "Original body").unwrap();
    store.save_document(&original).unwrap();
    assert!(store.update(|library| {
        library.documents[0].title = "Must not commit".into();
        host.authority.invalidate();
        Ok(())
    }).is_err());
    let current = host.consent();
    assert_eq!(Store::new(&host, &current).load().unwrap().documents[0].title, "Original");
}

fn png() -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(40, 20).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[test]
fn selected_images_are_normalized_integrity_checked_and_account_scoped() {
    let host = LocalHost::new(); let grant = host.consent(); let store = Store::new(&host, &grant);
    let asset = store.import_image(&png(), "封面\n.png").unwrap();
    assert_eq!((asset.width, asset.height), (40, 20));
    assert_eq!(asset.name, "封面.png");
    let bytes = store.asset_bytes(&asset.id).unwrap();
    assert_eq!(asset.id, blake3::hash(&bytes).to_hex().as_str());
    assert!(store.asset_bytes("../private").is_err());
    let cover = Cover { asset: asset.id.clone(), focal_x: 500, focal_y: 500, show_in_article: true };
    let square = image::load_from_memory(&crop_cover(&bytes, &cover, true).unwrap()).unwrap();
    assert_eq!((square.width(), square.height()), (20, 20));
    assert!(prepare_image(b"<svg><script/></svg>", "bad.svg").is_err());
    assert!(prepare_image(&vec![0; MAX_FILE as usize + 1], "large.png").is_err());
    let path = host.root.join("mini-apps").join(blake3::hash(b"local:writer").to_hex().as_str())
        .join("org.octosense.article-editor/assets").join(&asset.id);
    std::fs::write(path, b"corrupt").unwrap();
    assert!(store.asset_bytes(&asset.id).is_err());
}

#[test]
fn existing_schema_and_host_publication_metadata_roundtrip() {
    let host = LocalHost::new(); let grant = host.consent(); let store = Store::new(&host, &grant);
    let publication = serde_json::json!({"room":"!room:example.org","root":"$event","version":2});
    store.update(|library| {
        library.documents.push(Document::from_markdown("旧草稿", "正文").unwrap());
        library.publications.push(publication.clone());
        library.outbox.push(serde_json::json!({"transaction":"keep-this-id"}));
        Ok(())
    }).unwrap();
    let saved = store.load().unwrap();
    assert_eq!(saved.schema, 2);
    assert_eq!(saved.publications, vec![publication]);
    assert_eq!(saved.outbox[0]["transaction"], "keep-this-id");
    let path = host.root.join("mini-apps").join(blake3::hash(b"local:writer").to_hex().as_str())
        .join("org.octosense.article-editor/library-v2.json");
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

#[test]
fn legacy_source_migration_preserves_unsupported_content() {
    let host = LocalHost::new(); let grant = host.consent();
    let path = host.root.join("mini-apps").join(blake3::hash(b"local:writer").to_hex().as_str())
        .join("org.octosense.article-editor/draft.json");
    article_core::storage::atomic_write(&path, br#"{"title":"Old","markdown":"<table><tr><td>preserve me</td></tr></table>"}"#).unwrap();
    let library = Store::new(&host, &grant).load().unwrap();
    assert_eq!(library.documents[0].title, "Old");
    assert_eq!(library.documents[0].markdown(), "<table><tr><td>preserve me</td></tr></table>");
    assert!(library.legacy_source.is_none());
    assert!(path.exists());
}

#[test]
fn unapplied_source_survives_restart_and_is_scoped_to_its_document() {
    let host = LocalHost::new(); let grant = host.consent(); let store = Store::new(&host, &grant);
    let first = Document::from_markdown("First", "Original **body**").unwrap();
    let second = Document::from_markdown("Second", "Other article").unwrap();
    let source = "# 中文\n\n<table><tr><td>原始 👩‍💻</td></tr></table>\n";
    assert_eq!(Document::from_markdown(&first.title, source).unwrap().markdown(), source);
    store.save_document_with_source(&first, Some(source)).unwrap();
    store.save_document_with_source(&second, Some("`unapplied code`")).unwrap();
    let restored = Store::new(&host, &grant).load().unwrap();
    assert_eq!(restored.documents[0], first);
    assert_eq!(restored.source_for(&first.id), Some(source));
    assert_eq!(restored.source_for(&second.id), Some("`unapplied code`"));
    // Ordinary visual autosave does not discard an unapplied source draft.
    store.save_document(&first).unwrap();
    assert_eq!(store.load().unwrap().source_for(&first.id), Some(source));
    store.save_document_with_source(&first, None).unwrap();
    let applied = store.load().unwrap();
    assert_eq!(applied.source_for(&first.id), None);
    assert_eq!(applied.source_for(&second.id), Some("`unapplied code`"));
    assert!(store.save_document_with_source(&second, Some(&"x".repeat(article_core::document::MAX_BODY + 1))).is_err());
    assert_eq!(store.load().unwrap().source_for(&second.id), Some("`unapplied code`"));
}

#[test]
fn legacy_source_belongs_only_to_the_migrated_document() {
    let first = Document::default();
    let second = Document::default();
    let mut library: article_core::storage::Library = serde_json::from_value(serde_json::json!({
        "schema": 2, "documents": [first, second], "assets": {}, "publications": [], "outbox": [],
        "legacy_source": "<p>Original source</p>"
    })).unwrap();
    assert_eq!(library.source_for(&first.id), Some("<p>Original source</p>"));
    assert_eq!(library.source_for(&second.id), None);
    library.clear_source(&second.id);
    assert!(library.legacy_source.is_some());
    library.clear_source(&first.id);
    assert!(library.legacy_source.is_none());
}

#[test]
fn history_restores_content_and_styles_together() {
    let mut doc = Document::from_markdown("文章", "你好世界").unwrap();
    let mut history = EditHistory::default(); history.checkpoint(&doc);
    doc.blocks[0].format(0..6, Some(true), None, None).unwrap();
    let changed = doc.clone();
    assert!(history.undo(&mut doc)); assert!(doc.blocks[0].marks.is_empty());
    assert!(history.redo(&mut doc)); assert_eq!(doc, changed);
}

#[cfg(feature = "l0")]
#[test]
fn same_l0_bindings_work_without_either_application() {
    use article_core::{bindings::*, editing::EditorState};
    let mut state = EditorState::default();
    apply_input(&mut state, "title_changed", "中文 Title").unwrap();
    apply_input(&mut state, "markdown_changed", "## hello").unwrap();
    for chinese in [false, true] {
        let fields = realize_editor(&state, chinese).unwrap();
        assert_eq!(fields[0].text, "中文 Title");
        assert_eq!(fields[0].placeholder, if chinese { "文章标题" } else { "Article title" });
    }
    assert!(apply_input(&mut state, "publish", "arbitrary").is_err());
    let unchanged = state.clone();
    assert!(apply_input(&mut state, "title_changed", &"长".repeat(121)).is_err());
    assert_eq!(state, unchanged);
}
