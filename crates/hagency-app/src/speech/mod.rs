//! Account preferences and HTTP speech adapters. Audio never goes to a service
//! unless the user selected that provider and explicitly started dictation.
pub mod capture;

use serde::{Deserialize, Serialize};
use robius_speech::{SpeechError, SpeechErrorKind};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    #[default]
    System,
    WhisperCpp,
    OpenAiCompatible,
    LocalCodex,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeechConfig {
    pub provider: Provider,
    /// Empty means the OS default for native recognition, auto-detect for HTTP.
    pub locale: String,
    pub prefer_on_device: bool,
    pub whisper_endpoint: String,
    pub api_endpoint: String,
    pub model: String,
    /// Only a variable NAME is persisted, never its secret value.
    pub api_key_env: String,
    pub prompt: String,
    pub max_seconds: u32,
}

impl Default for SpeechConfig {
    fn default() -> Self {
        Self {
            provider: Provider::System,
            locale: String::new(),
            prefer_on_device: true,
            whisper_endpoint: "http://127.0.0.1:8080/inference".into(),
            api_endpoint: "https://api.openai.com/v1/audio/transcriptions".into(),
            model: "whisper-1".into(),
            api_key_env: "OPENAI_API_KEY".into(),
            prompt: String::new(),
            max_seconds: 60,
        }
    }
}

pub fn error(message: &'static str) -> SpeechError {
    SpeechError::new(SpeechErrorKind::Other, crate::i18n::tr(message))
}

pub fn native_description() -> &'static str {
    native_description_with_capabilities(Capabilities::current())
}

pub(crate) fn native_description_with_capabilities(capabilities: Capabilities) -> &'static str {
    if let Some(message) = capabilities.unavailable_message(Provider::System) {
        return message;
    }
    if cfg!(target_os = "windows") { "Windows SAPI · on-device dictation" }
    else if cfg!(any(target_os = "macos", target_os = "ios")) { "Apple Speech · availability depends on language and device" }
    else if cfg!(target_os = "android") { "Android SpeechRecognizer · availability depends on installed service" }
    else { SYSTEM_UNSUPPORTED }
}

pub fn supported(config: &SpeechConfig) -> bool {
    Capabilities::current().supports(config.provider)
}

const PLATFORM_UNSUPPORTED: &str = "Speech input is not supported on this platform. Your draft has been kept.";
const SYSTEM_UNSUPPORTED: &str = "System speech recognition is not supported on this platform. Choose another service in Settings → Speech.";
const CODEX_UNSUPPORTED: &str = "Local Codex speech is not supported on this platform. Choose another service in Settings → Speech.";
const RECORDING_UNSUPPORTED: &str = "Speech recording is not supported on this platform. Choose another service in Settings → Speech.";

#[derive(Clone, Copy)]
pub(crate) struct Capabilities {system: bool, recording: bool, codex: bool}

impl Capabilities {
    pub(crate) fn current() -> Self {
        Self::for_target(std::env::consts::OS, if cfg!(target_env = "ohos") {"ohos"} else {""}, robius_speech::NativeSpeechSession::is_supported())
    }
    pub(crate) fn for_target(os: &str, env: &str, native: bool) -> Self {
        Self {
            system: native && env != "ohos",
            // Makepad's OHOS audio_input/use_audio_inputs are currently no-ops.
            recording: env != "ohos" && matches!(os, "windows" | "macos" | "linux" | "android" | "ios"),
            codex: hagency::console::speech::local_codex_supported_on(os, env),
        }
    }
    pub(crate) fn supports(self, provider: Provider) -> bool {
        match provider {Provider::System => self.system, Provider::LocalCodex => self.codex, _ => self.recording}
    }
    pub(crate) fn unavailable_message(self, provider: Provider) -> Option<&'static str> {
        if self.supports(provider) {return None;}
        Some(if !self.system && !self.recording && !self.codex {PLATFORM_UNSUPPORTED} else {match provider {
            Provider::System => SYSTEM_UNSUPPORTED, Provider::LocalCodex => CODEX_UNSUPPORTED, _ => RECORDING_UNSUPPORTED,
        }})
    }
    pub(crate) fn ensure(self, provider: Provider) -> Result<(), SpeechError> {
        match self.unavailable_message(provider) {
            Some(message) => Err(SpeechError::new(SpeechErrorKind::Unavailable, crate::i18n::tr(message))),
            None => Ok(()),
        }
    }
}

pub(crate) fn ensure_supported(config: &SpeechConfig) -> Result<(), SpeechError> {Capabilities::current().ensure(config.provider)}

impl SpeechConfig {
    pub fn endpoint(&self) -> &str {
        if self.provider == Provider::WhisperCpp { &self.whisper_endpoint } else { &self.api_endpoint }
    }

    pub fn validate(&self) -> Result<(), SpeechError> {
        if self.locale.len() > 35 || !self.locale.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
            return Err(error("Use a language tag such as zh-CN or en-US, or leave it empty."));
        }
        if !(10..=120).contains(&self.max_seconds) || self.prompt.len() > 4000 {
            return Err(error("Use a recording limit of 10–120 seconds and a prompt under 4000 bytes."));
        }
        if matches!(self.provider, Provider::System | Provider::LocalCodex) { return Ok(()); }
        let url = matrix_sdk::reqwest::Url::parse(self.endpoint()).map_err(|_| error("Enter a complete speech endpoint URL."))?;
        let local = is_loopback(&url);
        if !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
            return Err(error("Keep credentials out of the URL. Use the API key environment variable."));
        }
        if url.scheme() != "https" && !(url.scheme() == "http" && local) {
            return Err(error("Use HTTPS for remote services. HTTP is allowed only on this device."));
        }
        if self.provider == Provider::WhisperCpp && !local {
            return Err(error("The local Whisper endpoint must use localhost, 127.0.0.1 or ::1."));
        }
        if self.provider == Provider::OpenAiCompatible {
            if self.model.trim().is_empty() || self.model.len() > 200 {
                return Err(error("Enter the transcription model served by this endpoint."));
            }
            if !self.api_key_env.is_empty() && (self.api_key_env.len() > 100 ||
                !self.api_key_env.bytes().enumerate().all(|(i,c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))) {
                return Err(error("Enter an environment variable name, not the API key itself."));
            }
        }
        Ok(())
    }

    pub fn credential(&self) -> Result<Option<String>, SpeechError> {
        if self.provider != Provider::OpenAiCompatible || self.api_key_env.is_empty() {return Ok(None);}
        std::env::var(&self.api_key_env).ok().filter(|value| !value.trim().is_empty())
            .map(Some).ok_or_else(|| error("The API key environment variable is missing. Set it before starting Hagency."))
    }
}

fn is_loopback(url: &matrix_sdk::reqwest::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(name)) => name.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    }
}

/// 16kHz mono PCM16 WAV, generated in memory. No temporary recording files.
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let size = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(size as usize + 44);
    out.extend_from_slice(b"RIFF"); out.extend_from_slice(&(size + 36).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt "); out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&16000u32.to_le_bytes()); out.extend_from_slice(&32000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes()); out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data"); out.extend_from_slice(&size.to_le_bytes());
    for sample in samples {out.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());}
    out
}

fn multipart(config: &SpeechConfig, audio: &[u8], boundary: &str) -> Vec<u8> {
    let mut fields = vec![("response_format", "json".to_owned())];
    if config.provider == Provider::OpenAiCompatible {fields.push(("model", config.model.clone()));}
    if !config.locale.is_empty() {
        if config.provider == Provider::OpenAiCompatible && config.model.starts_with("gpt-transcribe") {
            fields.push(("languages[]", config.locale.to_ascii_lowercase()));
        } else {
            fields.push(("language", config.locale.split('-').next().unwrap_or("").to_ascii_lowercase()));
        }
    } else if config.provider == Provider::WhisperCpp {
        fields.push(("language", "auto".into()));
        fields.push(("detect_language", "true".into()));
    }
    if !config.prompt.is_empty() {fields.push(("prompt", config.prompt.clone()));}
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
    }
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"dictation.wav\"\r\nContent-Type: audio/wav\r\n\r\n").as_bytes());
    body.extend_from_slice(audio);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

pub async fn transcribe(config: SpeechConfig, credential: Option<String>, audio: Vec<u8>) -> Result<String, SpeechError> {
    transcribe_inner(config, credential, audio, false).await
}

pub async fn test_service(config: SpeechConfig, credential: Option<String>) -> Result<(), SpeechError> {
    transcribe_inner(config, credential, wav(&vec![0.0; 16000]), true).await.map(|_| ())
}

async fn transcribe_inner(config: SpeechConfig, credential: Option<String>, audio: Vec<u8>, probe: bool) -> Result<String, SpeechError> {
    ensure_supported(&config)?;
    config.validate()?;
    if config.provider == Provider::LocalCodex {
        let pcm = audio.get(44..).filter(|pcm| pcm.len() >= 3200).ok_or_else(|| error("No microphone audio was captured. Check the microphone and its permission."))?;
        let service = crate::hagency::backend::Service::for_current().map_err(|_| error("Sign in to Hagency, then connect your local Codex in Agents → Model & resources."))?;
        return service.transcribe_speech(pcm, &config.locale, &config.prompt, probe).await.map_err(|code| codex_error(&code));
    }
    if config.provider == Provider::System {return Err(error("Select a local or compatible speech service for HTTP transcription."));}
    let boundary = format!("hagency-{}", uuid::Uuid::new_v4());
    let body = multipart(&config, &audio, &boundary);
    let mut builder = crate::http::client_builder().connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120)).redirect(matrix_sdk::reqwest::redirect::Policy::none());
    if matrix_sdk::reqwest::Url::parse(config.endpoint()).is_ok_and(|url| is_loopback(&url)) {builder = builder.no_proxy();}
    let client = builder.build().map_err(|_| error("Could not initialize the speech HTTP client."))?;
    let mut request = client.post(config.endpoint()).header("Content-Type", format!("multipart/form-data; boundary={boundary}")).body(body);
    if let Some(key) = credential {request = request.bearer_auth(key);}
    let mut response = request.send().await.map_err(|_| error("Could not reach the speech service. Check its URL, TLS and connection."))?;
    if !response.status().is_success() {
        return Err(error(match response.status().as_u16() {
            401 | 403 => "Speech authentication failed. Check the API key and account access.",
            404 => "The speech endpoint was not found. Enter the full transcription URL.",
            429 => "The speech service is busy or its quota has been reached.",
            _ => "The speech service rejected the request. Check its model, language and server configuration.",
        }));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| error("The speech response was interrupted."))? {
        if bytes.len() + chunk.len() > 1024 * 1024 {return Err(error("The speech response is too large."));}
        bytes.extend_from_slice(&chunk);
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| error("The speech service did not return a JSON transcript."))?;
    value.get("text").and_then(|text| text.as_str()).filter(|text| text.len() <= 100_000)
        .map(|text| text.trim().to_owned()).ok_or_else(|| error("The speech response has no text field."))
}

pub(crate) fn codex_error(code: &str) -> SpeechError {
    if code == hagency::console::speech::CODEX_PLATFORM_UNSUPPORTED {
        return SpeechError::new(SpeechErrorKind::Unavailable, crate::i18n::tr(
            Capabilities::current().unavailable_message(Provider::LocalCodex).unwrap_or(CODEX_UNSUPPORTED)));
    }
    error(match code {
        "speech_codex_local_required" => "Connect your local Codex account in Agents → Model & resources before using Codex speech.",
        "local_codex_account_not_signed_in" => "Codex is not signed in. Sign in through your local Codex, then check the account again.",
        "speech_codex_authentication_failed" => "Codex speech authentication failed. Your Codex login may not have access to experimental realtime voice. Your draft has been kept.",
        "speech_codex_protocol_unsupported" | "codex_protocol_mismatch" => "This Codex version does not support the required experimental speech protocol. Update local Codex and retry.",
        "speech_codex_no_transcript" => "Codex returned no user speech transcript. Your draft has been kept.",
        "speech_codex_audio_backpressure" => "Codex speech could not keep up with the microphone. Your transcribed draft has been kept; please retry.",
        "speech_codex_media_unavailable" => "Could not establish the Codex voice connection. Check your network and retry. Your draft has been kept.",
        "speech_codex_timeout" | "codex_provider_timeout" => "Codex speech timed out. Your draft has been kept.",
        "speech_codex_quota" => "Codex speech is busy or its quota has been reached. Your draft has been kept.",
        "codex_binary_unavailable" => "Codex is not installed or could not be started. Check the local Codex installation.",
        "local_codex_configuration_unsupported" | "provider_file_credentials_or_config_rejected" => "This Codex configuration cannot be safely used by Hagency. Your existing account has not been changed. Check the configuration or use a separate Codex account.",
        "matrix_account_changed" | "matrix_account_changing" => "The account changed. The speech result was discarded.",
        _ => "Codex speech is unavailable. Check the local Codex connection in Agents → Model & resources and experimental voice access. Your draft has been kept.",
    })
}

#[derive(Debug)]
pub struct SettingsSaved(pub SpeechConfig);

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::{Read, Write}, net::TcpListener, thread};

    #[test]
    fn capability_matrix_keeps_plain_linux_and_excludes_ohos_capture() {
        for (os, env, native, expected) in [
            ("windows", "msvc", true, [true, true, true, true]),
            ("macos", "", true, [true, true, true, true]),
            ("linux", "gnu", false, [false, true, true, true]),
            ("linux", "musl", false, [false, true, true, true]),
            ("linux", "ohos", false, [false, false, false, false]),
            ("android", "", true, [true, true, true, false]),
            ("ios", "", true, [true, true, true, false]),
            ("unknown", "", false, [false, false, false, false]),
        ] {
            let capabilities = Capabilities::for_target(os, env, native);
            for (provider, supported) in [Provider::System, Provider::WhisperCpp, Provider::OpenAiCompatible, Provider::LocalCodex].into_iter().zip(expected) {
                assert_eq!(capabilities.supports(provider), supported, "{os}/{env} {provider:?}");
                assert_eq!(capabilities.unavailable_message(provider).is_none(), supported);
                match capabilities.ensure(provider) {
                    Ok(()) => assert!(supported),
                    Err(error) => {
                        assert!(!supported);
                        assert_eq!(error.kind(), SpeechErrorKind::Unavailable);
                        assert_eq!(error.message(), crate::i18n::tr(capabilities.unavailable_message(provider).unwrap()));
                    }
                }
            }
        }
        assert_eq!(Capabilities::current().supports(Provider::LocalCodex), hagency::console::speech::local_codex_supported());
    }

    #[test]
    fn unavailable_platforms_do_not_recommend_another_unavailable_service() {
        for (os, env) in [("linux", "ohos"), ("unknown", "")] {
            let capabilities = Capabilities::for_target(os, env, false);
            for provider in [Provider::System, Provider::WhisperCpp, Provider::OpenAiCompatible, Provider::LocalCodex] {
                assert_eq!(capabilities.unavailable_message(provider), Some(PLATFORM_UNSUPPORTED));
            }
            assert_eq!(native_description_with_capabilities(capabilities), PLATFORM_UNSUPPORTED);
        }
        assert_eq!(Capabilities::for_target("linux", "gnu", false).unavailable_message(Provider::System), Some(SYSTEM_UNSUPPORTED));
        assert_eq!(Capabilities::for_target("ios", "", true).unavailable_message(Provider::LocalCodex), Some(CODEX_UNSUPPORTED));
    }

    #[test]
    fn codex_platform_and_protocol_failures_have_distinct_guidance() {
        let platform = codex_error(hagency::console::speech::CODEX_PLATFORM_UNSUPPORTED);
        let protocol = codex_error("speech_codex_protocol_unsupported");
        assert_eq!(platform.kind(), SpeechErrorKind::Unavailable);
        assert_eq!(platform.message(), crate::i18n::tr(Capabilities::current().unavailable_message(Provider::LocalCodex).unwrap_or(CODEX_UNSUPPORTED)));
        assert_ne!(platform.message(), protocol.message());
        assert_ne!(protocol.kind(), SpeechErrorKind::Unavailable);
    }

    fn server(status: &str, response: &str) -> (String, thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/inference", listener.local_addr().unwrap());
        let response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{response}", response.len());
        let task = thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            let mut connection = loop {
                match listener.accept() {
                    Ok((connection, _)) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && std::time::Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                    Err(error) => panic!("mock speech listener: {error}"),
                }
            };
            // Windows may inherit the listener's nonblocking mode on accept.
            connection.set_nonblocking(false).unwrap();
            connection.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 8192];
            loop {
                let read = connection.read(&mut buffer).unwrap();
                if read == 0 {break;}
                request.extend_from_slice(&buffer[..read]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length: usize = headers.lines().find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").map(str::trim).and_then(|value| value.parse().ok())).unwrap_or(0);
                    if request.len() >= end + 4 + length {break;}
                }
            }
            let _ = connection.write_all(response.as_bytes());
            request
        });
        (endpoint, task)
    }

    #[test]
    fn provider_preferences_roundtrip_without_embedding_credentials() {
        for provider in [Provider::System, Provider::WhisperCpp, Provider::OpenAiCompatible, Provider::LocalCodex] {
            let config = SpeechConfig { provider, locale: "zh-CN".into(), ..Default::default() };
            let encoded = serde_json::to_value(&config).unwrap();
            assert!(encoded.get("api_key").is_none());
            assert_eq!(serde_json::from_value::<SpeechConfig>(encoded).unwrap(), config);
        }
        let config: SpeechConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.provider, Provider::System);
    }

    #[test]
    fn endpoints_reject_plaintext_remote_credentials_and_false_local_hosts() {
        let mut config = SpeechConfig { provider: Provider::WhisperCpp, ..Default::default() };
        for url in ["http://localhost.evil/inference", "http://192.168.1.5/inference", "https://example.com/inference", "http://user:password@localhost/inference", "http://localhost/inference?key=secret"] {
            config.whisper_endpoint = url.into(); assert!(config.validate().is_err(), "{url}");
        }
        for url in ["http://127.0.0.1:8080/inference", "http://localhost:8080/inference", "http://[::1]:8080/inference"] {
            config.whisper_endpoint = url.into(); assert!(config.validate().is_ok());
        }
        config.provider = Provider::OpenAiCompatible;
        config.api_endpoint = "http://example.com/transcribe".into(); assert!(config.validate().is_err());
        config.api_endpoint = "https://example.com/audio/transcriptions".into(); assert!(config.validate().is_ok());
        config.api_key_env = "sk-not-a-variable-name".into(); assert!(config.validate().is_err());
    }

    #[test]
    fn wav_and_language_fields_match_provider_protocols() {
        let audio = wav(&[0.0, 1.0, -1.0]);
        assert_eq!(&audio[..4], b"RIFF"); assert_eq!(&audio[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(audio[24..28].try_into().unwrap()), 16000);
        assert_eq!(audio.len(), 50);
        let mut config = SpeechConfig { provider: Provider::OpenAiCompatible, locale: "zh-CN".into(), ..Default::default() };
        let legacy = String::from_utf8_lossy(&multipart(&config, &audio, "boundary")).into_owned();
        assert!(legacy.contains("name=\"language\"\r\n\r\nzh\r\n"));
        config.model = "gpt-transcribe".into();
        let modern = String::from_utf8_lossy(&multipart(&config, &audio, "boundary")).into_owned();
        assert!(modern.contains("name=\"languages[]\"\r\n\r\nzh-cn\r\n"));
        assert!(!modern.contains("name=\"language\""));
    }

    #[test]
    fn http_adapters_upload_wav_and_return_plain_transcript() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        for provider in [Provider::WhisperCpp, Provider::OpenAiCompatible] {
            let (endpoint, server) = server("200 OK", r#"{"text":"你好，Hagency"}"#);
            let config = SpeechConfig { provider, locale: "zh-CN".into(), whisper_endpoint: endpoint.clone(), api_endpoint: endpoint, ..Default::default() };
            let result = runtime.block_on(transcribe(config, Some("test-credential".into()), wav(&vec![0.0;16000]))).unwrap();
            assert_eq!(result, "你好，Hagency");
            let request = server.join().unwrap();
            let request = String::from_utf8_lossy(&request);
            assert!(request.contains("name=\"file\"; filename=\"dictation.wav\""));
            assert!(request.contains("RIFF"));
            assert!(request.contains("authorization: Bearer test-credential"));
            assert_eq!(request.contains("name=\"model\""), provider == Provider::OpenAiCompatible);
        }
    }

    #[test]
    fn provider_errors_do_not_surface_raw_responses_or_credentials() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let (endpoint, server) = server("401 Unauthorized", "private-provider-response credential=secret");
        let config = SpeechConfig { provider: Provider::OpenAiCompatible, api_endpoint: endpoint, ..Default::default() };
        let failure = runtime.block_on(transcribe(config, None, wav(&[0.0;1600]))).unwrap_err().to_string();
        server.join().unwrap();
        assert!(failure.contains("authentication")); assert!(!failure.contains("private-provider")); assert!(!failure.contains("secret"));
    }
}
