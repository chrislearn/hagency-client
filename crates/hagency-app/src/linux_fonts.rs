//! Honor Fontconfig's installed sans-serif family for Simplified Chinese.
use std::{path::Path, process::Command};
use article_makepad::system_fonts::install_faces;
use makepad_widgets::{
    *,
    makepad_draw::text::{font_face::FontFace, loader::FontDefinition},
};

const REGULAR: &str = "fontconfig-zh-cn-regular";
const SEMIBOLD: &str = "fontconfig-zh-cn-semibold";

pub(crate) fn install(vm: &mut ScriptVm) {
    if let Err(error) = install_faces(vm, [REGULAR, SEMIBOLD], load) {
        log!(
            "Chinese system sans unavailable; using bundled fallback: {}",
            error
        );
    }
}

fn load(name: &str) -> Result<FontDefinition, String> {
    let weight = if name == SEMIBOLD {
        "demibold"
    } else {
        "regular"
    };
    let query = format!("sans-serif:lang=zh-cn:charset=4e2d:weight={weight}");
    let output = Command::new("fc-match")
        .args(["-f", "%{index}\n%{file}", &query])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("fc-match failed for {query}"));
    }
    let selection = std::str::from_utf8(&output.stdout).map_err(|e| e.to_string())?;
    let (index, path) = selection
        .split_once('\n')
        .ok_or("missing Fontconfig face")?;
    // Fontconfig stores the variable-font named instance in the high word.
    // FontMember selects the wght axis for each text role.
    let index = index.parse::<u32>().map_err(|e| e.to_string())? & 0xFFFF;
    let data = SharedBytes::from_file_mmap_or_read(Path::new(path)).map_err(|e| e.to_string())?;
    let face =
        FontFace::from_data_and_index(data.clone(), index).ok_or("invalid Fontconfig face")?;
    if !face.with_ttf_parser_face(|face| "中文".chars().all(|c| face.glyph_index(c).is_some())) {
        return Err("installed sans-serif family has no Chinese glyphs".into());
    }
    Ok(FontDefinition {
        data,
        index,
        ascender_fudge_in_ems: 0.0,
        descender_fudge_in_ems: 0.0,
        weight: None,
        variations: Vec::new(),
    })
}
