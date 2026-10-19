use makepad_widgets::*;
use crate::speech::{self, Provider, SpeechConfig};
use std::sync::mpsc::{self, Receiver};
type ServiceResult = Result<&'static str, String>;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    let SpeechSection = RinxCard {
        width: Fill height: Fit flow: Down padding: 16 spacing: 10
    }
    let SpeechField = RinxInput {width: Fill height: RINX_CONTROL_HEIGHT}
    let SpeechSelect = RinxDropDown {
        width: Fill height: RINX_CONTROL_HEIGHT
        popup_menu_position: PopupMenuPosition.BelowInput
    }
    let FieldLabel = RinxLabel {
        draw_text.text_style: theme.font_bold {font_size: mod.widgets.RINX_UI_SIZE}
    }

    mod.widgets.SpeechSettings = #(SpeechSettings::register_widget(vm)) {
        width: Fill height: Fit flow: Down spacing: 16
        SpeechSection {
            FieldLabel {text: #(crate::i18n::tr("Speech recognition")) i18n_text: "Speech recognition"}
            RinxHint {width: Fill flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("The microphone inserts a transcript into your draft. It never sends the message automatically."))
                i18n_text: "The microphone inserts a transcript into your draft. It never sends the message automatically."
            }
            provider := SpeechSelect {
                labels: ["System speech", "Local Whisper server", "OpenAI-compatible speech API", "Local Codex (experimental)"]
                i18n_labels: ["System speech", "Local Whisper server", "OpenAI-compatible speech API", "Local Codex (experimental)"]
            }
            provider_note := RinxHint {width: Fill flow: Flow.Right{wrap: true}}
        }
        SpeechSection {
            FieldLabel {text: #(crate::i18n::tr("Recognition language")) i18n_text: "Recognition language"}
            language := SpeechSelect {
                labels: ["System default / automatic", "Simplified Chinese", "English (US)", "Other language"]
                i18n_labels: ["System default / automatic", "Simplified Chinese", "English (US)", "Other language"]
            }
            locale_row := View {width: Fill height: Fit flow: Down spacing: 6 visible: false
                locale := SpeechField {empty_text: "ja-JP, fr-FR, de-DE…"}
            }
            RinxHint {width: Fill flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("For Chinese dictation, select Chinese explicitly. The speech language is independent of the app language."))
                i18n_text: "For Chinese dictation, select Chinese explicitly. The speech language is independent of the app language."
            }
            native_options := View {width: Fill height: Fit flow: Down
                on_device := RinxCheckBox {width: Fill
                    text: #(crate::i18n::tr("Prefer on-device recognition when supported")) i18n_text: "Prefer on-device recognition when supported"
                }
            }
        }
        service_options := SpeechSection {
            FieldLabel {text: #(crate::i18n::tr("Speech service")) i18n_text: "Speech service"}
            endpoint_row := View {width: Fill height: Fit flow: Down
                endpoint := SpeechField {empty_text: "http://127.0.0.1:8080/inference"}
            }
            codex_options := RinxHint {visible: false width: Fill flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("Shares the local Codex account with Agents → Model & resources on this device. Connecting here selects that same account for Agents. Audio is processed by Codex's online voice service; availability depends on your Codex version and account."))
                i18n_text: "Shares the local Codex account with Agents → Model & resources on this device. Connecting here selects that same account for Agents. Audio is processed by Codex's online voice service; availability depends on your Codex version and account."
            }
            connect_codex := RinxButton {visible: false text: #(crate::i18n::tr("Connect local Codex")) i18n_text: "Connect local Codex"}
            model_row := View {width: Fill height: Fit flow: Down spacing: 6
                FieldLabel {text: #(crate::i18n::tr("Transcription model")) i18n_text: "Transcription model"}
                model := SpeechField {empty_text: "whisper-1 / gpt-transcribe"}
                FieldLabel {text: #(crate::i18n::tr("API key environment variable")) i18n_text: "API key environment variable"}
                key_env := SpeechField {empty_text: "OPENAI_API_KEY"}
                RinxHint {width: Fill flow: Flow.Right{wrap: true}
                    text: #(crate::i18n::tr("Only the variable name is saved. Set the secret in your environment before starting Hagency; leave the name empty for a server without authentication."))
                    i18n_text: "Only the variable name is saved. Set the secret in your environment before starting Hagency; leave the name empty for a server without authentication."
                }
            }
            FieldLabel {text: #(crate::i18n::tr("Recognition context (optional)")) i18n_text: "Recognition context (optional)"}
            prompt := SpeechField {
                empty_text: #(crate::i18n::tr("Names or terminology you expect to dictate")) i18n_empty_text: "Names or terminology you expect to dictate"
            }
            recording_note := RinxHint {width: Fill flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("Recordings stay in memory. When you stop, a WAV recording is sent only to the selected endpoint. Escape cancels; there is no automatic cloud fallback."))
                i18n_text: "Recordings stay in memory. When you stop, a WAV recording is sent only to the selected endpoint. Escape cancels; there is no automatic cloud fallback."
            }
        }
        SpeechSection {
            FieldLabel {text: #(crate::i18n::tr("Recording limit")) i18n_text: "Recording limit"}
            limit := SpeechSelect {labels: ["30 seconds", "60 seconds", "120 seconds"] i18n_labels: ["30 seconds", "60 seconds", "120 seconds"] selected_item: 1}
            RinxHint {width: Fill flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("Service recordings stop at this limit. Codex and native dictation update your draft while you speak."))
                i18n_text: "Service recordings stop at this limit. Codex and native dictation update your draft while you speak."
            }
            View {width: Fill height: Fit flow: Flow.Right{wrap: true} spacing: 8
                save := RinxPrimaryButton {text: #(crate::i18n::tr("Save speech settings")) i18n_text: "Save speech settings"}
                test := RinxButton {text: #(crate::i18n::tr("Test service")) i18n_text: "Test service"}
                cancel_test := RinxGhostButton {visible: false text: #(crate::i18n::tr("Cancel")) i18n_text: "Cancel"}
            }
            RinxHint {width: Fill flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("The service test sends one second of generated silence, without opening the microphone. Native mode checks platform availability only."))
                i18n_text: "The service test sends one second of generated silence, without opening the microphone. Native mode checks platform availability only."
            }
            status := RinxLabel {width: Fill flow: Flow.Right{wrap: true}}
        }
        SpeechSection {
            FieldLabel {text: #(crate::i18n::tr("Platform and assistant support")) i18n_text: "Platform and assistant support"}
            platform := RinxHint {width: Fill flow: Flow.Right{wrap: true}}
            RinxHint {width: Fill flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("Microphone: the system default input device. Change it in your operating system audio settings."))
                i18n_text: "Microphone: the system default input device. Change it in your operating system audio settings."
            }
            RinxHint {width: Fill flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("Windows uses local SAPI; Apple uses Speech; Android uses its installed recognition service. Language packs, permissions and offline support depend on the system. Linux uses a configured speech service."))
                i18n_text: "Windows uses local SAPI; Apple uses Speech; Android uses its installed recognition service. Language packs, permissions and offline support depend on the system. Linux uses a configured speech service."
            }
            RinxHint {width: Fill flow: Flow.Right{wrap: true}
                text: #(crate::i18n::tr("Local Codex speech uses its own experimental realtime connection. Claude Code can process text after transcription. Runtime sign-in is not an API key for a speech endpoint."))
                i18n_text: "Local Codex speech uses its own experimental realtime connection. Claude Code can process text after transcription. Runtime sign-in is not an API key for a speech endpoint."
            }
        }
    }
}

#[derive(Script, Widget)]
pub struct SpeechSettings {
    #[deref] view: View,
    #[rust] config: SpeechConfig,
    #[rust] loaded: bool,
    #[rust] receiver: Option<Receiver<ServiceResult>>,
    #[rust] task: Option<tokio::task::JoinHandle<()>>,
    #[rust] test_epoch: u64,
}

impl ScriptHook for SpeechSettings {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            let config = cx.global::<super::app_preferences::AppPreferencesGlobal>().0.speech.clone();
            self.populate(cx, &config);
        });
    }
    fn on_after_apply(&mut self, vm: &mut ScriptVm, apply: &Apply, _: &mut Scope, _: ScriptValue) {
        if self.loaded && apply.is_script_reapply() {vm.with_cx_mut(|cx| self.refresh(cx));}
    }
}

impl SpeechSettings {
    pub fn populate(&mut self, cx: &mut Cx, config: &SpeechConfig) {
        self.cancel_test(cx);
        self.config = config.clone();
        self.loaded = true;
        for (id, value) in [(id!(locale), &config.locale), (id!(endpoint), &config.endpoint().to_string()),
            (id!(model), &config.model), (id!(key_env), &config.api_key_env), (id!(prompt), &config.prompt)] {
            self.view.text_input(cx, &[id]).set_text(cx, value);
        }
        self.view.label(cx, ids!(status)).set_text(cx, "");
        self.refresh(cx);
    }

    fn refresh(&mut self, cx: &mut Cx) {
        self.refresh_with_capabilities(cx, speech::Capabilities::current());
    }

    fn refresh_with_capabilities(&mut self, cx: &mut Cx, capabilities: speech::Capabilities) {
        let system = self.config.provider == Provider::System;
        let compatible = self.config.provider == Provider::OpenAiCompatible;
        let codex = self.config.provider == Provider::LocalCodex;
        self.view.view(cx, ids!(service_options)).set_visible(cx, !system);
        self.view.view(cx, ids!(endpoint_row)).set_visible(cx, !codex);
        self.view.widget(cx, ids!(codex_options)).set_visible(cx, codex);
        self.view.button(cx, ids!(connect_codex)).set_visible(cx, codex);
        self.view.view(cx, ids!(model_row)).set_visible(cx, compatible);
        self.view.view(cx, ids!(native_options)).set_visible(cx, system);
        let index = match self.config.provider {Provider::System => 0, Provider::WhisperCpp => 1, Provider::OpenAiCompatible => 2, Provider::LocalCodex => 3};
        self.view.drop_down(cx, ids!(provider)).set_selected_item(cx, index);
        let language = match self.config.locale.as_str() {"" => 0, "zh-CN" => 1, "en-US" => 2, _ => 3};
        self.view.drop_down(cx, ids!(language)).set_selected_item(cx, language);
        self.view.view(cx, ids!(locale_row)).set_visible(cx, language == 3);
        self.view.check_box(cx, ids!(on_device)).set_active(cx, self.config.prefer_on_device, Animate::No);
        self.view.drop_down(cx, ids!(limit)).set_selected_item(cx, match self.config.max_seconds {0..=30 => 0,31..=60 => 1,_ => 2});
        let note = capabilities.unavailable_message(self.config.provider).unwrap_or(match self.config.provider {
            Provider::System => speech::native_description(),
            Provider::WhisperCpp => "Use a running whisper.cpp server on this device, for example /inference. Its loaded model is selected on the server.",
            Provider::OpenAiCompatible => "Use the full /audio/transcriptions endpoint. OpenAI, compatible third-party providers and local compatible servers are supported.",
            Provider::LocalCodex => "Experimental Codex dictation. Only your speech transcript is inserted into the draft; messages are never sent automatically.",
        });
        let available = capabilities.supports(self.config.provider) && self.task.is_none();
        self.view.button(cx, ids!(test)).set_enabled(cx, available);
        self.view.button(cx, ids!(connect_codex)).set_enabled(cx, codex && available);
        self.view.label(cx, ids!(recording_note)).set_text(cx, crate::i18n::tr(if codex {
            "Local Codex connects before recording, then streams audio and updates your draft as you speak. Wait for the listening indicator before speaking. Stopping only finalizes the last words; Escape cancels."
        } else {"Recordings stay in memory. When you stop, a WAV recording is sent only to the selected endpoint. Escape cancels; there is no automatic cloud fallback."}));
        self.view.label(cx, ids!(provider_note)).set_text(cx, crate::i18n::tr(note));
        self.view.label(cx, ids!(platform)).set_text(cx, crate::i18n::tr(speech::native_description_with_capabilities(capabilities)));
        self.view.redraw(cx);
    }

    fn read_fields(&mut self, cx: &mut Cx) {
        if self.view.view(cx, ids!(locale_row)).visible() {self.config.locale = self.view.text_input(cx, ids!(locale)).text().trim().to_owned();}
        let endpoint = self.view.text_input(cx, ids!(endpoint)).text().trim().to_owned();
        if self.config.provider == Provider::WhisperCpp {self.config.whisper_endpoint = endpoint;}
        else if self.config.provider == Provider::OpenAiCompatible {self.config.api_endpoint = endpoint;}
        self.config.model = self.view.text_input(cx, ids!(model)).text().trim().to_owned();
        self.config.api_key_env = self.view.text_input(cx, ids!(key_env)).text().trim().to_owned();
        self.config.prompt = self.view.text_input(cx, ids!(prompt)).text().trim().to_owned();
    }

    fn cancel_test(&mut self, cx: &mut Cx) {
        if let Some(task) = self.task.take() {task.abort();}
        self.receiver = None;
        let available = speech::supported(&self.config);
        self.view.button(cx, ids!(test)).set_enabled(cx, available);
        self.view.button(cx, ids!(connect_codex)).set_enabled(cx, available && self.config.provider == Provider::LocalCodex);
        self.view.button(cx, ids!(cancel_test)).set_visible(cx, false);
    }

    fn test(&mut self, cx: &mut Cx) -> Result<(), robius_speech::SpeechError> {
        self.test_with_capabilities(cx, speech::Capabilities::current())
    }

    fn test_with_capabilities(&mut self, cx: &mut Cx, capabilities: speech::Capabilities) -> Result<(), robius_speech::SpeechError> {
        capabilities.ensure(self.config.provider)?;
        self.config.validate()?;
        if self.config.provider == Provider::System {
            self.view.label(cx, ids!(status)).set_text(cx, crate::i18n::tr(
                "Native recognition is available. The selected language and microphone are checked when you start dictation."
            ));
            return Ok(());
        }
        let credential = self.config.credential()?;
        self.cancel_test(cx);
        let config = self.config.clone();
        let (sender, receiver) = mpsc::channel();
        let signal = thread::SignalToUI::new();
        self.test_epoch = crate::account_session::epoch();
        self.receiver = Some(receiver);
        self.view.button(cx, ids!(test)).set_enabled(cx, false);
        self.view.button(cx, ids!(connect_codex)).set_enabled(cx, false);
        self.view.button(cx, ids!(cancel_test)).set_visible(cx, true);
        self.view.label(cx, ids!(status)).set_text(cx, crate::i18n::tr("Testing the speech endpoint…"));
        self.task = Some(crate::account_session::spawn(async move {
            let message = if config.provider == Provider::LocalCodex {
                "Codex accepted test audio without a voice error. Test dictation to verify recognition accuracy."
            } else {"The endpoint accepted WAV audio and returned a valid transcript response."};
            let result = speech::test_service(config, credential).await.map(|()| message).map_err(|error| error.to_string());
            let _ = sender.send(result);
            signal.set();
        }));
        Ok(())
    }

    fn connect_codex(&mut self, cx: &mut Cx) {
        self.connect_codex_with_capabilities(cx, speech::Capabilities::current());
    }

    fn connect_codex_with_capabilities(&mut self, cx: &mut Cx, capabilities: speech::Capabilities) {
        if self.config.provider != Provider::LocalCodex {return;}
        if let Err(error) = capabilities.ensure(self.config.provider) {
            self.cancel_test(cx);
            self.refresh_with_capabilities(cx, capabilities);
            self.view.label(cx, ids!(status)).set_text(cx, error.message());
            return;
        }
        self.cancel_test(cx);
        let (sender, receiver) = mpsc::channel();
        let signal = thread::SignalToUI::new();
        self.test_epoch = crate::account_session::epoch();
        self.receiver = Some(receiver);
        self.view.button(cx, ids!(test)).set_enabled(cx, false);
        self.view.button(cx, ids!(connect_codex)).set_enabled(cx, false);
        self.view.button(cx, ids!(cancel_test)).set_visible(cx, true);
        self.view.label(cx, ids!(status)).set_text(cx, crate::i18n::tr("Checking the local Codex account…"));
        self.task = Some(crate::account_session::spawn(async move {
            let result = async {
                let service = crate::hagency::backend::Service::for_current().map_err(|_| speech::error("Sign in to Hagency, then connect your local Codex in Agents → Model & resources.").to_string())?;
                let value = service.execute(crate::hagency::model::Command::Provider {
                    action: crate::hagency::model::ProviderAction::UseLocal,
                }).await.map_err(|code| speech::codex_error(&code).to_string())?;
                if value["authenticated"] != true || value["shared"] != true {return Err(speech::codex_error("local_codex_account_not_signed_in").to_string());}
                Ok("Local Codex connected. Agent services on this device share this account. You can now test speech.")
            }.await;
            let _ = sender.send(result);
            signal.set();
        }));
    }
}

impl Widget for SpeechSettings {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // A dropdown exposes children only while its popup is open. Route
        // pointer input to that popup exclusively: CheckBox reacts on down,
        // before the dropdown emits its selection on up, so filtering the
        // resulting Actions would be too late to prevent a click underneath.
        let popup = [id!(provider), id!(language), id!(limit)].into_iter().find_map(|id| {
            let widget = self.view.widget(cx, &[id]);
            let mut open = false;
            widget.children(&mut |_, _| open = true);
            open.then_some(widget)
        });
        if let Some(popup) = popup.filter(|_| matches!(event,
            Event::MouseDown(_) | Event::MouseUp(_) | Event::MouseMove(_) | Event::TouchUpdate(_))) {
            popup.handle_event(cx, event, scope);
        } else {
            self.view.handle_event(cx, event, scope);
        }
        let result = self.receiver.as_ref().and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = result {
            let current_account = self.test_epoch == crate::account_session::epoch();
            self.cancel_test(cx);
            if current_account {
                self.view.label(cx, ids!(status)).set_text(cx, &match result {
                    Ok(message) => crate::i18n::tr(message).to_owned(),
                    Err(error) => error,
                });
            }
        }
        if let Event::Actions(actions) = event {
            let settings_changed = [id!(locale),id!(endpoint),id!(model),id!(key_env),id!(prompt)].into_iter()
                .any(|id| self.view.text_input(cx, &[id]).changed(actions).is_some())
                || [id!(provider),id!(language),id!(limit)].into_iter()
                    .any(|id| self.view.drop_down(cx, &[id]).changed(actions).is_some())
                || self.view.check_box(cx, ids!(on_device)).changed(actions).is_some();
            if let Some(index) = self.view.drop_down(cx, ids!(provider)).changed(actions) {
                self.read_fields(cx);
                self.config.provider = match index {1 => Provider::WhisperCpp,2 => Provider::OpenAiCompatible,3 => Provider::LocalCodex,_ => Provider::System};
                self.cancel_test(cx);
                self.view.text_input(cx, ids!(endpoint)).set_text(cx, self.config.endpoint());
                self.view.label(cx, ids!(status)).set_text(cx, "");
                self.refresh(cx);
            }
            if let Some(index) = self.view.drop_down(cx, ids!(language)).changed(actions) {
                self.config.locale = match index {0 => "",1 => "zh-CN",2 => "en-US",_ => "ja-JP"}.to_owned();
                self.view.text_input(cx, ids!(locale)).set_text(cx, &self.config.locale);
                self.refresh(cx);
            }
            if let Some(value) = self.view.check_box(cx, ids!(on_device)).changed(actions) {self.config.prefer_on_device = value;}
            if let Some(index) = self.view.drop_down(cx, ids!(limit)).changed(actions) {self.config.max_seconds = [30,60,120][index.min(2)];}
            if settings_changed {
                self.cancel_test(cx);
                self.view.label(cx, ids!(status)).set_text(cx, crate::i18n::tr("Configuration changed. Save or test these settings."));
            }
            if self.view.button(cx, ids!(cancel_test)).clicked(actions) {
                self.cancel_test(cx);
                self.view.label(cx, ids!(status)).set_text(cx, crate::i18n::tr("Service test cancelled."));
            }
            if self.view.button(cx, ids!(save)).clicked(actions) {
                self.read_fields(cx);
                match self.config.validate() {
                    Ok(()) => {
                        cx.action(speech::SettingsSaved(self.config.clone()));
                        self.view.label(cx, ids!(status)).set_text(cx, crate::i18n::tr("Speech settings saved."));
                    }
                    Err(error) => self.view.label(cx, ids!(status)).set_text(cx, &error.to_string()),
                }
            }
            if self.view.button(cx, ids!(test)).clicked(actions) {
                self.read_fields(cx);
                if let Err(error) = self.test(cx) {self.view.label(cx, ids!(status)).set_text(cx, &error.to_string());}
            }
            if self.view.button(cx, ids!(connect_codex)).clicked(actions) {self.connect_codex(cx);}
        }
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {self.view.draw_walk(cx, scope, walk)}
}

impl SpeechSettingsRef {
    pub fn populate(&self, cx: &mut Cx, config: &SpeechConfig) {
        if let Some(mut inner) = self.borrow_mut() {inner.populate(cx, config);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use makepad_widgets::makepad_draw::cx_draw::CxDraw;

    fn panel(cx: &mut Cx) -> SpeechSettings {
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm); crate::app::register_widgets(vm);
            vm.bx.captured_errors = Some(Vec::new());
            let value = script_eval!(vm, {mod.widgets.SpeechSettings {}});
            let panel = SpeechSettings::script_from_value(vm, value);
            let errors = vm.take_errors(); assert!(errors.is_empty(), "{errors:?}"); panel
        })
    }

    #[test]
    fn unsupported_providers_show_one_reason_and_never_start_work() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut panel = panel(&mut cx);
        for (os, env, native) in [("linux", "ohos", false), ("unknown", "", false), ("android", "", true), ("ios", "", true), ("linux", "gnu", false)] {
            let capabilities = speech::Capabilities::for_target(os, env, native);
            for provider in [Provider::System, Provider::WhisperCpp, Provider::OpenAiCompatible, Provider::LocalCodex] {
                if capabilities.supports(provider) {continue;}
                let mut config = SpeechConfig::default();
                config.provider = provider;
                config.api_endpoint = "invalid URL must not be used".into();
                config.api_key_env = "HAGENCY_UNSUPPORTED_SPEECH_MUST_NOT_READ_KEY".into();
                panel.populate(&mut cx, &config);
                panel.refresh_with_capabilities(&mut cx, capabilities);
                let message = crate::i18n::tr(capabilities.unavailable_message(provider).unwrap());
                assert_eq!(panel.view.label(&mut cx, ids!(provider_note)).text(), message);
                assert!(!panel.view.button(&mut cx, ids!(test)).borrow().unwrap().enabled());
                assert!(!panel.view.button(&mut cx, ids!(connect_codex)).borrow().unwrap().enabled());
                let error = panel.test_with_capabilities(&mut cx, capabilities).unwrap_err();
                assert_eq!(error.kind(), robius_speech::SpeechErrorKind::Unavailable);
                assert_eq!(error.message(), message);
                if provider == Provider::LocalCodex {
                    panel.connect_codex_with_capabilities(&mut cx, capabilities);
                    assert_eq!(panel.view.label(&mut cx, ids!(status)).text(), message);
                }
                assert!(panel.task.is_none());
                assert!(panel.receiver.is_none());
                assert_eq!(panel.config, config, "capability checks must preserve saved configuration");
            }
        }
        let mut config = SpeechConfig::default(); config.provider = Provider::WhisperCpp;
        panel.populate(&mut cx, &config);
        panel.refresh_with_capabilities(&mut cx, speech::Capabilities::for_target("linux", "gnu", false));
        assert!(panel.view.button(&mut cx, ids!(test)).borrow().unwrap().enabled());
    }

    #[test]
    fn codex_selection_reuses_agent_configuration_without_http_credentials() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut panel = panel(&mut cx);
        let mut config = SpeechConfig::default();
        config.provider = Provider::LocalCodex;
        config.locale = "zh-CN".into();
        config.api_endpoint = "invalid HTTP endpoint is irrelevant".into();
        config.api_key_env = "MISSING_SPEECH_KEY".into();
        panel.populate(&mut cx, &config);
        assert_eq!(panel.view.drop_down(&mut cx, ids!(provider)).selected_item(), 3);
        assert!(!panel.view.view(&mut cx, ids!(endpoint_row)).visible());
        assert!(!panel.view.view(&mut cx, ids!(model_row)).visible());
        assert!(panel.view.button(&mut cx, ids!(connect_codex)).visible());
        assert!(config.validate().is_ok());
        assert_eq!(config.credential().unwrap(), None);
        let uid = panel.view.button(&mut cx, ids!(save)).widget_uid();
        let clicks = cx.capture_actions(|cx| cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default())));
        let saved = cx.capture_actions(|cx| panel.handle_event(cx, &Event::Actions(clicks), &mut Scope::empty()));
        let saved = saved.iter().find_map(|a| a.downcast_ref::<speech::SettingsSaved>()).unwrap();
        assert_eq!(saved.0.provider, Provider::LocalCodex);
        assert_eq!(saved.0.locale, "zh-CN");
        assert!(panel.task.is_none());
    }

    #[test]
    fn speech_form_saves_selected_provider_and_language_without_a_microphone() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut panel = panel(&mut cx);
        let mut config = SpeechConfig::default(); config.provider = Provider::WhisperCpp; config.locale = "zh-CN".into();
        panel.populate(&mut cx, &config);
        assert!(panel.view.view(&mut cx, ids!(service_options)).visible());
        assert!(!panel.view.view(&mut cx, ids!(model_row)).visible());
        panel.view.text_input(&mut cx, ids!(endpoint)).set_text(&mut cx, "http://127.0.0.1:9988/inference");
        let uid = panel.view.button(&mut cx, ids!(save)).widget_uid();
        let clicks = cx.capture_actions(|cx| cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default())));
        let saved = cx.capture_actions(|cx| panel.handle_event(cx, &Event::Actions(clicks), &mut Scope::empty()));
        let saved = saved.iter().find_map(|action| action.downcast_ref::<speech::SettingsSaved>()).unwrap();
        assert_eq!(saved.0.locale, "zh-CN");
        assert_eq!(saved.0.whisper_endpoint, "http://127.0.0.1:9988/inference");
        assert_eq!(saved.0.provider, Provider::WhisperCpp);
        assert!(panel.task.is_none());
    }

    #[test]
    fn language_popup_does_not_toggle_the_control_underneath() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut panel = panel(&mut cx);
        let pass = DrawPass::new(&mut cx); pass.set_size(&mut cx, dvec2(760.0, 1600.0));
        let mut list = DrawList2d::new(&mut cx);
        let overlay = Overlay {draw_list: DrawList::new(&mut cx)};
        let draw_panel = |panel: &mut SpeechSettings, cx: &mut Cx, list: &mut DrawList2d| {
            let event = DrawEvent {redraw_all: true, ..DrawEvent::default()};
            let mut draw = CxDraw::new(cx, &event); let mut cx2d = Cx2d::new(&mut draw);
            cx2d.begin_pass(&pass, None); list.begin_always(&mut cx2d);
            overlay.begin(&mut cx2d);
            cx2d.begin_root_turtle(dvec2(760.0, 1600.0), Layout::flow_overlay());
            panel.draw_walk_all(&mut cx2d, &mut Scope::empty(), Walk {width: Size::Fixed(760.0), height: Size::fit(), ..Walk::default()});
            cx2d.end_pass_sized_turtle(); overlay.end(&mut cx2d); list.end(&mut cx2d); cx2d.end_pass(&pass);
        };
        draw_panel(&mut panel, &mut cx, &mut list);
        let checkbox = panel.view.check_box(&mut cx, ids!(on_device));
        let rect = checkbox.area().rect(&cx);
        let point = rect.pos + dvec2(8.0, rect.size.y * 0.5);
        let down = || Event::MouseDown(MouseDownEvent {abs: point, button: MouseButton::PRIMARY,
            window_id: WindowId(0,0), modifiers: KeyModifiers::default(), handled: std::cell::Cell::new(Area::Empty), time: 1.0});
        let underlying = cx.capture_actions(|cx| panel.handle_event(cx, &down(), &mut Scope::empty()));
        assert_eq!(checkbox.changed(&underlying), Some(false), "the point must hit the underlying checkbox when the popup is closed");
        panel.populate(&mut cx, &SpeechConfig::default());
        panel.view.drop_down(&mut cx, ids!(language)).borrow_mut().unwrap().set_active(&mut cx);
        draw_panel(&mut panel, &mut cx, &mut list);
        let actions = cx.capture_actions(|cx| panel.handle_event(cx, &down(), &mut Scope::empty()));
        assert!(checkbox.changed(&actions).is_none(), "a popup click must not toggle the underlying preference");
        assert!(panel.config.prefer_on_device);
    }

    #[test]
    fn speech_controls_fit_phone_and_desktop_content_widths() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut panel = panel(&mut cx);
        let mut config = SpeechConfig::default(); config.provider = Provider::OpenAiCompatible;
        panel.populate(&mut cx, &config);
        for width in [287.0, 760.0] {
            cx.redraw_all();
            let pass = DrawPass::new(&mut cx); pass.set_size(&mut cx, dvec2(width, 1600.0));
            let mut list = DrawList2d::new(&mut cx);
            let event = DrawEvent {redraw_all: true, ..DrawEvent::default()};
            let mut draw = CxDraw::new(&mut cx, &event); let mut cx2d = Cx2d::new(&mut draw);
            cx2d.begin_pass(&pass, None); list.begin_always(&mut cx2d);
            cx2d.begin_root_turtle(dvec2(width, 1600.0), Layout::flow_overlay());
            panel.draw_walk_all(&mut cx2d, &mut Scope::empty(), Walk {width: Size::Fixed(width), height: Size::fit(), ..Walk::default()});
            cx2d.end_pass_sized_turtle(); list.end(&mut cx2d); cx2d.end_pass(&pass);
            drop(cx2d); drop(draw);
            for id in [id!(provider), id!(language), id!(endpoint), id!(model), id!(key_env), id!(limit), id!(save), id!(test)] {
                let rect = panel.view.widget(&cx, &[id]).area().rect(&cx);
                assert!(rect.size.x > 0.0 && rect.pos.x >= 0.0 && rect.pos.x + rect.size.x <= width + 0.1, "{id:?} width={width} {rect:?}");
            }
        }
    }
}
