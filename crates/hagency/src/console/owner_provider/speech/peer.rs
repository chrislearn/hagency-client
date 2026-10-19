//! Native WebRTC media only. Codex owns the call's authentication and signaling.
//! PCMU is negotiated explicitly; incoming audio is discarded, never played.
use super::super::{ProviderError, failure};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
use webrtc::{
    api::{
        APIBuilder,
        interceptor_registry::register_default_interceptors,
        media_engine::{MIME_TYPE_PCMU, MediaEngine},
    },
    interceptor::registry::Registry,
    media::Sample,
    peer_connection::{
        RTCPeerConnection, configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
    },
    rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType},
    track::track_local::{TrackLocal, track_local_static_sample::TrackLocalStaticSample},
};

fn unavailable() -> ProviderError {
    failure(502, "speech_codex_media_unavailable")
}

pub(super) struct Peer {
    connection: Arc<RTCPeerConnection>,
    track: Arc<TrackLocalStaticSample>,
    state: watch::Receiver<RTCPeerConnectionState>,
    channel: watch::Receiver<bool>,
    rtcp: Option<tokio::task::JoinHandle<()>>,
    closed: bool,
    events: Option<Arc<webrtc::data_channel::RTCDataChannel>>,
}
impl Peer {
    pub(super) async fn new() -> Result<Self, ProviderError> {
        // The desktop SDK also enables aws-lc. DTLS's verifier cannot choose a
        // process provider when both features are present. Respect an existing
        // choice and otherwise select ring explicitly before any handshake.
        let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();
        let codec = RTCRtpCodecCapability {
            mime_type: MIME_TYPE_PCMU.into(),
            clock_rate: 8000,
            channels: 1,
            ..Default::default()
        };
        let mut media = MediaEngine::default();
        media
            .register_codec(
                RTCRtpCodecParameters {
                    capability: codec.clone(),
                    payload_type: 0,
                    ..Default::default()
                },
                RTPCodecType::Audio,
            )
            .map_err(|_| unavailable())?;
        let registry = register_default_interceptors(Registry::new(), &mut media)
            .map_err(|_| unavailable())?;
        let api = APIBuilder::new()
            .with_media_engine(media)
            .with_interceptor_registry(registry)
            .build();
        let connection = Arc::new(
            api.new_peer_connection(RTCConfiguration::default())
                .await
                .map_err(|_| unavailable())?,
        );
        let track = Arc::new(TrackLocalStaticSample::new(
            codec,
            "dictation".into(),
            "hagency-speech".into(),
        ));
        let (state_tx, state) = watch::channel(RTCPeerConnectionState::New);
        let (channel_tx, channel) = watch::channel(false);
        let mut peer = Self {
            connection,
            track,
            state,
            channel,
            rtcp: None,
            closed: false,
            events: None,
        };
        peer.connection
            .on_peer_connection_state_change(Box::new(move |state| {
                state_tx.send_replace(state);
                Box::pin(async {})
            }));
        peer.connection.on_track(Box::new(|track, _, _| {
            tokio::spawn(async move { while track.read_rtp().await.is_ok() {} });
            Box::pin(async {})
        }));
        let sender = peer
            .connection
            .add_track(peer.track.clone() as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(|_| unavailable())?;
        peer.rtcp = Some(tokio::spawn(async move {
            while sender.read_rtcp().await.is_ok() {}
        }));
        let channel = peer
            .connection
            .create_data_channel("oai-events", None)
            .await
            .map_err(|_| unavailable())?;
        peer.events = Some(channel.clone());
        channel.on_open(Box::new(move || {
            channel_tx.send_replace(true);
            Box::pin(async {})
        }));
        Ok(peer)
    }
    pub(super) async fn offer(&self) -> Result<String, ProviderError> {
        let mut gathered = self.connection.gathering_complete_promise().await;
        let offer = self
            .connection
            .create_offer(None)
            .await
            .map_err(|_| unavailable())?;
        self.connection
            .set_local_description(offer)
            .await
            .map_err(|_| unavailable())?;
        tokio::time::timeout(Duration::from_secs(10), gathered.recv())
            .await
            .map_err(|_| unavailable())?;
        self.connection
            .local_description()
            .await
            .map(|s| s.sdp)
            .ok_or_else(unavailable)
    }
    pub(super) async fn answer(&mut self, sdp: String) -> Result<(), ProviderError> {
        if sdp.len() > 256 * 1024 {
            return Err(unavailable());
        }
        self.connection
            .set_remote_description(RTCSessionDescription::answer(sdp).map_err(|_| unavailable())?)
            .await
            .map_err(|_| unavailable())?;
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                let state = *self.state.borrow();
                if state == RTCPeerConnectionState::Connected && *self.channel.borrow() {
                    return Ok(());
                }
                if matches!(
                    state,
                    RTCPeerConnectionState::Failed
                        | RTCPeerConnectionState::Closed
                        | RTCPeerConnectionState::Disconnected
                ) {
                    return Err(unavailable());
                }
                tokio::select! {
                    r = self.state.changed() => r.map_err(|_| unavailable())?,
                    r = self.channel.changed() => r.map_err(|_| unavailable())?,
                }
            }
        })
        .await
        .map_err(|_| {
            tracing::warn!(state = ?*self.state.borrow(), channel_open = *self.channel.borrow(), "Codex speech media connection timed out");
            unavailable()
        })?
    }
    pub(super) async fn send_audio(
        &self,
        mut audio: crate::console::speech::Audio,
    ) -> Result<Option<std::time::Instant>, ProviderError> {
        let mut total = 0;
        let mut last_voice = None;
        let mut encoder = Encoder::default();
        while let Some(pcm) = audio.recv().await {
            total += pcm.len();
            if pcm.is_empty()
                || pcm.len() > crate::console::speech::FRAME_BYTES
                || !pcm.len().is_multiple_of(2)
                || total > 120 * 32000
            {
                return Err(failure(400, "invalid_arguments"));
            }
            if pcm
                .as_chunks::<2>()
                .0
                .iter()
                .any(|b| i16::from_le_bytes([b[0], b[1]]).unsigned_abs() > 512)
            {
                last_voice = Some(std::time::Instant::now());
            }
            // The capture clock paces live input. Never replay a completed clip.
            for frame in encoder.push(&pcm) {
                self.write_frame(frame).await?;
            }
        }
        if total < 3200 {
            return Err(failure(400, "speech_codex_no_transcript"));
        }
        if let Some(frame) = encoder.finish() {
            self.write_frame(frame).await?;
        }
        Ok(last_voice)
    }
    pub(super) async fn finish_input(&self) -> Result<(), ProviderError> {
        // Frameless v3's upstream wire close; no Codex realtime/stop or handoff.
        self.events
            .as_ref()
            .ok_or_else(unavailable)?
            .send_text(r#"{"type":"session.close"}"#.to_owned())
            .await
            .map_err(|_| unavailable())?;
        Ok(())
    }
    pub(super) async fn silence(&self) -> Result<(), ProviderError> {
        // Keep RTP flowing until VAD finalizes: a single short silence followed
        // by no media can leave the server waiting for its idle timeout.
        let mut clock = tokio::time::interval(Duration::from_millis(20));
        clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            clock.tick().await;
            self.write_frame(vec![0xff; 160]).await?;
        }
    }
    async fn write_frame(&self, frame: Vec<u8>) -> Result<(), ProviderError> {
        if *self.state.borrow() != RTCPeerConnectionState::Connected {
            return Err(unavailable());
        }
        self.track
            .write_sample(&Sample {
                data: frame.into(),
                duration: Duration::from_millis(20),
                ..Default::default()
            })
            .await
            .map_err(|_| unavailable())
    }
    pub(super) async fn close(&mut self) {
        if let Some(task) = self.rtcp.take() {
            task.abort();
        }
        let _ = self.connection.close().await;
        self.closed = true;
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        if let Some(task) = self.rtcp.take() {
            task.abort();
        }
        if !self.closed
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            let connection = self.connection.clone();
            runtime.spawn(async move {
                let _ = connection.close().await;
            });
        }
    }
}

/// Carries half a downsample pair and a partial RTP frame across capture chunks.
#[derive(Default)]
struct Encoder {
    pair: Option<i16>,
    frame: Vec<u8>,
}
impl Encoder {
    fn push(&mut self, pcm: &[u8]) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();
        for sample in pcm.as_chunks::<2>().0 {
            let sample = i16::from_le_bytes([sample[0], sample[1]]);
            if let Some(first) = self.pair.take() {
                self.frame
                    .push(mulaw(((first as i32 + sample as i32) / 2) as i16));
                if self.frame.len() == 160 {
                    frames.push(std::mem::take(&mut self.frame));
                }
            } else {
                self.pair = Some(sample);
            }
        }
        frames
    }
    fn finish(&mut self) -> Option<Vec<u8>> {
        if let Some(sample) = self.pair.take() {
            self.frame.push(mulaw(sample));
        }
        if self.frame.is_empty() {
            return None;
        }
        self.frame.resize(160, 0xff);
        Some(std::mem::take(&mut self.frame))
    }
}

fn mulaw(sample: i16) -> u8 {
    let sign = if sample < 0 { 0x80 } else { 0 };
    let value = (sample as i32).abs().min(32635) + 0x84;
    let mut exponent = 7;
    while exponent > 0 && value & (1 << (exponent + 7)) == 0 {
        exponent -= 1;
    }
    let mantissa = ((value >> (exponent + 3)) & 0x0f) as u8;
    !(sign | ((exponent as u8) << 4) | mantissa)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pcmu_standard_silence_and_extrema() {
        assert_eq!(mulaw(0), 0xff);
        assert_eq!(mulaw(-1), 0x7f);
        assert_eq!(mulaw(32124), 0x80);
        assert_eq!(mulaw(-32124), 0x00);
        assert_eq!(mulaw(i16::MIN), 0x00);
    }
    #[test]
    fn encoder_preserves_pairs_and_rtp_frames_across_arbitrary_chunks() {
        let pcm: Vec<u8> = (0..1001i16).flat_map(i16::to_le_bytes).collect();
        let mut whole = Encoder::default();
        let mut expected = whole.push(&pcm);
        expected.extend(whole.finish());
        for size in [2, 6, 638, 640] {
            let mut encoder = Encoder::default();
            let mut actual = Vec::new();
            for chunk in pcm.chunks(size) {
                actual.extend(encoder.push(chunk));
            }
            actual.extend(encoder.finish());
            assert_eq!(actual, expected);
            assert!(actual.iter().all(|f| f.len() == 160));
        }
    }
    #[tokio::test]
    async fn native_offer_contains_only_audio_pcmu_and_event_channel() {
        let mut peer = Peer::new().await.unwrap();
        assert!(tokio_rustls::rustls::crypto::CryptoProvider::get_default().is_some());
        let offer = peer.offer().await.unwrap();
        assert!(offer.contains("PCMU/8000"));
        assert!(offer.contains("m=application"));
        assert!(!offer.contains("m=video"));
        peer.close().await;
    }
}
