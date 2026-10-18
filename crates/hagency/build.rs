//! ADR-189: embed the console build when `HAGENCY_CONSOLE_DIR` names one.
//! Release builds set it; development builds without it serve the console
//! from `--console-assets` only. The embedded files are checked against their
//! own `manifest.json` at start, exactly like a console folder.
use std::{fmt::Write, path::Path};

fn main() {
    println!("cargo:rerun-if-env-changed=HAGENCY_CONSOLE_DIR");
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    let mut code = String::from("pub(crate) static FILES: &[(&str, &[u8])] = &[\n");
    if let Some(dir) = std::env::var_os("HAGENCY_CONSOLE_DIR") {
        let root = Path::new(&dir)
            .canonicalize()
            .expect("HAGENCY_CONSOLE_DIR must name an existing console build");
        println!("cargo:rerun-if-changed={}", root.display());
        let mut files = Vec::new();
        collect(&root, &root, &mut files);
        files.sort();
        assert!(
            files.iter().any(|(rel, _)| rel == "manifest.json"),
            "HAGENCY_CONSOLE_DIR has no manifest.json; build it with mockup/scripts/build-native-console.mjs"
        );
        for (rel, abs) in files {
            println!("cargo:rerun-if-changed={abs}");
            writeln!(code, "    ({rel:?}, include_bytes!({abs:?})),").unwrap();
        }
    }
    code.push_str("];\n");
    std::fs::write(Path::new(&out).join("embedded_console.rs"), code).unwrap();
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            files.push((rel, path.to_string_lossy().into_owned()));
        }
    }
}
