#![cfg(windows)]

use std::{
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::PathBuf,
    process::Command,
};

#[test]
fn long_directory_keeps_identity_or_refuses_when_short_names_are_disabled() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    assert_eq!(
        hagency_platform::process_current_directory(&root).unwrap(),
        root
    );
    let mut long = root;
    while long.as_os_str().encode_wide().count() < 300 {
        long = long.join("owner_0123456789abcdef0123456789abcdef");
    }
    std::fs::create_dir_all(&long).unwrap();
    match hagency_platform::process_current_directory(&long) {
        Ok(short) => {
            assert!(short.as_os_str().encode_wide().count() < 260);
            assert_eq!(short.canonicalize().unwrap(), long.canonicalize().unwrap());
            let marker = temp.path().join("actual-cwd.json");
            // Exercise the actual native executable. cmd.exe applies its own
            // legacy expanded-path limit after CreateProcess has succeeded.
            let output = Command::new(env!("CARGO_BIN_EXE_hagency-platform-probe"))
                .arg("current-directory")
                .arg(&marker)
                .current_dir(&short)
                .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
                .output()
                .unwrap();
            assert!(output.status.success(), "cwd={short:?}; {output:?}");
            let actual: PathBuf = serde_json::from_slice(&std::fs::read(marker).unwrap()).unwrap();
            assert_eq!(actual.canonicalize().unwrap(), long.canonicalize().unwrap());
        }
        Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput),
    }
}
