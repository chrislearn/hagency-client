//! Bounded live dictation boundary. PCM16 mono 16 kHz, at most 20 ms per chunk.
use std::sync::Arc;

pub const FRAME_BYTES: usize = 640;
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
