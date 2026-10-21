//! Use the OS-provided Microsoft YaHei faces on Windows, without shipping them.
//!
//! Mirrors the Apple path (`article-makepad/src/apple_fonts.rs`): resolve the
//! installed TTC faces by PostScript name, register them through Makepad's
//! native font API, then rebuild the text families with the system face first
//! and the bundled faces (IBMPlexSans / LXGWWenKai / emoji) retained as
//! fallback. YaHei's metrics track Latin UI fonts far more closely than
//! LXGWWenKai's, which also fixes CJK text sitting low inside controls.

use std::{cell::RefCell, path::Path, rc::Rc};
use makepad_widgets::{
    *,
    makepad_draw::text::{
        font::FontId,
        font_face::FontFace,
        fonts::Fonts,
        loader::{FontDefinition, FontFamilyDefinition},
    },
};

const REGULAR_FILE: &str = "C:\\Windows\\Fonts\\msyh.ttc";
const SEMIBOLD_FILE: &str = "C:\\Windows\\Fonts\\msyhbd.ttc";
// The UI faces come first: Windows renders its own UI with YaHei UI.
const REGULAR_NAMES: [&str; 2] = ["MicrosoftYaHeiUI", "MicrosoftYaHei"];
const SEMIBOLD_NAMES: [&str; 2] = ["MicrosoftYaHeiUI-Bold", "MicrosoftYaHei-Bold"];

pub(crate) fn install(vm: &mut ScriptVm) {
    let faces = [
        load_collection_face(REGULAR_FILE, &REGULAR_NAMES),
        load_collection_face(SEMIBOLD_FILE, &SEMIBOLD_NAMES),
    ];
    let [Ok(regular), Ok(semibold)] = faces else {
        log!(
            "Microsoft YaHei unavailable; keeping bundled fonts: {:?}",
            faces.map(|face| face.err())
        );
        return;
    };
    vm.with_cx_mut(|cx| {
        CxDraw::lazy_construct_fonts(cx);
    });
    let fonts = vm.with_cx_mut(|cx| cx.get_global::<Rc<RefCell<Fonts>>>().clone());
    for (name, definition) in [
        (REGULAR_NAMES[0], regular),
        (SEMIBOLD_NAMES[0], semibold),
    ] {
        let mut fonts = fonts.borrow_mut();
        let id = FontId::from(name);
        if !fonts.is_font_known(id) {
            fonts.define_font(id, definition);
        }
    }

    let styles = [
        (script_eval!(vm, { mod.theme.font_label }), REGULAR_NAMES[0]),
        (script_eval!(vm, { mod.theme.font_regular }), REGULAR_NAMES[0]),
        (script_eval!(vm, { mod.theme.font_bold }), SEMIBOLD_NAMES[0]),
        (script_eval!(vm, { mod.theme.font_italic }), REGULAR_NAMES[0]),
        (script_eval!(vm, { mod.theme.font_bold_italic }), SEMIBOLD_NAMES[0]),
    ];
    for (value, name) in styles {
        let style = TextStyle::script_from_value(vm, value);
        vm.with_cx_mut(|cx| style.ensure_fonts_loaded(cx));
        let family_id = style.font_family_id();
        let mut fonts = fonts.borrow_mut();
        let fallback = fonts.get_or_load_font_family(family_id);
        let primary = FontId::from(name);
        let font_ids: Vec<_> = std::iter::once(primary)
            .chain(
                fallback
                    .fonts()
                    .iter()
                    .map(|font| font.id())
                    .filter(|id| *id != primary),
            )
            .collect();
        fonts.set_font_family_definition(
            family_id,
            FontFamilyDefinition {
                expected_member_count: font_ids.len(),
                font_ids,
                diagnostics: Default::default(),
            },
        );
    }
    // Code and icon families retain their specialized fonts. YaHei has no
    // italic face; italic prose uses the corresponding upright YaHei weight.
    log!(
        "Windows typography: Microsoft YaHei UI regular/bold for English and Chinese; bundled fallback retained"
    );
}

/// Selects a face from a font collection (or a single-face file) by matching
/// any of the given OpenType PostScript names (name ID 6).
fn load_collection_face(path: &str, names: &[&str]) -> Result<FontDefinition, String> {
    let data = SharedBytes::from_file_mmap_or_read(Path::new(path))
        .map_err(|error| format!("{path}: {error}"))?;
    let count = if data.as_slice().starts_with(b"ttcf") {
        let count = data
            .as_slice()
            .get(8..12)
            .ok_or("truncated font collection")?;
        u32::from_be_bytes(count.try_into().unwrap())
    } else {
        1
    };
    for index in 0..count {
        let Some(face) = FontFace::from_data_and_index(data.clone(), index) else {
            continue;
        };
        let matches = face.with_ttf_parser_face(|face| {
            face.names().into_iter().any(|record| {
                record.name_id == 6
                    && record
                        .to_string()
                        .as_deref()
                        .is_some_and(|name| names.contains(&name))
            })
        });
        if matches {
            return Ok(FontDefinition {
                data,
                index,
                ascender_fudge_in_ems: 0.0,
                descender_fudge_in_ems: 0.0,
                weight: None,
                variations: Vec::new(),
            });
        }
    }
    Err(format!("{path}: none of {names:?} found in the collection"))
}
