//! Machine-local shell preferences, independent of Matrix accounts and hosted apps.
use makepad_widgets::*;
use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
mod windows;

/// Held by the standalone executable for its lifetime. Hosted apps never claim it.
#[cfg(target_os = "windows")]
pub struct InstanceGuard {
    _native: windows::InstanceGuard,
}

#[cfg(target_os = "windows")]
pub fn acquire_instance() -> anyhow::Result<Option<InstanceGuard>> {
    windows::acquire_instance().map(|guard| guard.map(|native| InstanceGuard { _native: native }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopPreferences {
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub start_hidden: bool,
}

impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            minimize_to_tray: true,
            close_to_tray: true,
            start_hidden: false,
        }
    }
}

#[derive(Default)]
pub struct DesktopSettings {
    pub enabled: bool,
    pub preferences: DesktopPreferences,
}

pub fn save_preferences(cx: &mut Cx, preferences: DesktopPreferences) -> anyhow::Result<()> {
    let dir = crate::app_data_dir();
    std::fs::create_dir_all(dir)?;
    let pending = dir.join("desktop-settings.json.pending");
    use std::io::Write;
    let mut file = std::fs::File::create(&pending)?;
    file.write_all(&serde_json::to_vec_pretty(&preferences)?)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(pending, dir.join("desktop-settings.json"))?;
    cx.global::<DesktopSettings>().preferences = preferences;
    Ok(())
}

pub fn autostart_enabled() -> anyhow::Result<bool> {
    #[cfg(target_os = "windows")]
    return windows::autostart_enabled();
    #[cfg(not(target_os = "windows"))]
    Ok(false)
}

pub fn set_autostart(enabled: bool) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    return windows::set_autostart(enabled);
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        anyhow::bail!("Desktop startup integration is unavailable on this platform")
    }
}

#[derive(Default)]
pub struct DesktopIntegration {
    initialized: bool,
    #[cfg(target_os = "windows")]
    tray: Option<windows::Tray>,
    #[cfg(target_os = "windows")]
    failed: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum TrayAction {
    Show,
    Settings,
    Quit,
}

#[derive(Clone, Debug)]
pub struct OpenDesktopSettings;

impl DesktopIntegration {
    pub fn handle_event(
        &mut self,
        cx: &mut Cx,
        event: &Event,
        window: Option<WindowId>,
    ) -> Option<TrayAction> {
        if !self.initialized {
            self.initialized = true;
            let preferences = if cfg!(test) {
                DesktopPreferences::default()
            } else {
                match std::fs::read(crate::app_data_dir().join("desktop-settings.json")) {
                    Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                        warning!("Could not read desktop preferences: {error}");
                        DesktopPreferences::default()
                    }),
                    Err(error) => {
                        if error.kind() != std::io::ErrorKind::NotFound {
                            warning!("Could not load desktop preferences: {error}");
                        }
                        DesktopPreferences::default()
                    }
                }
            };
            *cx.global::<DesktopSettings>() = DesktopSettings {
                enabled: cfg!(target_os = "windows"),
                preferences,
            };
        }
        #[cfg(target_os = "windows")]
        {
            if matches!(event, Event::Shutdown) {
                self.tray.take();
                return None;
            }
            if self.tray.is_none()
                && !self.failed
                && !cfg!(test)
                && let Some(window) = window
            {
                match windows::Tray::new(window, cx.global::<DesktopSettings>().preferences) {
                    Ok(tray) => self.tray = tray,
                    Err(error) => {
                        self.failed = true;
                        error!("Could not create system tray: {error}");
                        crate::shared::popup_list::enqueue_popup_notification(
                            crate::i18n::tr(
                                "The system tray is unavailable. Hagency will keep its normal window controls.",
                            ),
                            crate::shared::popup_list::PopupKind::Error,
                            Some(6.0),
                        );
                    }
                }
            }
            if let Some(tray) = &self.tray {
                tray.update(cx.global::<DesktopSettings>().preferences);
            }
            windows::take_action()
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (event, window);
            None
        }
    }

    /// Only hide when a live icon offers a way back to the application.
    pub fn hide_on_close(&self, cx: &Cx) -> bool {
        #[cfg(target_os = "windows")]
        if let Some(tray) = &self.tray {
            return cx
                .get_global_ref::<DesktopSettings>()
                .is_some_and(|s| s.preferences.close_to_tray)
                && tray.hide();
        }
        let _ = cx;
        false
    }

    /// Makepad's custom caption closes with a platform operation, bypassing
    /// WindowCloseRequested. Keep that path consistent with Alt+F4 / WM_CLOSE.
    pub fn intercept_caption_close(
        &self,
        cx: &mut Cx,
        event: &Event,
        button: DesktopButtonRef,
    ) -> bool {
        #[cfg(target_os = "windows")]
        if self.tray.as_ref().is_some_and(windows::Tray::is_ready)
            && cx
                .get_global_ref::<DesktopSettings>()
                .is_some_and(|s| s.preferences.close_to_tray)
            && matches!(event, Event::MouseUp(_) | Event::TouchUpdate(_))
            && let Hit::FingerUp(up) = event.hits(cx, button.area())
        {
            // Consume the release before DesktopButton emits Clicked. Window's
            // default action would destroy the HWND without a close request.
            // Other widget actions and async responses remain untouched.
            if let Some(mut button) = button.borrow_mut() {
                button.animator_play(cx, ids!(hover.off));
            }
            return up.is_over && self.hide_on_close(cx);
        }
        let _ = (cx, event, button);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_and_partial_settings_keep_recovery_defaults() {
        let defaults: DesktopPreferences = serde_json::from_str("{}").unwrap();
        assert!(defaults.close_to_tray && defaults.minimize_to_tray);
        assert!(!defaults.start_hidden);
        let partial: DesktopPreferences =
            serde_json::from_str(r#"{"close_to_tray":false}"#).unwrap();
        assert!(!partial.close_to_tray);
        assert!(partial.minimize_to_tray);
        assert!(serde_json::from_str::<DesktopPreferences>(r#"{"close_to_tray":"yes"}"#).is_err());
    }
}
