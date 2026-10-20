//! Win32 shell adapter. All HWND operations run on the window's UI thread.
use std::{cell::Cell, os::windows::ffi::OsStrExt};
use makepad_widgets::{SignalToUI, WindowId};
use ::windows::{
    core::{PCWSTR, w},
    Win32::{
        Foundation::*,
        System::{
            Registry::*,
            Threading::{CreateMutexW, GetCurrentThreadId},
        },
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};
use super::{DesktopPreferences, TrayAction};

const CALLBACK: u32 = WM_APP + 510;
const HIDE: u32 = WM_APP + 511;
const SHOW: u32 = WM_APP + 512;
const SUBCLASS_ID: usize = 0x484147;
// Shell's NIN_KEYSELECT macro is not emitted by windows-rs metadata.
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;
const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

thread_local! {
    static ACTION: Cell<Option<TrayAction>> = const { Cell::new(None) };
    static PREFERENCES: Cell<DesktopPreferences> = Cell::new(DesktopPreferences::default());
    static ICON: Cell<HICON> = const { Cell::new(HICON(std::ptr::null_mut())) };
    static READY: Cell<bool> = const { Cell::new(false) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
    static ACTIVATE_INSTANCE: Cell<u32> = const { Cell::new(0) };
}

fn instance_name() -> Vec<u16> {
    // Explicit profiles stay independent; launches for the same profile restore
    // the resident window instead of adding another icon and SDK session.
    let path: Vec<u8> = crate::app_data_dir()
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect();
    wide(&format!(
        "Local\\Hagency.Desktop.{}",
        blake3::hash(&path).to_hex()
    ))
}

pub(super) struct InstanceGuard(HANDLE);
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub(super) fn acquire_instance() -> anyhow::Result<Option<InstanceGuard>> {
    let name = instance_name();
    unsafe {
        let handle = CreateMutexW(None, false, PCWSTR(name.as_ptr()))?;
        let existing = GetLastError() == ERROR_ALREADY_EXISTS;
        let guard = InstanceGuard(handle);
        if existing {
            let message = RegisterWindowMessageW(PCWSTR(name.as_ptr()));
            anyhow::ensure!(
                message != 0,
                "Could not register the application activation message"
            );
            let _ = AllowSetForegroundWindow(ASFW_ANY);
            PostMessageW(Some(HWND_BROADCAST), message, WPARAM(0), LPARAM(0))?;
            return Ok(None);
        }
        Ok(Some(guard))
    }
}

pub(super) fn take_action() -> Option<TrayAction> {
    ACTION.take()
}

fn action(value: TrayAction) {
    ACTION.set(Some(value));
    SignalToUI::set_ui_signal();
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn startup_command() -> anyhow::Result<Vec<u16>> {
    let path = std::env::current_exe()?;
    let mut value = vec![b'"' as u16];
    value.extend(path.as_os_str().encode_wide());
    value.extend(wide("\""));
    Ok(value)
}

pub(super) fn autostart_enabled() -> anyhow::Result<bool> {
    autostart_enabled_at(RUN_KEY)
}

fn autostart_enabled_at(key: PCWSTR) -> anyhow::Result<bool> {
    unsafe {
        let mut bytes = 0;
        let result = RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            w!("Hagency"),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut bytes),
        );
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        result.ok()?;
        let mut value = vec![0u16; (bytes as usize).div_ceil(2)];
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            w!("Hagency"),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
        .ok()?;
        let expected = startup_command()?;
        // RegGetValue may reserve another terminator when sizing a string.
        Ok(value.split(|c| *c == 0).next() == expected.split(|c| *c == 0).next())
    }
}

pub(super) fn set_autostart(enabled: bool) -> anyhow::Result<()> {
    set_autostart_at(RUN_KEY, enabled)
}

fn set_autostart_at(key: PCWSTR, enabled: bool) -> anyhow::Result<()> {
    unsafe {
        if enabled {
            let value = startup_command()?;
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key,
                w!("Hagency"),
                REG_SZ.0,
                Some(value.as_ptr().cast()),
                (value.len() * 2) as u32,
            )
            .ok()?;
        } else {
            let result = RegDeleteKeyValueW(HKEY_CURRENT_USER, key, w!("Hagency"));
            if result != ERROR_FILE_NOT_FOUND {
                result.ok()?;
            }
        }
    }
    Ok(())
}

pub(super) struct Tray {
    hwnd: HWND,
    icon: HICON,
}

impl Tray {
    pub(super) fn new(
        window: WindowId,
        preferences: DesktopPreferences,
    ) -> anyhow::Result<Option<Self>> {
        let mut search = WindowSearch {
            id: window,
            hwnd: HWND::default(),
        };
        // Match both the backend class and WindowId; secondary windows must never
        // acquire the main window's tray or intercept its minimize command.
        unsafe {
            let _ = EnumThreadWindows(
                GetCurrentThreadId(),
                Some(find_window),
                LPARAM((&mut search as *mut WindowSearch) as isize),
            );
        }
        if search.hwnd.is_invalid() {
            return Ok(None);
        }
        Self::attach(search.hwnd, preferences).map(Some)
    }

    fn attach(hwnd: HWND, preferences: DesktopPreferences) -> anyhow::Result<Self> {
        unsafe {
            let icon = CreateIconFromResourceEx(
                include_bytes!("../../resources/icon_32.png"),
                true,
                0x00030000,
                32,
                32,
                LR_DEFAULTCOLOR,
            )?;
            ICON.set(icon);
            PREFERENCES.set(preferences);
            TASKBAR_CREATED.set(RegisterWindowMessageW(w!("TaskbarCreated")));
            ACTIVATE_INSTANCE.set(RegisterWindowMessageW(PCWSTR(instance_name().as_ptr())));
            if !SetWindowSubclass(hwnd, Some(subclass), SUBCLASS_ID, 0).as_bool() {
                let _ = DestroyIcon(icon);
                anyhow::bail!("Could not install tray callback");
            }
            let tray = Self { hwnd, icon };
            if !add_icon(tray.hwnd) {
                anyhow::bail!("Shell_NotifyIconW failed");
            }
            if preferences.start_hidden {
                tray.hide();
            }
            Ok(tray)
        }
    }

    pub(super) fn update(&self, preferences: DesktopPreferences) {
        PREFERENCES.set(preferences);
    }

    pub(super) fn hide(&self) -> bool {
        READY.get() && unsafe { PostMessageW(Some(self.hwnd), HIDE, WPARAM(0), LPARAM(0)).is_ok() }
    }

    pub(super) fn is_ready(&self) -> bool {
        READY.get()
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        READY.set(false);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &icon_data(self.hwnd));
            let _ = RemoveWindowSubclass(self.hwnd, Some(subclass), SUBCLASS_ID);
            let _ = DestroyIcon(self.icon);
        }
        ICON.set(HICON::default());
    }
}

struct WindowSearch {
    id: WindowId,
    hwnd: HWND,
}

unsafe extern "system" fn find_window(hwnd: HWND, data: LPARAM) -> ::windows::core::BOOL {
    // EnumThreadWindows is synchronous and `search` outlives every invocation.
    let search = unsafe { &mut *(data.0 as *mut WindowSearch) };
    let mut class = [0u16; 64];
    let length = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
    if class[..length] == "MakepadWindow".encode_utf16().collect::<Vec<_>>() {
        // The pinned Makepad backend stores a live Win32Window here, only
        // after native initialization. Never interpret another class's data.
        let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) }
            as *const makepad_widgets::makepad_platform::os::windows::win32_window::Win32Window;
        if !ptr.is_null() && unsafe { (*ptr).window_id } == search.id {
            search.hwnd = hwnd;
            return false.into();
        }
    }
    true.into()
}

fn icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: CALLBACK,
        hIcon: ICON.get(),
        ..Default::default()
    };
    let tip = wide("Hagency");
    data.szTip[..tip.len()].copy_from_slice(&tip);
    data
}

unsafe fn add_icon(hwnd: HWND) -> bool {
    let mut data = icon_data(hwnd);
    let ready = unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool();
    READY.set(ready);
    if ready {
        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        unsafe {
            let _ = Shell_NotifyIconW(NIM_SETVERSION, &data);
        }
    }
    ready
}

unsafe extern "system" fn subclass(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if message != 0 && message == ACTIVATE_INSTANCE.get() {
        unsafe {
            let _ = PostMessageW(Some(hwnd), SHOW, WPARAM(0), LPARAM(0));
        }
        return LRESULT(0);
    }
    if message == TASKBAR_CREATED.get() && message != 0 {
        // Explorer restarts invalidate every icon. If re-registration fails,
        // restore the window so a hidden app is still reachable.
        if !unsafe { add_icon(hwnd) } {
            unsafe {
                let _ = PostMessageW(Some(hwnd), SHOW, WPARAM(0), LPARAM(0));
            }
        }
        return LRESULT(0);
    }
    match message {
        WM_SYSCOMMAND
            if wp.0 as u32 & 0xfff0 == SC_MINIMIZE
                && READY.get()
                && PREFERENCES.get().minimize_to_tray =>
        {
            unsafe {
                let _ = PostMessageW(Some(hwnd), HIDE, WPARAM(0), LPARAM(0));
            }
            return LRESULT(0);
        }
        WM_SIZE
            if wp.0 as u32 == SIZE_MINIMIZED
                && READY.get()
                && PREFERENCES.get().minimize_to_tray =>
        {
            // Makepad's custom titlebar minimizes through ShowWindow rather
            // than WM_SYSCOMMAND; retain its geometry update below.
            unsafe {
                let _ = PostMessageW(Some(hwnd), HIDE, WPARAM(0), LPARAM(0));
            }
        }
        HIDE if READY.get() => {
            // Posted messages avoid reentering Makepad while its Cx is borrowed.
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            return LRESULT(0);
        }
        SHOW => {
            unsafe {
                let _ = ShowWindow(
                    hwnd,
                    if IsIconic(hwnd).as_bool() {
                        SW_RESTORE
                    } else {
                        SW_SHOW
                    },
                );
                let _ = SetForegroundWindow(hwnd);
            }
            action(TrayAction::Show);
            return LRESULT(0);
        }
        CALLBACK => {
            match lp.0 as u32 & 0xffff {
                NIN_SELECT | NIN_KEYSELECT | WM_LBUTTONUP | WM_LBUTTONDBLCLK => unsafe {
                    let _ = PostMessageW(Some(hwnd), SHOW, WPARAM(0), LPARAM(0));
                },
                WM_CONTEXTMENU | WM_RBUTTONUP => unsafe {
                    show_menu(hwnd);
                },
                _ => {}
            }
            return LRESULT(0);
        }
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, message, wp, lp) }
}

unsafe fn show_menu(hwnd: HWND) {
    unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        for (id, label) in [
            (1, crate::i18n::tr("Show main window")),
            (2, crate::i18n::tr("Settings")),
            (3, crate::i18n::tr("Quit Hagency")),
        ] {
            let label = wide(label);
            let _ = AppendMenuW(menu, MF_STRING, id, PCWSTR(label.as_ptr()));
        }
        let mut point = POINT::default();
        let _ = GetCursorPos(&mut point);
        let _ = SetForegroundWindow(hwnd);
        let selected = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            hwnd,
            None,
        )
        .0;
        let _ = DestroyMenu(menu);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        match selected {
            1 => {
                let _ = PostMessageW(Some(hwnd), SHOW, WPARAM(0), LPARAM(0));
            }
            2 => {
                let _ = ShowWindow(
                    hwnd,
                    if IsIconic(hwnd).as_bool() {
                        SW_RESTORE
                    } else {
                        SW_SHOW
                    },
                );
                let _ = SetForegroundWindow(hwnd);
                action(TrayAction::Settings);
            }
            3 => action(TrayAction::Quit),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_registration_round_trip_uses_an_isolated_non_startup_key() {
        let key = wide(&format!(
            "Software\\Hagency-Tray-Test-{}",
            uuid::Uuid::new_v4()
        ));
        struct Cleanup(Vec<u16>);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                unsafe {
                    let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(self.0.as_ptr()));
                }
            }
        }
        let cleanup = Cleanup(key);
        let key = PCWSTR(cleanup.0.as_ptr());
        assert!(!autostart_enabled_at(key).unwrap());
        set_autostart_at(key, true).unwrap();
        assert!(autostart_enabled_at(key).unwrap());
        let command = startup_command().unwrap();
        assert_eq!(command.first(), Some(&(b'"' as u16)));
        assert_eq!(&command[command.len() - 2..], &[b'"' as u16, 0]);
        set_autostart_at(key, false).unwrap();
        set_autostart_at(key, false).unwrap();
        assert!(!autostart_enabled_at(key).unwrap());
    }

    #[test]
    fn native_window_defers_hiding_and_obeys_minimize_preferences() {
        unsafe {
            // A private, off-screen tool window exercises real Win32 dispatch.
            // No Explorer process is required on CI; actual tray registration
            // is checked separately in an interactive application smoke test.
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("Hagency tray test"),
                WS_OVERLAPPEDWINDOW,
                -32000,
                -32000,
                64,
                64,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            struct Cleanup(HWND);
            impl Drop for Cleanup {
                fn drop(&mut self) {
                    unsafe {
                        let _ = DestroyWindow(self.0);
                    }
                }
            }
            let _cleanup = Cleanup(hwnd);
            let icon = CreateIconFromResourceEx(
                include_bytes!("../../resources/icon_32.png"),
                true,
                0x00030000,
                32,
                32,
                LR_DEFAULTCOLOR,
            )
            .unwrap();
            ICON.set(icon);
            PREFERENCES.set(DesktopPreferences::default());
            READY.set(true);
            assert!(SetWindowSubclass(hwnd, Some(subclass), SUBCLASS_ID, 0).as_bool());
            let tray = Tray { hwnd, icon };
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SendMessageW(
                hwnd,
                WM_SYSCOMMAND,
                Some(WPARAM(SC_MINIMIZE as usize)),
                Some(LPARAM(0)),
            );
            assert!(
                IsWindowVisible(hwnd).as_bool(),
                "hiding must be deferred until the current callback returns"
            );
            pump(hwnd);
            assert!(!IsWindowVisible(hwnd).as_bool());
            let _ = SendMessageW(hwnd, SHOW, None, None);
            assert!(IsWindowVisible(hwnd).as_bool());
            assert!(matches!(take_action(), Some(TrayAction::Show)));
            tray.update(DesktopPreferences {
                minimize_to_tray: false,
                ..Default::default()
            });
            let _ = SendMessageW(
                hwnd,
                WM_SYSCOMMAND,
                Some(WPARAM(SC_MINIMIZE as usize)),
                Some(LPARAM(0)),
            );
            pump(hwnd);
            assert!(
                IsIconic(hwnd).as_bool(),
                "disabled preference retains ordinary taskbar minimization"
            );
            assert!(IsWindowVisible(hwnd).as_bool());
            drop(tray);
            assert!(!READY.get());
        }
    }

    unsafe fn pump(hwnd: HWND) {
        let mut message = MSG::default();
        unsafe {
            while PeekMessageW(&mut message, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
                DispatchMessageW(&message);
            }
        }
    }
}
