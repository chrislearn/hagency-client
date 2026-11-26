//! System faces belong in Makepad's declared font family. A cache-only override
//! is lost on the next draw when the renderer reconciles its font members.
use makepad_widgets::{
    makepad_draw::text::{font_face::FontFace, loader::FontDefinition},
    makepad_platform::script::res::{CxScriptResource, CxScriptResourceData, CxScriptResourceGc},
    *,
};
use std::{path::Path, rc::Rc};

pub fn install_faces(
    vm: &mut ScriptVm,
    names: [&str; 2],
    load: fn(&str) -> Result<FontDefinition, String>,
) -> Result<(), String> {
    let regular = system_face_resource(vm, names[0], load)?;
    let semibold = system_face_resource(vm, names[1], load)?;
    // widgets_mod selects the light/dark base before registering widgets.
    // Declare both bases so the chosen appearance inherits the same fonts.
    // A stylesheet's later explicit font choice (e.g. mono) still wins.
    let themes = [
        script_eval!(vm, { mod.themes.light }),
        script_eval!(vm, { mod.themes.dark }),
        script_eval!(vm, { mod.theme }),
    ];
    let mut seen = Vec::new();
    let member_proto = script_eval!(vm, { mod.text.FontMember });
    for theme in themes {
        let Some(theme) = theme.as_object() else {
            continue;
        };
        if seen.contains(&theme) {
            continue;
        }
        seen.push(theme);
        let code = vm.bx.heap.value(theme, id!(font_code).into(), NoTrap);
        let code = TextStyle::script_from_value(vm, code);
        for (role, resource, weight) in [
            (id!(font_label), regular, 400.0),
            (id!(font_regular), regular, 400.0),
            (id!(font_bold), semibold, 600.0),
            (id!(font_italic), regular, 400.0),
            (id!(font_bold_italic), semibold, 600.0),
        ] {
            let previous = vm.bx.heap.value(theme, role.into(), NoTrap);
            let Some(previous_obj) = previous.as_object() else {
                continue;
            };
            if TextStyle::script_from_value(vm, previous).font_family == code.font_family {
                continue;
            }
            let previous_family = vm
                .bx
                .heap
                .value(previous_obj, id!(font_family).into(), NoTrap);
            let Some(previous_family) = previous_family.as_object() else {
                continue;
            };
            let members: Vec<_> = (0..vm.bx.heap.vec_len(previous_family))
                .map(|i| vm.bx.heap.vec_key_value(previous_family, i, NoTrap))
                .collect();
            if members
                .iter()
                .any(|member| member.key == id_lut!(system_ui).into())
            {
                continue;
            }
            // Apple and Windows already select their UI face for mixed-script text.
            // Fontconfig only supplies Linux's missing Han glyphs. Custom and italic
            // Latin primaries on all platforms keep their original position.
            let insertion = members.iter().position(|member| {
                (!cfg!(target_os = "linux")
                    && (member.key == id!(ibm_plex_text).into()
                        || member.key == id!(ibm_plex_semibold).into()))
                    || member.key == id!(lxgw_wenkai_regular).into()
                    || member.key == id!(lxgw_wenkai_bold).into()
            });
            let Some(insertion) = insertion else { continue };
            let primary = vm.bx.heap.new_with_proto(member_proto);
            vm.bx
                .heap
                .set_value(primary, id!(res).into(), resource, NoTrap);
            vm.bx
                .heap
                .set_value(primary, id!(weight).into(), weight.into(), NoTrap);
            // Splash builds stock widget templates before the host's isolate hook.
            // Update the family they already hold. A fresh style object here would
            // only reach templates registered afterwards.
            for _ in insertion..members.len() {
                vm.bx.heap.vec_pop(previous_family, NoTrap);
            }
            vm.bx.heap.vec_push(
                previous_family,
                id_lut!(system_ui).into(),
                primary.into(),
                NoTrap,
            );
            for member in &members[insertion..] {
                vm.bx
                    .heap
                    .vec_push(previous_family, member.key, member.value, NoTrap);
            }
        }
    }
    Ok(())
}

fn system_face_resource(
    vm: &mut ScriptVm,
    name: &str,
    load: fn(&str) -> Result<FontDefinition, String>,
) -> Result<ScriptValue, String> {
    let path = format!("hagency-system-font://{name}");
    let heap_key = vm.bx.heap.heap_key();
    if let Some(handle) = vm
        .cx()
        .script_data
        .resources
        .get_handle_by_abs_path(heap_key, &path)
    {
        return Ok(handle.into());
    }
    let data = if vm
        .cx()
        .script_data
        .resources
        .resources
        .borrow()
        .iter()
        .any(|r| r.abs_path == path)
    {
        None
    } else {
        let face = load(name)?;
        Some(Rc::new(face_at_index_zero(&face)?))
    };
    let resource_type = vm.handle_type(id_lut!(res));
    let resources = &vm.cx().script_data.resources;
    let gc = CxScriptResourceGc {
        resources: resources.resources.clone(),
        handles_by_abs_path: resources.handles_by_abs_path.clone(),
        handle: ScriptHandle::ZERO,
        heap_key,
    };
    let handle = vm.bx.heap.new_handle(resource_type, Box::new(gc));
    let resources = &vm.cx().script_data.resources;
    if !resources.attach_handle_for_path(heap_key, &path, handle) {
        resources.insert_resource(
            heap_key,
            CxScriptResource {
                abs_path: path,
                dependency_path: None,
                web_url: None,
                data: CxScriptResourceData::Loaded(data.expect("new system font resource")),
                handles: vec![(heap_key, handle)],
            },
        );
    }
    Ok(handle.into())
}

/// FontMember loads collection index zero, so the requested face is handed over as a font of
/// its own: the face's table directory with its tables packed behind it. Table data and table
/// checksums stay intact. Repointing the collection's first directory entry would also work,
/// but the text stack keeps several private copies of every font it opens, and PingFang's
/// collection is 78 MB where one face is 13 MB.
/// This is an in-memory view of a local font, never a shipped or saved asset.
fn face_at_index_zero(face: &FontDefinition) -> Result<Vec<u8>, String> {
    let source = face.data.as_slice();
    let data = if source.starts_with(b"ttcf") {
        let count = be_u32(source, 8).ok_or("truncated font collection")?;
        if face.index >= count {
            return Err("font collection index out of range".into());
        }
        let entry = 12 + usize::try_from(face.index).map_err(|_| "font index overflow")? * 4;
        let directory = be_u32(source, entry).ok_or("truncated font collection directory")?;
        let directory = usize::try_from(directory).map_err(|_| "font directory overflow")?;
        standalone_face(source, directory)?
    } else if face.index != 0 {
        return Err("non-collection font has a nonzero index".into());
    } else {
        source.to_vec()
    };
    Ok(data)
}

fn be_u32(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// The collection face whose table directory starts at `directory`, as a single font: the
/// directory, then each table at a new offset, padded to four bytes as the format asks.
fn standalone_face(collection: &[u8], directory: usize) -> Result<Vec<u8>, String> {
    const HEADER: usize = 12;
    const RECORD: usize = 16;
    let header = collection
        .get(directory..directory.saturating_add(HEADER))
        .ok_or("truncated font directory")?;
    let tables = usize::from(u16::from_be_bytes([header[4], header[5]]));
    let records_at = directory.saturating_add(HEADER);
    let records = collection
        .get(records_at..records_at.saturating_add(tables * RECORD))
        .ok_or("truncated font table records")?;
    let mut spans = Vec::with_capacity(tables);
    let mut size = HEADER + tables * RECORD;
    for i in 0..tables {
        let record = i * RECORD;
        let offset = be_u32(records, record + 8).ok_or("truncated font table records")? as usize;
        let length = be_u32(records, record + 12).ok_or("truncated font table records")? as usize;
        let table = collection
            .get(offset..offset.saturating_add(length))
            .ok_or("font table out of range")?;
        size = size.saturating_add((length + 3) & !3);
        spans.push(table);
    }
    if u32::try_from(size).is_err() {
        return Err("font face too large".into());
    }
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(header);
    out.extend_from_slice(records);
    for (i, table) in spans.into_iter().enumerate() {
        let offset_field = HEADER + i * RECORD + 8;
        let at = (out.len() as u32).to_be_bytes();
        out[offset_field..offset_field + 4].copy_from_slice(&at);
        out.extend_from_slice(table);
        let padded = (out.len() + 3) & !3;
        out.resize(padded, 0);
    }
    Ok(out)
}

/// Selects a face from a font collection (or a single-face file) by matching
/// any of the given OpenType PostScript names (name ID 6).
pub fn collection_face(path: &Path, names: &[&str]) -> Result<FontDefinition, String> {
    let data = SharedBytes::from_file_mmap_or_read(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let count = if data.as_slice().starts_with(b"ttcf") {
        let count = data
            .as_slice()
            .get(8..12)
            .ok_or("truncated font collection")?;
        u32::from_be_bytes(count.try_into().unwrap())
    } else {
        1
    };
    for requested in names {
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
                            .is_some_and(|name| name == *requested)
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
    }
    Err(format!(
        "{}: none of {names:?} found in the collection",
        path.display()
    ))
}
