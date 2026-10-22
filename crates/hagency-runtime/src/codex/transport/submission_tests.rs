use super::*;
use std::{
    io,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};
use tokio::io::{DuplexStream, ReadBuf};
use tokio::sync::Notify;

#[derive(Default)]
struct Submission {
    submitted: AtomicBool,
    allow_completion: AtomicBool,
    completed: AtomicBool,
    frames: Mutex<Vec<Vec<u8>>>,
    wake: Mutex<Option<Waker>>,
    began: Notify,
}
struct DelayedWriter(Arc<Submission>);
impl DelayedWriter {
    fn pending(&self) -> bool {
        self.0.submitted.load(Ordering::SeqCst) && !self.0.completed.load(Ordering::SeqCst)
    }
}
impl AsyncWrite for DelayedWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let state = &self.0;
        if !state.submitted.swap(true, Ordering::SeqCst) {
            state.frames.lock().unwrap().push(bytes.to_vec());
            state.began.notify_one();
        } else {
            assert_eq!(
                state.frames.lock().unwrap()[0],
                bytes,
                "original frame changed"
            );
        }
        if state.allow_completion.load(Ordering::SeqCst) {
            state.completed.store(true, Ordering::SeqCst);
            return Poll::Ready(Ok(bytes.len()));
        }
        *state.wake.lock().unwrap() = Some(cx.waker().clone());
        Poll::Pending
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        assert!(self.0.completed.load(Ordering::SeqCst));
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
struct ObservedReader {
    inner: DuplexStream,
    read: Arc<Notify>,
}
impl AsyncRead for ObservedReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = bytes.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, bytes);
        if bytes.filled().len() > before {
            self.read.notify_one();
        }
        result
    }
}
struct Fixture {
    driver: Driver<ObservedReader, DelayedWriter, DuplexStream>,
    frame: PreparedFrame,
    stdout: DuplexStream,
    _stderr: DuplexStream,
    state: Arc<Submission>,
    read: Arc<Notify>,
}
fn line(value: Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    bytes
}
fn fixture() -> Fixture {
    let (stdout, out) = tokio::io::duplex(4096);
    let (stderr, err) = tokio::io::duplex(4096);
    let state = Arc::new(Submission::default());
    let read = Arc::new(Notify::new());
    let mut driver = Driver::new(
        ObservedReader {
            inner: stdout,
            read: read.clone(),
        },
        DelayedWriter(state.clone()),
        stderr,
        Limits {
            write_timeout_ms: 1000,
            event_wait_ms: 1000,
            lifetime_ms: 30_000,
        },
    )
    .unwrap();
    driver.observe_write_submission(DelayedWriter::pending);
    let (id, _) = driver.connection.initialize("fixture", 0, 1000).unwrap();
    driver.connection.receive(&line(serde_json::json!({"id":id,"result":{
        "userAgent":"fixture/0.153.4","platformFamily":"fixture","platformOs":"fixture","codexHome":"/fixture"
    }})), 0).unwrap();
    driver.connection.initialized(0).unwrap();
    let id = RequestId::String("approval-1".into());
    let method = "item/commandExecution/requestApproval".to_owned();
    let params = serde_json::json!({"threadId":"t","turnId":"u","itemId":"i","startedAtMs":1,"command":"echo fixture","cwd":"/work"});
    driver
        .connection
        .receive(
            &line(serde_json::json!({"id":id,"method":method,"params":params})),
            0,
        )
        .unwrap();
    let request = super::super::approval::ApprovalRequest::parse(id, method, params).unwrap();
    let frame = driver
        .prepare_approval(
            request.response(true),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    Fixture {
        driver,
        frame,
        stdout: out,
        _stderr: err,
        state,
        read,
    }
}
fn resolution() -> Vec<u8> {
    line(
        serde_json::json!({"method":"serverRequest/resolved","params":{"threadId":"t","requestId":"approval-1"}}),
    )
}

#[tokio::test]
async fn native_codex_prepared_submission_retains_original_frame_until_receipt() {
    let mut f = fixture();
    let peer = async {
        while !f.state.submitted.load(Ordering::SeqCst) {
            f.state.began.notified().await;
        }
        f.stdout.write_all(&resolution()).await.unwrap();
        f.read.notified().await;
        f.state.allow_completion.store(true, Ordering::SeqCst);
        if let Some(wake) = f.state.wake.lock().unwrap().take() {
            wake.wake();
        }
    };
    let (result, ()) = tokio::join!(f.driver.send_prepared_or_event(&mut f.frame), peer);
    let Controlled::Control(receipt) = result.unwrap() else {
        panic!("a peer read/resolution is not the original pending write receipt");
    };
    assert!(f.state.completed.load(Ordering::SeqCst));
    let frames = f.state.frames.lock().unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(receipt.bytes, frames[0].len());
    assert_eq!(
        serde_json::from_slice::<Value>(&frames[0]).unwrap(),
        serde_json::json!({"id":"approval-1","result":{"decision":"accept"}})
    );
    assert!(matches!(
        f.driver.buffered_event().unwrap(),
        Some(Event::Notification { .. })
    ));
    assert!(
        !f.driver
            .prepared_admissible(&RequestId::String("approval-1".into()))
    );
}

#[tokio::test]
async fn native_codex_prepared_submission_cancel_preserves_unknown_transmission() {
    let mut f = fixture();
    let peer = async {
        while !f.state.submitted.load(Ordering::SeqCst) {
            f.state.began.notified().await;
        }
        f.stdout.write_all(&resolution()).await.unwrap();
        f.read.notified().await;
        // Deliberately withhold the actual write completion.
    };
    let (result, ()) = tokio::join!(
        tokio::time::timeout(
            Duration::from_millis(50),
            f.driver.send_prepared_or_event(&mut f.frame)
        ),
        peer
    );
    assert!(
        result.is_err(),
        "no receipt or event may abandon a submitted frame"
    );
    let stopped = f.driver.termination().unwrap();
    assert_eq!(stopped.cause, Error::CancelledOperation);
    let write = stopped.unconfirmed_write.as_ref().unwrap();
    assert_eq!(write.accepted_bytes, 0);
    assert!(write.submission_pending);
    assert!(!write.is_definitely_untransmitted());
    assert!(!f.state.completed.load(Ordering::SeqCst));
    assert_eq!(f.state.frames.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn native_codex_prepared_resolution_before_submission_keeps_quiet_path() {
    let mut f = fixture();
    f.stdout.write_all(&resolution()).await.unwrap();
    assert!(matches!(
        f.driver.send_prepared_or_event(&mut f.frame).await.unwrap(),
        Controlled::Event(_)
    ));
    assert!(!f.state.submitted.load(Ordering::SeqCst));
    assert!(f.state.frames.lock().unwrap().is_empty());
}
