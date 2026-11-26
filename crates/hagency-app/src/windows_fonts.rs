//! Microsoft YaHei UI shares a TTC with the print-oriented faces.
use std::path::PathBuf;
use article_makepad::system_fonts::{collection_face, install_faces};
use makepad_widgets::{*, makepad_draw::text::loader::FontDefinition};

const REGULAR: &str = "MicrosoftYaHeiUI";
const BOLD: &str = "MicrosoftYaHeiUI-Bold";

pub(crate) fn install(vm: &mut ScriptVm) {
    if let Err(error) = install_faces(vm, [REGULAR, BOLD], load) {
        log!(
            "Microsoft YaHei unavailable; using bundled fallback: {}",
            error
        );
    }
}

fn load(name: &str) -> Result<FontDefinition, String> {
    let fonts = PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| r"C:\Windows".into()))
        .join("Fonts");
    let (file, alias) = if name == BOLD {
        ("msyhbd.ttc", "MicrosoftYaHei-Bold")
    } else {
        ("msyh.ttc", "MicrosoftYaHei")
    };
    collection_face(&fonts.join(file), &[name, alias])
}
