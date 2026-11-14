//! ADR 0026/0027 presence host sidecar. The runtime owns the host process and
//! speaks presence protocol v1 over private pipes: JSON lines to the host's
//! stdin, status and frames from its stdout. No socket, port or listener.
use crate::cancellation::Cancellation;
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    sync::oneshot,
};

/// A frame line holds at most 1 MiB of PNG as base64 plus a small header.
pub const MAX_LINE: usize = 1_500_000;
const MAX_PNG: usize = 1024 * 1024;

/// What the runtime needs from a presence host: one frame of a display, and
/// rendering of already-validated presentation events.
#[async_trait]
pub trait LookHost: Send + Sync {
    /// Captures one display at 1x point resolution, excluding Astrid's overlay.
    /// Returns PNG bytes for the runtime's own validation.
    async fn capture(&self, display: u32, cancel: &Cancellation) -> Result<Vec<u8>, String>;
    /// Writes one presence protocol v1 event.
    async fn present(&self, event: Value) -> Result<(), String>;
    /// ADR 0038: the frontmost window's interactive elements read through
    /// macOS Accessibility with the latest capture, as untrusted host data
    /// (`[[id, role, label, x, y, w, h], ...]` in display points). The runtime
    /// sanitizes it. Hosts without it return None.
    fn elements(&self) -> Option<Value> {
        None
    }
}

/// How to start the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Launch {
    /// Spawn the executable directly with piped stdio. Screen Recording, if
    /// needed, is attributed to the launching terminal.
    Direct(PathBuf),
    /// Start `Astrid Presence.app` through LaunchServices with private FIFOs, so
    /// the app bundle owns the Screen Recording grant.
    App(PathBuf),
}
impl Launch {
    pub fn from_path(path: &Path) -> Self {
        if path.extension().is_some_and(|e| e == "app") {
            Self::App(path.to_path_buf())
        } else {
            Self::Direct(path.to_path_buf())
        }
    }
}

type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<Result<Vec<u8>, String>>>>>;

/// A running presence host.
pub struct PresenceHost {
    writer: tokio::sync::Mutex<Option<Box<dyn AsyncWrite + Send + Unpin>>>,
    pending: Pending,
    displays: Arc<Mutex<Vec<Value>>>,
    closed: Arc<std::sync::atomic::AtomicBool>,
    _child: Option<tokio::process::Child>,
    _reader: tokio::task::JoinHandle<()>,
}

impl PresenceHost {
    /// Starts the host and waits up to ten seconds for its `ready` line.
    pub async fn start(launch: &Launch, extra: &[String]) -> Result<Self, String> {
        let mut args = vec!["--stdin".to_string(), "--exit-at-end".to_string()];
        args.extend(extra.iter().cloned());
        let (reader, writer, child): (
            Box<dyn AsyncRead + Send + Unpin>,
            Box<dyn AsyncWrite + Send + Unpin>,
            _,
        ) = match launch {
            Launch::Direct(path) => {
                let mut child = tokio::process::Command::new(path)
                    .args(&args)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|_| "presence host could not be started".to_string())?;
                let stdin = child
                    .stdin
                    .take()
                    .ok_or("presence host stdin unavailable")?;
                let stdout = child
                    .stdout
                    .take()
                    .ok_or("presence host stdout unavailable")?;
                (Box::new(stdout), Box::new(stdin), Some(child))
            }
            Launch::App(app) => {
                let (reader, writer, child) = launch_app(app, &args).await?;
                (reader, writer, Some(child))
            }
        };
        Self::from_pipes(reader, writer, child).await
    }

    /// Wires an already-connected host. Used directly by tests.
    pub async fn from_pipes(
        reader: Box<dyn AsyncRead + Send + Unpin>,
        writer: Box<dyn AsyncWrite + Send + Unpin>,
        child: Option<tokio::process::Child>,
    ) -> Result<Self, String> {
        let pending: Pending = Arc::default();
        let displays: Arc<Mutex<Vec<Value>>> = Arc::default();
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (ready_tx, ready_rx) = oneshot::channel();
        let task = tokio::spawn(read_loop(
            reader,
            pending.clone(),
            displays.clone(),
            closed.clone(),
            ready_tx,
        ));
        match tokio::time::timeout(Duration::from_secs(10), ready_rx).await {
            Ok(Ok(())) => {}
            _ => {
                task.abort();
                return Err("presence host did not report ready within 10s".into());
            }
        }
        Ok(Self {
            writer: tokio::sync::Mutex::new(Some(writer)),
            pending,
            displays,
            closed,
            _child: child,
            _reader: task,
        })
    }

    /// Display list from the latest `ready`/`displays_changed`, in host order.
    pub fn displays(&self) -> Vec<Value> {
        self.displays
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    async fn write(&self, event: &Value) -> Result<(), String> {
        let mut line = serde_json::to_vec(event).map_err(|_| "unencodable presence event")?;
        line.push(b'\n');
        let mut writer = self.writer.lock().await;
        let pipe = writer.as_mut().ok_or("presence host closed")?;
        pipe.write_all(&line)
            .await
            .map_err(|_| "presence host pipe closed")?;
        pipe.flush()
            .await
            .map_err(|_| "presence host pipe closed".into())
    }

    /// Keeps Astrid's last pointing visible for `linger`, then lets the host
    /// fade out and exit on its own. Idempotent.
    pub async fn finish(&self, linger: Duration) {
        let ms = linger.as_millis().min(60_000) as u64;
        let _ = self.write(&json!({"v":1,"type":"wait","ms":ms})).await;
        let _ = self.write(&json!({"v":1,"type":"hide"})).await;
        let _ = self.write(&json!({"v":1,"type":"wait","ms":300})).await;
        self.writer.lock().await.take();
    }

    /// Stops the host now, for cancellation.
    pub async fn quit(&self) {
        let _ = self.write(&json!({"v":1,"type":"quit"})).await;
        self.writer.lock().await.take();
    }
}

#[async_trait]
impl LookHost for PresenceHost {
    async fn capture(&self, display: u32, cancel: &Cancellation) -> Result<Vec<u8>, String> {
        if self.closed.load(std::sync::atomic::Ordering::Acquire) {
            return Err("presence host exited".into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.clone(), tx);
        let result = async {
            self.write(&json!({"v":1,"type":"capture","request_id":id,"display":display}))
                .await?;
            tokio::select! {biased;
                _ = cancel.cancelled() => Err("capture cancelled".to_string()),
                reply = rx => reply.unwrap_or_else(|_| Err("presence host exited".into())),
            }
        }
        .await;
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        result
    }

    async fn present(&self, event: Value) -> Result<(), String> {
        self.write(&event).await
    }
}

async fn read_loop(
    reader: Box<dyn AsyncRead + Send + Unpin>,
    pending: Pending,
    displays: Arc<Mutex<Vec<Value>>>,
    closed: Arc<std::sync::atomic::AtomicBool>,
    ready: oneshot::Sender<()>,
) {
    let mut ready = Some(ready);
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    loop {
        line.clear();
        // Bound every line, including a frame, before allocating more.
        let read = (&mut reader)
            .take(MAX_LINE as u64 + 1)
            .read_until(b'\n', &mut line)
            .await;
        match read {
            Ok(0) | Err(_) => break,
            Ok(_) if line.len() > MAX_LINE => break,
            Ok(_) => {}
        }
        let Ok(message) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        if message["v"] != 1 {
            continue;
        }
        match message["type"].as_str() {
            Some("ready") | Some("displays_changed") => {
                if let Some(list) = message["displays"].as_array() {
                    *displays.lock().unwrap_or_else(|e| e.into_inner()) = list.clone();
                }
                if let Some(tx) = ready.take() {
                    let _ = tx.send(());
                }
            }
            Some("frame") | Some("capture_failed") => {
                let Some(id) = message["request_id"].as_str() else {
                    continue;
                };
                let Some(tx) = pending.lock().unwrap_or_else(|e| e.into_inner()).remove(id) else {
                    continue;
                };
                let _ = tx.send(frame_bytes(&message));
            }
            _ => {}
        }
    }
    closed.store(true, std::sync::atomic::Ordering::Release);
    // Fail any waiting capture rather than leaving it to its deadline.
    pending.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// Decodes a `frame` reply, or turns `capture_failed` into its reason.
pub fn frame_bytes(message: &Value) -> Result<Vec<u8>, String> {
    if message["type"] == "capture_failed" {
        let reason = message["reason"].as_str().unwrap_or("capture_failed");
        let reason: String = reason
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .take(64)
            .collect();
        return Err(format!("host capture failed: {reason}"));
    }
    let encoded = message["png_base64"]
        .as_str()
        .ok_or("frame without png_base64")?;
    if encoded.len() > MAX_PNG.div_ceil(3) * 4 {
        return Err("frame exceeds 1 MiB".into());
    }
    STANDARD
        .decode(encoded)
        .map_err(|_| "frame base64 invalid".to_string())
}

/// Starts the app bundle via `open` with two private FIFOs as its stdio.
async fn launch_app(
    app: &Path,
    args: &[String],
) -> Result<
    (
        Box<dyn AsyncRead + Send + Unpin>,
        Box<dyn AsyncWrite + Send + Unpin>,
        tokio::process::Child,
    ),
    String,
> {
    use std::os::unix::fs::DirBuilderExt;
    let dir = std::env::temp_dir().join(format!("astrid-presence-{}", uuid::Uuid::new_v4()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&dir)
        .map_err(|_| "could not create private presence pipes")?;
    let cleanup = Cleanup(dir.clone());
    let (input, output) = (dir.join("in"), dir.join("out"));
    for fifo in [&input, &output] {
        let c = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes())
            .map_err(|_| "invalid pipe path")?;
        if unsafe { libc::mkfifo(c.as_ptr(), 0o600) } != 0 {
            return Err("could not create private presence pipes".into());
        }
    }
    let child = tokio::process::Command::new("/usr/bin/open")
        .arg("-W")
        .arg("-n")
        .arg("-a")
        .arg(app)
        .arg("--stdin")
        .arg(&input)
        .arg("--stdout")
        .arg(&output)
        .arg("--args")
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| "could not launch Astrid Presence.app")?;
    // Read the host's stdout FIFO on a plain blocking thread and bridge it to a
    // socketpair: tokio's kqueue readiness on macOS FIFOs stalled after the
    // first line, leaving the host blocked writing a frame.
    let (bridge, ours) =
        std::os::unix::net::UnixStream::pair().map_err(|_| "could not create presence bridge")?;
    let fifo = output.clone();
    std::thread::Builder::new()
        .name("astrid-presence-stdout".into())
        .spawn(move || {
            use std::io::{Read, Write};
            let (Ok(mut source), mut sink) = (std::fs::File::open(&fifo), bridge) else {
                return;
            };
            let mut buffer = [0u8; 64 * 1024];
            while let Ok(n) = source.read(&mut buffer) {
                if n == 0 || sink.write_all(&buffer[..n]).is_err() {
                    break;
                }
            }
        })
        .map_err(|_| "could not start presence reader")?;
    ours.set_nonblocking(true)
        .map_err(|_| "could not configure presence bridge")?;
    let reader = tokio::net::UnixStream::from_std(ours)
        .map_err(|_| "could not configure presence bridge")?;
    // The write end can only open once the app has opened its stdin.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let writer = loop {
        match tokio::net::unix::pipe::OpenOptions::new().open_sender(&input) {
            Ok(sender) => break sender,
            Err(_) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(50)).await
            }
            Err(_) => {
                // Release the reader thread, which may still be waiting to open.
                use std::os::unix::fs::OpenOptionsExt;
                let _ = std::fs::OpenOptions::new()
                    .write(true)
                    .custom_flags(libc::O_NONBLOCK)
                    .open(&output);
                return Err("Astrid Presence.app did not open its input within 10s".into());
            }
        }
    };
    drop(cleanup);
    Ok((Box::new(reader), Box::new(writer), child))
}

/// Removes the FIFO directory; open pipe ends stay valid after unlinking.
struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const READY: &str = "{\"v\":1,\"type\":\"ready\",\"displays\":[{\"index\":0,\"x\":0,\"y\":0,\"width\":1512,\"height\":982,\"scale\":2}]}\n";

    async fn fake_host() -> (
        PresenceHost,
        tokio::io::DuplexStream,
        tokio::io::DuplexStream,
    ) {
        let (host_out, ours_in) = tokio::io::duplex(4 * MAX_LINE);
        let (ours_out, host_in) = tokio::io::duplex(64 * 1024);
        let mut host_out = host_out;
        host_out.write_all(READY.as_bytes()).await.unwrap();
        let host = PresenceHost::from_pipes(Box::new(ours_in), Box::new(ours_out), None)
            .await
            .unwrap();
        (host, host_out, host_in)
    }

    async fn next_line(input: &mut tokio::io::DuplexStream) -> Value {
        let mut reader = BufReader::new(input);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        serde_json::from_str(&line).unwrap()
    }

    #[tokio::test]
    async fn capture_round_trips_a_frame_by_request_id() {
        let (host, mut host_out, mut host_in) = fake_host().await;
        assert_eq!(host.displays()[0]["width"], 1512);
        let cancel = Cancellation::default();
        let capture = host.capture(0, &cancel);
        let respond = async {
            let request = next_line(&mut host_in).await;
            assert_eq!(request["type"], "capture");
            assert_eq!(request["display"], 0);
            let id = request["request_id"].as_str().unwrap().to_owned();
            // An unrelated reply is ignored.
            host_out
                .write_all(b"{\"v\":1,\"type\":\"frame\",\"request_id\":\"other\",\"png_base64\":\"AA==\"}\n")
                .await
                .unwrap();
            let line = json!({"v":1,"type":"frame","request_id":id,"display":0,"width":1,"height":1,"png_base64":STANDARD.encode(b"png!")});
            host_out
                .write_all(format!("{line}\n").as_bytes())
                .await
                .unwrap();
        };
        let (bytes, ()) = tokio::join!(capture, respond);
        assert_eq!(bytes.unwrap(), b"png!");
    }

    #[tokio::test]
    async fn capture_failure_reason_is_sanitized() {
        let (host, mut host_out, mut host_in) = fake_host().await;
        let cancel = Cancellation::default();
        let (result, ()) = tokio::join!(host.capture(0, &cancel), async {
            let id = next_line(&mut host_in).await["request_id"]
                .as_str()
                .unwrap()
                .to_owned();
            let line = json!({"v":1,"type":"capture_failed","request_id":id,"reason":"screen_recording_permission_required\u{1b}[2J"});
            host_out
                .write_all(format!("{line}\n").as_bytes())
                .await
                .unwrap();
        });
        assert_eq!(
            result.unwrap_err(),
            "host capture failed: screen_recording_permission_required2J"
        );
    }

    #[tokio::test]
    async fn cancellation_and_host_exit_end_a_pending_capture() {
        let (host, host_out, _host_in) = fake_host().await;
        let cancel = Cancellation::default();
        cancel.cancel();
        assert_eq!(
            host.capture(0, &cancel).await.unwrap_err(),
            "capture cancelled"
        );
        drop(host_out);
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(
            host.capture(0, &Cancellation::default()).await.unwrap_err(),
            "presence host exited"
        );
    }

    #[tokio::test]
    async fn oversized_lines_close_the_host_without_buffering_them() {
        let (host, mut host_out, _host_in) = fake_host().await;
        let big = vec![b'a'; MAX_LINE + 10];
        host_out.write_all(&big).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(host.closed.load(std::sync::atomic::Ordering::Acquire));
    }

    #[tokio::test]
    async fn missing_ready_fails_start() {
        tokio::time::pause();
        let (_host_out, ours_in) = tokio::io::duplex(1024);
        let (ours_out, _host_in) = tokio::io::duplex(1024);
        let started = PresenceHost::from_pipes(Box::new(ours_in), Box::new(ours_out), None);
        assert!(started.await.is_err());
    }

    #[tokio::test]
    async fn finish_lingers_hides_and_closes_input() {
        let (host, _host_out, mut host_in) = fake_host().await;
        host.present(json!({"v":1,"type":"say","text":"hi"}))
            .await
            .unwrap();
        host.finish(Duration::from_secs(8)).await;
        let mut all = String::new();
        host_in.read_to_string(&mut all).await.unwrap();
        let types: Vec<Value> = all
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap()["type"].clone())
            .collect();
        assert_eq!(types, ["say", "wait", "hide", "wait"]);
        assert!(all.contains("\"ms\":8000"));
        assert!(host.present(json!({"v":1,"type":"hide"})).await.is_err());
    }

    #[test]
    fn launch_kind_follows_the_path() {
        assert!(matches!(
            Launch::from_path(Path::new("/x/Astrid Presence.app")),
            Launch::App(_)
        ));
        assert!(matches!(
            Launch::from_path(Path::new("/x/astrid-presence")),
            Launch::Direct(_)
        ));
    }

    #[test]
    fn frame_bytes_bounds_base64_before_decoding() {
        let huge = "A".repeat(MAX_PNG.div_ceil(3) * 4 + 4);
        assert!(frame_bytes(&json!({"type":"frame","png_base64":huge})).is_err());
        assert!(frame_bytes(&json!({"type":"frame"})).is_err());
    }
}

/// Exercises the real Swift host when `ASTRID_PRESENCE_HOST_TEST` names its
/// executable or `Astrid Presence.app` (synthetic capture; no screen access).
#[cfg(test)]
mod host_integration {
    use super::*;

    #[tokio::test]
    async fn real_host_synthetic_frame_passes_runtime_png_validation() {
        let Ok(path) = std::env::var("ASTRID_PRESENCE_HOST_TEST") else {
            eprintln!("skipped: ASTRID_PRESENCE_HOST_TEST not set");
            return;
        };
        let launch = Launch::from_path(Path::new(&path));
        let host = PresenceHost::start(&launch, &["--capture-source".into(), "synthetic".into()])
            .await
            .expect("host starts and reports ready");
        let display = &host.displays()[0];
        eprintln!("ready: {display}");
        let png = tokio::time::timeout(
            Duration::from_secs(10),
            host.capture(0, &Cancellation::default()),
        )
        .await
        .expect("frame within 10s")
        .expect("synthetic frame");
        eprintln!("frame: {} bytes", png.len());
        let canonical = crate::computer_png::canonicalize(&png).expect("runtime accepts host PNG");
        assert_eq!(
            u64::from(canonical.width),
            display["width"].as_f64().unwrap() as u64
        );
        assert_eq!(
            u64::from(canonical.height),
            display["height"].as_f64().unwrap() as u64
        );
        assert!(host.capture(9, &Cancellation::default()).await.is_err());
        host.present(json!({"v":1,"type":"point_at","x":10,"y":10,"display":0}))
            .await
            .unwrap();
        host.quit().await;
    }
}
