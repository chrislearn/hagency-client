//! Shared Makepad microphone capture with live Codex and file-service transports.
//! Native OS dictation continues to own its own capture implementation.
use super::{SpeechConfig, error, transcribe, wav};
use makepad_widgets::{makepad_platform::audio::*, thread::SignalToUI, *};
use robius_speech::{NativeSpeechEvent, SpeechError};
use std::sync::{Arc, Mutex, Weak, atomic::{AtomicBool, Ordering}};

type Callback = Arc<dyn Fn(NativeSpeechEvent) + Send + Sync>;

#[derive(Default)]
struct CaptureService {
    devices: Option<AudioDevicesEvent>,
    active: Weak<Recording>,
    running: Vec<AudioDeviceId>,
}

#[derive(Default)]
struct Samples {
    data: Vec<f32>,
    phase: f64,
    previous: f32,
    total: usize,
    stream: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
}

struct Recording {
    samples: Mutex<Samples>,
    config: SpeechConfig,
    credential: Mutex<Option<String>>,
    callback: Callback,
    active: AtomicBool,
    started: AtomicBool,
    limit_reached: AtomicBool,
    finishing: AtomicBool,
    cancelled: AtomicBool,
    done: AtomicBool,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Recording {
    fn emit(&self, event: NativeSpeechEvent) {
        if !self.cancelled.load(Ordering::Acquire) {(self.callback)(event);}
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.active.store(false, Ordering::Release);
        if let Some(task) = self.task.lock().unwrap().take() {task.abort();}
        let mut samples = self.samples.lock().unwrap();
        samples.data.clear();
        samples.stream.take();
        SignalToUI::set_ui_signal();
    }

    fn fail(&self, failure: SpeechError) {
        self.active.store(false, Ordering::Release);
        self.done.store(true, Ordering::Release);
        self.samples.lock().unwrap().stream.take();
        if let Some(task) = self.task.lock().unwrap().take() {task.abort();}
        self.emit(NativeSpeechEvent::Error(failure));
        SignalToUI::set_ui_signal();
    }

    fn finish(self: &Arc<Self>) {
        if self.cancelled.load(Ordering::Acquire) || self.finishing.swap(true, Ordering::AcqRel) {return;}
        self.active.store(false, Ordering::Release);
        SignalToUI::set_ui_signal();
        let mut buffer = self.samples.lock().unwrap();
        let samples = std::mem::take(&mut buffer.data);
        if self.config.provider == super::Provider::LocalCodex {
            let sender = buffer.stream.take();
            let total = buffer.total;
            drop(buffer);
            if total < 1600 {
                self.fail(error("No microphone audio was captured. Check the microphone and its permission."));
            } else if !samples.is_empty() && sender.is_some_and(|tx| tx.try_send(pcm(&samples)).is_err()) {
                self.fail(super::codex_error("speech_codex_audio_backpressure"));
            }
            // Dropping the final sender tells the connected peer to finalize.
            return;
        }
        drop(buffer);
        if samples.len() < 1600 {
            self.fail(error("No microphone audio was captured. Check the microphone and its permission."));
            return;
        }
        let audio = wav(&samples);
        let config = self.config.clone();
        let credential = self.credential.lock().unwrap().take();
        let this = self.clone();
        let task = crate::account_session::spawn(async move {
            match transcribe(config, credential, audio).await {
                Ok(text) if !text.is_empty() => this.emit(NativeSpeechEvent::Transcript {text, is_final: true}),
                Ok(_) => this.emit(NativeSpeechEvent::Error(error("No speech was recognized. Your draft has been kept."))),
                Err(failure) => this.emit(NativeSpeechEvent::Error(failure)),
            }
            this.done.store(true, Ordering::Release);
            this.emit(NativeSpeechEvent::Stopped);
        });
        *self.task.lock().unwrap() = Some(task);
    }

    fn start_stream(self: &Arc<Self>) {
        let (sender, audio) = hagency::console::speech::channel();
        self.samples.lock().unwrap().stream = Some(sender);
        let this = self.clone();
        let weak = Arc::downgrade(self);
        let updates: hagency::console::speech::Updates = Arc::new(move |event| {
            let Some(this) = weak.upgrade() else {return;};
            if this.cancelled.load(Ordering::Acquire) || this.done.load(Ordering::Acquire) {return;}
            match event {
                hagency::console::speech::Update::Ready => {
                    if !this.finishing.load(Ordering::Acquire) {this.active.store(true, Ordering::Release);}
                    SignalToUI::set_ui_signal();
                }
                hagency::console::speech::Update::Transcript {text, is_final} => {
                    this.emit(NativeSpeechEvent::Transcript {text, is_final});
                }
            }
        });
        let task = crate::account_session::spawn(async move {
            let result = async {
                let service = crate::hagency::backend::Service::for_current().map_err(|code| super::codex_error(&code))?;
                service.stream_speech(audio, &this.config.locale, &this.config.prompt, updates).await.map_err(|code| super::codex_error(&code))
            }.await;
            if let Err(failure) = result {this.emit(NativeSpeechEvent::Error(failure));}
            this.active.store(false, Ordering::Release);
            this.done.store(true, Ordering::Release);
            this.samples.lock().unwrap().stream.take();
            this.emit(NativeSpeechEvent::Stopped);
            SignalToUI::set_ui_signal();
        });
        *self.task.lock().unwrap() = Some(task);
    }

    fn push(&self, info: AudioInfo, audio: &AudioBuffer) {
        if !self.active.load(Ordering::Acquire) {return;}
        if !info.sample_rate.is_finite() || !(8000.0..=192000.0).contains(&info.sample_rate) || audio.channel_count() == 0 {return;}
        let maximum = self.config.max_seconds as usize * 16000;
        let mut samples = self.samples.lock().unwrap();
        if !self.active.load(Ordering::Acquire) {return;}
        if !self.started.swap(true, Ordering::AcqRel) {self.emit(NativeSpeechEvent::Started);}
        let mut peak = 0.0f32;
        for frame in 0..audio.frame_count() {
            let value = (0..audio.channel_count()).map(|channel| audio.channel(channel)[frame]).sum::<f32>() / audio.channel_count() as f32;
            let value = if value.is_finite() {value.clamp(-1.0, 1.0)} else {0.0};
            peak = peak.max(value.abs());
            samples.phase += 16000.0 / info.sample_rate;
            while samples.phase >= 1.0 && samples.total < maximum {
                samples.phase -= 1.0;
                let fraction = (1.0 - samples.phase * info.sample_rate / 16000.0).clamp(0.0, 1.0) as f32;
                let interpolated = samples.previous + (value - samples.previous) * fraction;
                samples.data.push(interpolated);
                samples.total += 1;
                if samples.stream.is_some() && samples.data.len() == 320 {
                    let frame = pcm(&samples.data);
                    samples.data.clear();
                    if samples.stream.as_ref().unwrap().try_send(frame).is_err() {
                        drop(samples);
                        self.fail(super::codex_error("speech_codex_audio_backpressure"));
                        return;
                    }
                }
            }
            samples.previous = value;
            if samples.total >= maximum {
                self.active.store(false, Ordering::Release);
                self.limit_reached.store(true, Ordering::Release);
                SignalToUI::set_ui_signal();
                break;
            }
        }
        drop(samples);
        self.emit(NativeSpeechEvent::AudioLevel(peak));
    }
}

fn pcm(samples: &[f32]) -> Vec<u8> {
    samples.iter().flat_map(|value| ((value.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect()
}

pub struct RemoteSession(Arc<Recording>);
impl RemoteSession {
    pub fn start(cx: &mut Cx, config: SpeechConfig, callback: Callback) -> Result<Self, SpeechError> {
        super::ensure_supported(&config)?;
        config.validate()?;
        let credential = config.credential()?;
        if cx.global::<CaptureService>().active.upgrade().is_some_and(|recording|
            !recording.cancelled.load(Ordering::Acquire) && !recording.done.load(Ordering::Acquire)) {
            return Err(error("Another speech recording is already active."));
        }
        let live = config.provider == super::Provider::LocalCodex;
        let recording = Arc::new(Recording {
            samples: Mutex::new(Samples::default()), config, credential: Mutex::new(credential), callback,
            active: AtomicBool::new(!live), started: AtomicBool::new(false), limit_reached: AtomicBool::new(false),
            finishing: AtomicBool::new(false), cancelled: AtomicBool::new(false), done: AtomicBool::new(false),
            task: Mutex::new(None),
        });
        if live {recording.start_stream();}
        cx.global::<CaptureService>().active = Arc::downgrade(&recording);
        let weak = Arc::downgrade(&recording);
        // Installing the callback enumerates devices; opening a microphone is
        // deferred to the selected default INPUT device in handle_event.
        cx.audio_input(0, move |info, audio| {
            if let Some(recording) = weak.upgrade() {recording.push(info, audio);}
        });
        maintain(cx);
        Ok(Self(recording))
    }
    pub fn stop(&self) {self.0.finish();}
    pub fn cancel(&self) {self.0.cancel();}
    pub fn cancelled(&self) -> bool {self.0.cancelled.load(Ordering::Acquire)}
    pub fn finishing(&self) -> bool {self.0.finishing.load(Ordering::Acquire)}
    pub fn finish_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(if self.0.config.provider == super::Provider::LocalCodex {
            35
        } else {135})
    }
}
impl Drop for RemoteSession {
    fn drop(&mut self) {self.0.cancel();}
}

fn maintain(cx: &mut Cx) {
    let service = cx.global::<CaptureService>();
    let active = service.active.upgrade();
    if let Some(recording) = &active {
        if recording.limit_reached.load(Ordering::Acquire) {recording.finish();}
    }
    let desired = if let Some(recording) = &active {
        if recording.active.load(Ordering::Acquire) {
            service.devices.as_ref().map(|devices| devices.descs.iter()
                .filter(|device| device.is_default && device.device_type == AudioDeviceType::Input && !device.has_failed)
                .map(|device| device.device_id).take(1).collect::<Vec<_>>())
        } else {Some(Vec::new())}
    } else {Some(Vec::new())};
    if let Some(desired) = desired {
        if desired.is_empty() && active.as_ref().is_some_and(|recording| recording.active.load(Ordering::Acquire)) {
            active.as_ref().unwrap().fail(error("The default microphone is unavailable. Check your audio device and microphone permission."));
        }
        if service.running != desired {
            service.running = desired.clone();
            cx.use_audio_inputs(&desired);
        }
    }
}

pub fn cancel_all(cx: &mut Cx) {
    if let Some(recording) = cx.global::<CaptureService>().active.upgrade() {recording.cancel();}
    maintain(cx);
}

pub fn handle_event(cx: &mut Cx, event: &Event) {
    if let Event::AudioDevices(devices) = event {cx.global::<CaptureService>().devices = Some(devices.clone());}
    if matches!(event, Event::Shutdown | Event::Background) {
        cancel_all(cx);
    }
    maintain(cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn recording() -> Arc<Recording> {
        Arc::new(Recording {
            samples: Mutex::new(Samples::default()), config: SpeechConfig {max_seconds: 10, ..SpeechConfig::default()},
            credential: Mutex::new(None), callback: Arc::new(|_| {}),
            active: AtomicBool::new(true), started: AtomicBool::new(false), limit_reached: AtomicBool::new(false),
            finishing: AtomicBool::new(false), cancelled: AtomicBool::new(false), done: AtomicBool::new(false), task: Mutex::new(None),
        })
    }
    #[test]
    fn microphone_frames_are_downmixed_and_resampled_to_16khz() {
        for rate in [8000, 44100, 48000] {
            let recording = recording();
            let mut audio = AudioBuffer::new_with_size(rate, 2);
            audio.channel_mut(0).fill(0.25); audio.channel_mut(1).fill(0.75);
            recording.push(AudioInfo {device_id: AudioDeviceId::default(), time: None, sample_rate: rate as f64}, &audio);
            let samples = recording.samples.lock().unwrap();
            assert!((samples.data.len() as isize - 16000).abs() <= 1, "rate={rate} samples={}", samples.data.len());
            assert!((samples.data.last().unwrap() - 0.5).abs() < 0.001);
            assert!(!recording.finishing.load(Ordering::Acquire), "capture alone must not start an upload");
        }
    }
    #[test]
    fn live_capture_sends_before_stop_flushes_tail_and_bounds_backpressure() {
        let mut recording = recording();
        Arc::get_mut(&mut recording).unwrap().config.provider = super::super::Provider::LocalCodex;
        let (tx,mut rx) = hagency::console::speech::channel();
        recording.samples.lock().unwrap().stream = Some(tx);
        let mut audio = AudioBuffer::new_with_size(1610,1); audio.data.fill(0.5);
        recording.push(AudioInfo {device_id:AudioDeviceId::default(),time:None,sample_rate:16000.0},&audio);
        for _ in 0..5 {assert_eq!(rx.try_recv().unwrap().len(),640);}
        assert_eq!(recording.samples.lock().unwrap().data.len(),10);
        assert!(!recording.finishing.load(Ordering::Acquire));
        recording.finish();
        assert_eq!(rx.try_recv().unwrap().len(),20);
        assert!(matches!(rx.try_recv(),Err(tokio::sync::mpsc::error::TryRecvError::Disconnected)));

        let recording = self::recording();
        let (tx,_rx) = hagency::console::speech::channel();
        recording.samples.lock().unwrap().stream = Some(tx);
        let audio = AudioBuffer::new_with_size(64000,1);
        recording.push(AudioInfo {device_id:AudioDeviceId::default(),time:None,sample_rate:16000.0},&audio);
        assert!(recording.done.load(Ordering::Acquire));
        assert!(!recording.active.load(Ordering::Acquire));
        assert!(recording.samples.lock().unwrap().data.len() < 320);
    }
    #[test]
    fn recording_limit_and_cancellation_bound_retained_audio() {
        let recording = recording();
        let mut audio = AudioBuffer::new_with_size(16000, 1); audio.data.fill(0.5);
        for _ in 0..15 {recording.push(AudioInfo {device_id: AudioDeviceId::default(), time: None, sample_rate: 16000.0}, &audio);}
        assert_eq!(recording.samples.lock().unwrap().data.len(), 160000);
        assert!(recording.limit_reached.load(Ordering::Acquire));
        recording.cancel();
        recording.push(AudioInfo {device_id: AudioDeviceId::default(), time: None, sample_rate: 16000.0}, &audio);
        assert!(recording.samples.lock().unwrap().data.is_empty());
        assert!(!recording.active.load(Ordering::Acquire));
    }
}
