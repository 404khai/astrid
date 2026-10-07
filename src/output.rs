//! Bounded raw tool output; byte counts describe observed pipe bytes only.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc;

pub const CHUNK_BYTES: usize = 8 * 1024;
pub const CAPTURE_BYTES: usize = 64 * 1024;
pub const QUEUED_CHUNKS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolOutput {
    pub stream: OutputStream,
    pub bytes: Vec<u8>,
}

/// Decode chunks without treating a split UTF-8 codepoint as malformed text.
/// Invalid complete sequences use replacement characters; incomplete terminal
/// suffixes remain unavailable for text rendering rather than being fabricated.
#[derive(Default)]
pub struct TextDecoder {
    pending: Vec<u8>,
}
impl TextDecoder {
    pub fn unavailable_suffix_bytes(&self) -> usize {
        self.pending.len()
    }
    pub fn push(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        let mut text = String::new();
        let mut start = 0;
        while start < self.pending.len() {
            match std::str::from_utf8(&self.pending[start..]) {
                Ok(valid) => {
                    text.push_str(valid);
                    start = self.pending.len();
                }
                Err(error) => {
                    let valid_end = start + error.valid_up_to();
                    text.push_str(
                        std::str::from_utf8(&self.pending[start..valid_end])
                            .expect("validated prefix"),
                    );
                    start = valid_end;
                    if let Some(length) = error.error_len() {
                        text.push('\u{fffd}');
                        start += length;
                    } else {
                        break;
                    }
                }
            }
        }
        self.pending.drain(..start);
        debug_assert!(self.pending.len() <= 3);
        text
    }
}

#[derive(Default)]
pub(crate) struct Capture {
    bytes: Vec<u8>,
    observed: u64,
    queued: u64,
}

impl Capture {
    pub(crate) fn observe(
        &mut self,
        bytes: &[u8],
        stream: OutputStream,
        sender: Option<&mpsc::Sender<ToolOutput>>,
        live_stopped: &mut bool,
    ) {
        self.observed = self.observed.saturating_add(bytes.len() as u64);
        let keep = bytes.len().min(CAPTURE_BYTES - self.bytes.len());
        self.bytes.extend_from_slice(&bytes[..keep]);
        if keep > 0
            && !*live_stopped
            && let Some(sender) = sender
        {
            match sender.try_send(ToolOutput {
                stream,
                bytes: bytes[..keep].to_vec(),
            }) {
                Ok(()) => self.queued += keep as u64,
                Err(_) => *live_stopped = true,
            }
        }
    }

    fn text(&self) -> String {
        TextDecoder::default().push(&self.bytes)
    }

    fn metadata(&self, complete: bool, live_available: bool) -> Value {
        let mut decoder = TextDecoder::default();
        let _ = decoder.push(&self.bytes);
        json!({
            "observed_bytes": self.observed,
            "captured_bytes": self.bytes.len(),
            "capture_omitted_bytes": self.observed.saturating_sub(self.bytes.len() as u64),
            "live_queued_bytes": self.queued,
            "live_omitted_bytes": self.observed.saturating_sub(self.queued),
            "complete": complete,
            "live_available": live_available,
            "truncated": !complete || self.observed > self.bytes.len() as u64,
            "limit_bytes": CAPTURE_BYTES,
            "text_unavailable_suffix_bytes": decoder.unavailable_suffix_bytes(),
        })
    }
}

pub(crate) fn result(
    stdout: &Capture,
    stderr: &Capture,
    exit_code: Option<i32>,
    timed_out: bool,
    cancelled: bool,
    complete: bool,
    live_available: bool,
) -> Value {
    json!({
        "stdout": stdout.text(), "stderr": stderr.text(),
        "exit_code": exit_code, "timed_out": timed_out, "cancelled": cancelled,
        "output": {
            "stdout": stdout.metadata(complete, live_available),
            "stderr": stderr.metadata(complete, live_available),
        }
    })
}
