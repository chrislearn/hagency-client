//! Installed Simplified Chinese UI faces supplied by CoreText.
use makepad_widgets::{makepad_draw::text::loader::FontDefinition, *};
use std::{
    ffi::{CString, OsString, c_char, c_void},
    os::unix::ffi::OsStringExt,
    path::PathBuf,
    ptr,
};

const REGULAR: &str = "PingFangSC-Regular";
const SEMIBOLD: &str = "PingFangSC-Semibold";

pub fn install(vm: &mut ScriptVm) {
    // Some Macs have Hiragino Sans GB installed without the downloadable PingFang asset.
    if let Err(error) =
        crate::system_fonts::install_faces(vm, [REGULAR, SEMIBOLD], load_system_face)
    {
        if crate::system_fonts::install_faces(
            vm,
            ["HiraginoSansGB-W3", "HiraginoSansGB-W6"],
            load_system_face,
        )
        .is_err()
        {
            log!(
                "Chinese UI fonts unavailable; using bundled fallback: {}",
                error
            );
        }
    }
}

fn load_system_face(name: &str) -> Result<FontDefinition, String> {
    crate::system_fonts::collection_face(&system_font_path(name)?, &[name])
}

// Small, private bindings to stable public APIs available on both Apple targets.
type CFRef = *const c_void;
#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    fn CTFontCreateWithName(name: CFRef, size: f64, matrix: *const c_void) -> CFRef;
    fn CTFontCopyPostScriptName(font: CFRef) -> CFRef;
    fn CTFontCopyAttribute(font: CFRef, attribute: CFRef) -> CFRef;
    static kCTFontURLAttribute: CFRef;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(allocator: CFRef, text: *const c_char, encoding: u32) -> CFRef;
    fn CFEqual(a: CFRef, b: CFRef) -> u8;
    fn CFGetTypeID(value: CFRef) -> usize;
    fn CFURLGetTypeID() -> usize;
    fn CFURLGetFileSystemRepresentation(
        url: CFRef,
        resolve: u8,
        buffer: *mut u8,
        length: isize,
    ) -> u8;
    fn CFRelease(value: CFRef);
}

struct OwnedCf(CFRef);
impl OwnedCf {
    fn checked(value: CFRef) -> Result<Self, String> {
        if value.is_null() {
            Err("CoreText returned no font resource".into())
        } else {
            Ok(Self(value))
        }
    }
}
impl Drop for OwnedCf {
    fn drop(&mut self) {
        // SAFETY: each instance owns one non-null Create/Copy-rule reference.
        unsafe { CFRelease(self.0) }
    }
}

fn system_font_path(name: &str) -> Result<PathBuf, String> {
    let name = CString::new(name).map_err(|error| error.to_string())?;
    // SAFETY: CoreFoundation objects remain alive throughout their use. All
    // pointers are checked, the attribute's type is validated, and CFURL receives
    // the actual writable buffer length. RAII releases every owned reference.
    unsafe {
        let requested = OwnedCf::checked(CFStringCreateWithCString(
            ptr::null(),
            name.as_ptr(),
            0x08000100,
        ))?;
        let font = OwnedCf::checked(CTFontCreateWithName(requested.0, 16.0, ptr::null()))?;
        let actual = OwnedCf::checked(CTFontCopyPostScriptName(font.0))?;
        if CFEqual(requested.0, actual.0) == 0 {
            return Err("CoreText substituted a different font".into());
        }
        let url = OwnedCf::checked(CTFontCopyAttribute(font.0, kCTFontURLAttribute))?;
        if CFGetTypeID(url.0) != CFURLGetTypeID() {
            return Err("font URL is not a CFURL".into());
        }
        let mut buffer = vec![0; libc::PATH_MAX as usize];
        if CFURLGetFileSystemRepresentation(url.0, 1, buffer.as_mut_ptr(), buffer.len() as isize)
            == 0
        {
            return Err("font URL has no readable filesystem path".into());
        }
        let length = buffer
            .iter()
            .position(|byte| *byte == 0)
            .ok_or("unterminated font path")?;
        buffer.truncate(length);
        Ok(PathBuf::from(OsString::from_vec(buffer)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use makepad_widgets::makepad_draw::text::{font_face::FontFace, fonts::Fonts};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn system_pingfang_faces_cover_english_and_chinese() {
        let mut indices = Vec::new();
        for name in [REGULAR, SEMIBOLD] {
            let definition = load_system_face(name).expect("installed PingFang face");
            indices.push(definition.index);
            let face = FontFace::from_data_and_index(definition.data, definition.index).unwrap();
            face.with_ttf_parser_face(|face| {
                for character in "Robrix Chats 0123 中文聊天简体繁體，。！？".chars() {
                    assert!(
                        face.glyph_index(character).is_some(),
                        "{name} missing {character}"
                    );
                    if !character.is_whitespace() {
                        assert!(
                            face.glyph_bounding_box(face.glyph_index(character).unwrap())
                                .is_some(),
                            "{name} has no drawable outline for {character}"
                        );
                    }
                }
            });
        }
        assert_ne!(
            indices[0], indices[1],
            "regular and semibold must select distinct TTC faces"
        );
    }

    #[test]
    fn missing_font_does_not_silently_use_coretext_substitution() {
        assert!(system_font_path("Robrix-DeliberatelyMissingFont").is_err());
    }

    #[test]
    fn theme_prefers_pingfang_and_preserves_fallback_after_widget_registration() {
        use makepad_widgets::desktop_style::{self, DesktopStyle, StyleSheet};
        for (dark, late_install) in [(false, false), (true, false), (false, true)] {
            let mut cx = Cx::new(Box::new(|_, _| {}));
            cx.with_vm(|vm| {
                makepad_widgets::theme_mod(vm);
                script_eval!(vm, { mod.theme = mod.themes.light });
                desktop_style::install(
                    vm,
                    StyleSheet::load_with_appearance(DesktopStyle::Macos, dark),
                );
                if !late_install {
                    install(vm);
                }
                makepad_widgets::widgets_mod(vm);
                desktop_style::apply_widgets(vm);
                install(vm);
                let italic = script_eval!(vm, { mod.theme.font_italic });
                let italic = TextStyle::script_from_value(vm, italic);
                assert_eq!(
                    italic.font_family.member_ids().next(),
                    Some("ibm_plex_italic")
                );
                assert!(italic.font_family.member_ids().any(|id| id == "system_ui"));
                let code = script_eval!(vm, { mod.theme.font_code });
                let code = TextStyle::script_from_value(vm, code);
                assert!(!code.font_family.member_ids().any(|id| id == "system_ui"));
                for (value, expected) in [
                    (script_eval!(vm, { mod.theme.font_regular }), REGULAR),
                    (script_eval!(vm, { mod.theme.font_bold }), SEMIBOLD),
                    (
                        script_eval!(vm, { mod.widgets.Label.draw_text.text_style }),
                        REGULAR,
                    ),
                    (
                        script_eval!(vm, { mod.widgets.TextInput.draw_text.text_style }),
                        REGULAR,
                    ),
                ] {
                    let style = TextStyle::script_from_value(vm, value);
                    assert!(
                        style
                            .font_family
                            .member_ids()
                            .any(|id| id == "noto_color_emoji")
                    );
                    vm.with_cx_mut(|cx| {
                        style.ensure_fonts_loaded(cx);
                        cx.script_data.resources.bump_generation();
                        style.ensure_fonts_loaded(cx);
                    });
                    let fonts = vm.with_cx_mut(|cx| cx.get_global::<Rc<RefCell<Fonts>>>().clone());
                    let family = fonts
                        .borrow_mut()
                        .get_or_load_font_family(style.font_family_id());
                    assert_eq!(
                        style
                            .font_family
                            .member_ids()
                            .filter(|id| *id == "system_ui")
                            .count(),
                        1
                    );
                    assert!(family.fonts().len() > 1);
                    let shaped = family.get_or_shape("English 中文聊天繁體 0123".into());
                    assert!(!shaped.glyphs.is_empty());
                    for glyph in &shaped.glyphs {
                        assert_ne!(glyph.id, 0);
                        let face =
                            FontFace::from_data_and_index(glyph.font.data().clone(), 0).unwrap();
                        face.with_ttf_parser_face(|face| {
                            assert!(
                                face.names().into_iter().any(|n| n.name_id == 6
                                    && n.to_string().as_deref() == Some(expected))
                            );
                        });
                    }
                }
            });
        }
    }
}
