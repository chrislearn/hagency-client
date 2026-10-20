//! A copied, headless instance waits for the GUI process before running NSIS.
use ::windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        UI::{
            Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
    core::PCWSTR,
};
use anyhow::{Context, Result, ensure};
use hagency_updater::{Staged, atomic_write, public_key, stable_version};
use std::{
    ffi::OsString,
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};

const FLAG: &str = "--hagency-apply-update";
const COMMIT: &str = "--hagency-update-committed";
const CREATE_NO_WINDOW: u32 = 0x08000000;

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
fn wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}

pub(super) struct Helper {
    child: Child,
    directory: PathBuf,
    started: Instant,
    committed: bool,
}
impl Helper {
    pub(super) fn spawn(root: &Path) -> Result<Self> {
        let exe = std::env::current_exe()?;
        let directory = root.join(format!("apply-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory)?;
        let copied = directory.join("hagency-update-helper.exe");
        std::fs::copy(&exe, &copied)?;
        // Direct arguments, never a shell expression. The helper is outside the
        // installation tree so NSIS can replace the complete application.
        let child = Command::new(copied)
            .arg(FLAG)
            .arg(root)
            .arg(std::process::id().to_string())
            .arg(exe)
            .arg(&directory)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
        Ok(Self {
            child,
            directory,
            started: Instant::now(),
            committed: false,
        })
    }
    pub(super) fn ready(&mut self) -> bool {
        self.directory.join("ready").is_file() && self.child.try_wait().ok().flatten().is_none()
    }
    pub(super) fn failed(&mut self) -> bool {
        self.started.elapsed() > Duration::from_secs(120)
            || self.child.try_wait().ok().flatten().is_some()
    }
    pub(super) fn commit(&mut self) {
        self.committed = true;
    }
}
impl Drop for Helper {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub(super) fn run_if_requested() -> Option<Result<()>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(FLAG)) {
        return None;
    }
    let values: Vec<_> = args.collect();
    Some((|| {
        ensure!(values.len() == 4, "Invalid update helper arguments");
        let root = PathBuf::from(&values[0]);
        let result = apply(&root, &values);
        let outcome = if result.is_ok() {
            b"success".as_slice()
        } else {
            b"failed".as_slice()
        };
        let _ = atomic_write(&root.join("install-result"), outcome);
        result
    })())
}

fn apply(root: &Path, args: &[OsString]) -> Result<()> {
    let expected_root = crate::app_data_dir().join("updates").canonicalize()?;
    ensure!(
        root.canonicalize()? == expected_root,
        "Update state directory changed"
    );
    let target = PathBuf::from(&args[2]);
    let helper_dir = PathBuf::from(&args[3]);
    ensure!(
        target.is_absolute() && target.file_name() == Some(std::ffi::OsStr::new("hagency.exe")),
        "Invalid installation target"
    );
    ensure!(
        helper_dir.canonicalize()?.parent() == Some(expected_root.as_path()),
        "Invalid helper directory"
    );
    ensure!(
        std::env::current_exe()?.canonicalize()?.parent()
            == Some(helper_dir.canonicalize()?.as_path()),
        "Helper must run outside the installation"
    );
    let pid: u32 = args[1]
        .to_str()
        .context("Invalid parent process")?
        .parse()?;
    let parent = Handle(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid)? });
    let key = public_key(super::worker::PUBLIC_KEY)?;
    let version = stable_version(env!("CARGO_PKG_VERSION"))?;
    let staged = Staged::load(root, &key, &version)?.context("No valid update is staged")?;
    let _verified = staged.verified_file(&key)?;
    atomic_write(&helper_dir.join("ready"), b"ready")?;
    // Abort if the GUI cannot finish shutdown. Never force-kill it.
    ensure!(
        unsafe { WaitForSingleObject(parent.0, 180_000) } == WAIT_OBJECT_0,
        "Hagency did not exit in time"
    );
    // Main writes this only after the real Makepad shutdown has completed.
    ensure!(
        helper_dir.join("committed").is_file(),
        "Update was canceled before shutdown completed"
    );
    let installer = wide(staged.directory.join("installer.exe"));
    // NSIS requires /D to be last and without quotes, including paths with spaces.
    let mut parameters = OsString::from("/P /D=");
    parameters.push(target.parent().context("Missing installation directory")?);
    let parameters = wide(parameters);
    let verb = wide("open");
    let mut execution = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(installer.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    let installed: Result<bool> = (|| unsafe {
        ShellExecuteExW(&mut execution)?;
        ensure!(
            !execution.hProcess.is_invalid(),
            "Installer did not return a process handle"
        );
        let process = Handle(execution.hProcess);
        ensure!(
            WaitForSingleObject(process.0, 30 * 60 * 1000) == WAIT_OBJECT_0,
            "Installer has not finished"
        );
        let mut code = 1;
        ::windows::Win32::System::Threading::GetExitCodeProcess(process.0, &mut code)?;
        Ok(code == 0)
    })();
    if installed.as_ref().is_ok_and(|success| *success) {
        atomic_write(&root.join("install-result"), b"success")?;
    } else {
        atomic_write(&root.join("install-result"), b"failed")?;
    }
    // Restart through this unelevated helper, rather than NSIS /R. Preserve
    // the user's data-directory overrides without inheriting administrator rights.
    if target.is_file() {
        Command::new(&target)
            .arg(COMMIT)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
    }
    ensure!(installed?, "Installation was canceled or failed");
    Ok(())
}

/// Called from main after app_main returns and all Shutdown handlers have run.
pub(super) fn commit_after_shutdown() {
    if let Some(directory) = COMMITTED_HELPER.lock().unwrap().take() {
        let _ = atomic_write(&directory.join("committed"), b"committed");
    }
}
static COMMITTED_HELPER: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

impl Helper {
    pub(super) fn record_commit(&mut self) {
        *COMMITTED_HELPER.lock().unwrap() = Some(self.directory.clone());
        self.commit();
    }
}
