//! Bounded live dictation boundary. PCM16 mono 16 kHz, at most 20 ms per chunk.
use std::sync::Arc;

pub const FRAME_BYTES: usize = 640;
pub const CODEX_PLATFORM_UNSUPPORTED: &str = "speech_codex_platform_unsupported";

/// Shared by the native transport and clients. OHOS also reports target_os=linux.
pub fn local_codex_supported_on(os: &str, env: &str) -> bool {
    env != "ohos" && matches!(os, "windows" | "macos" | "linux")
}

pub fn local_codex_supported() -> bool {
    local_codex_supported_on(std::env::consts::OS, if cfg!(target_env = "ohos") {"ohos"} else {""})
}
pub type Audio = tokio::sync::mpsc::Receiver<Vec<u8>>;
pub type Updates = Arc<dyn Fn(Update) + Send + Sync>;

#[derive(Clone, Debug)]
pub enum Update {
    Ready,
    Transcript { text: String, is_final: bool },
}

/// Two seconds maximum queued audio; producers must fail rather than drop words.
pub fn channel() -> (tokio::sync::mpsc::Sender<Vec<u8>>, Audio) {
    tokio::sync::mpsc::channel(100)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codex_platform_matrix_excludes_ohos_and_mobile() {
        for (os, env, supported) in [("windows", "msvc", true), ("macos", "", true), ("linux", "gnu", true), ("linux", "musl", true), ("linux", "ohos", false), ("android", "", false), ("ios", "", false), ("unknown", "", false)] {
            assert_eq!(local_codex_supported_on(os, env), supported, "{os}/{env}");
        }
        assert_eq!(local_codex_supported(), cfg!(any(target_os = "windows", target_os = "macos", all(target_os = "linux", not(target_env = "ohos")))));
    }
}
