#![allow(dead_code)]
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
pub fn command() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_hagency-cli"));
    c.env("PATH", "").stdin(Stdio::null());
    c
}
pub fn assets(dir: &Path) -> PathBuf {
    use sha2::Digest;
    let root = dir.join("console-assets");
    hagency_store::private::directory(&root).unwrap();
    let bytes = b"<!doctype html><html><body>owner CLI fixture</body></html>";
    let hash = format!("{:x}", sha2::Sha256::digest(bytes));
    let entries=["index.html","login/index.html","agents-owned/index.html", "projects/index.html", "projects/new/index.html"].map(|name| {
        let file=root.join(name); if let Some(parent)=file.parent(){fs::create_dir_all(parent).unwrap();}
        hagency_store::private::write_new(&file,bytes).unwrap();
        serde_json::json!({"path":name,"size":bytes.len(),"sha256":hash,"mime":"text/html; charset=utf-8"})
    });
    hagency_store::private::write_new(
        &root.join("manifest.json"),
        serde_json::json!({"version":1,"assets":entries})
            .to_string()
            .as_bytes(),
    )
    .unwrap();
    root.canonicalize().unwrap()
}
pub struct Running {
    pub child: Child,
    pub address: SocketAddr,
}
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub fn request(
    address: SocketAddr,
    method: &str,
    path: &str,
    body: &str,
    cookie: Option<&str>,
) -> String {
    request_result(address, method, path, body, cookie).unwrap()
}
fn request_result(
    address: SocketAddr,
    method: &str,
    path: &str,
    body: &str,
    cookie: Option<&str>,
) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(address)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let cookie = cookie
        .map(|value| format!("Cookie: {value}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nOrigin: http://{address}\r\nSec-Fetch-Site: same-origin\r\nContent-Type: application/json\r\n{cookie}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    let mut out = String::new();
    if let Err(error) = stream.read_to_string(&mut out) {
        // An early authorization refusal may close with unread request bytes,
        // producing TCP RST after a complete response. Never accept truncation.
        let complete = out.split_once("\r\n\r\n").is_some_and(|(headers, body)| {
            headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .is_some_and(|length| body.len() == length)
        });
        if error.kind() != std::io::ErrorKind::ConnectionReset || !complete {
            return Err(error);
        }
    }
    Ok(out)
}
pub fn status(wire: &str) -> u16 {
    wire.lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap()
}
pub fn launch(state: &Path, assets: &Path, start: bool) -> Running {
    let address = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let mut c = command();
    c.arg(if start { "start" } else { "serve" })
        .arg("--state-dir")
        .arg(state)
        .args(["--listen", &address.to_string(), "--console-assets"])
        .arg(assets)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if start {
        c.arg("--no-open");
    }
    let mut run = Running {
        child: c.spawn().unwrap(),
        address,
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(code) = run.child.try_wait().unwrap() {
            let mut stderr = String::new();
            run.child
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut stderr)
                .unwrap();
            panic!("OwnerHost exited {code}: {stderr}");
        }
        // A successful TCP probe can precede HTTP readiness. Only this
        // side-effect-free health read retries within the existing deadline.
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok()
            && request_result(address, "GET", "/health", "", None).is_ok_and(|wire| {
                wire.lines()
                    .next()
                    .is_some_and(|line| line.split_whitespace().nth(1) == Some("200"))
            })
        {
            return run;
        }
        assert!(Instant::now() < deadline, "OwnerHost readiness timeout");
        std::thread::sleep(Duration::from_millis(20));
    }
}
pub fn marker(state: &Path) -> Vec<u8> {
    fs::read(state.join("hagency-client-owned-v1.json")).unwrap()
}
#[cfg(unix)]
pub fn stop_clean(run: &mut Running) {
    let output = Command::new("/bin/kill")
        .args(["-TERM", &run.child.id().to_string()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(code) = run.child.try_wait().unwrap() {
            assert!(code.success(), "OwnerHost graceful exit: {code}");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "TERM exceeded service stop budget"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
