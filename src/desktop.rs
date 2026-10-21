//! ADR 0023: bounded in-process action controls, exercised with mocked transport.
//! No live transport, screenshot decoder, runtime tool, CLI or provider integration.
//! Observation metadata is a trusted host assertion, not verified image admission.
use crate::{
    cancellation::Cancellation,
    events::{RunId, ToolCallId},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::time::Instant;

pub const POLICY: &str = "cu02-native-v1";
pub const FRAME_AGE: Duration = Duration::from_secs(5);
pub const RUN_WINDOW: Duration = Duration::from_secs(30);
pub const ACTION_WAIT: Duration = Duration::from_secs(3);
const MAX_ACTIONS: usize = 2;
const MAX_RECEIPTS: usize = 2;
const MAX_TEXT: usize = 1024;
const MAX_RUN_TEXT: usize = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    Left,
    Right,
    Middle,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
}

/// Typed arguments only; never converted to shell commands in this slice.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Click {
        x: u32,
        y: u32,
        button: Button,
    },
    Type {
        text: String,
    },
    Scroll {
        direction: Direction,
        notches: u8,
    },
    Key {
        key: String,
        modifiers: Vec<Modifier>,
    },
}
impl Action {
    fn validate(&self, width: u32, height: u32) -> Result<(), Rejection> {
        let valid = match self {
            Self::Click { x, y, .. } => *x < width && *y < height,
            Self::Type { text } => {
                !text.is_empty() && text.len() <= MAX_TEXT && !text.contains('\0')
            }
            Self::Scroll { notches, .. } => (1..=10).contains(notches),
            Self::Key { key, modifiers } => {
                let base = (key.len() == 1
                    && key
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
                    || matches!(
                        key.as_str(),
                        "Enter"
                            | "Escape"
                            | "Tab"
                            | "Backspace"
                            | "Delete"
                            | "Left"
                            | "Right"
                            | "Up"
                            | "Down"
                            | "Home"
                            | "End"
                            | "PageUp"
                            | "PageDown"
                            | "Space"
                    );
                base && modifiers.len() <= 3
                    && modifiers
                        .iter()
                        .enumerate()
                        .all(|(i, m)| !modifiers[..i].contains(m))
            }
        };
        if valid {
            Ok(())
        } else {
            Err(Rejection::Arguments)
        }
    }
    fn text_bytes(&self) -> usize {
        match self {
            Self::Type { text } => text.len(),
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameId(uuid::Uuid);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arguments {
    pub frame_id: FrameId,
    pub action: Action,
}

/// Host identities. No Deserialize, credentials, local preset or resume authority.
#[derive(Clone, PartialEq, Eq)]
pub struct Owner {
    pub principal: String,
    pub run_id: RunId,
    pub machine_id: String,
}
impl Owner {
    fn validate(&self) -> Result<(), Rejection> {
        if valid_label(&self.principal) && crate::instabox::valid_id(&self.machine_id) {
            Ok(())
        } else {
            Err(Rejection::Binding)
        }
    }
}
fn valid_label(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
}

/// Trusted in-process seam ONLY. Future live receipts must originate in successful
/// CU-01 screenshot admission with decoded native geometry and complete accounting.
/// Installing metadata does not capture, decode or disclose any image.
pub struct ObservationMetadata {
    pub owner: Owner,
    pub width: u32,
    pub height: u32,
    pub admitted_at: Instant,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub owner: Owner,
    pub tool_call_id: ToolCallId,
    pub provider_call_id: String,
    pub args: Arguments,
    pub width: u32,
    pub height: u32,
    pub policy: &'static str,
}
impl ApprovalRequest {
    fn validate(&self) -> Result<(), Rejection> {
        self.owner.validate()?;
        if !valid_label(&self.provider_call_id)
            || self.policy != POLICY
            || !valid_geometry(self.width, self.height)
        {
            return Err(Rejection::Binding);
        }
        self.args.action.validate(self.width, self.height)
    }
    /// Content-bearing approval display, never metadata. ASCII-only complete JSON
    /// also escapes non-ASCII control/bidi characters without changing the tuple.
    pub fn description(&self) -> String {
        let json = serde_json::json!({
            "capability":"gui.act", "principal":self.owner.principal,
            "run_id":self.owner.run_id, "tool_call_id":self.tool_call_id,
            "provider_call_id":self.provider_call_id, "machine_id":self.owner.machine_id,
            "arguments":self.args, "native_width":self.width, "native_height":self.height,
            "policy":self.policy, "limits":{"actions":2,"receipts":2,"window_seconds":30,
                "frame_age_seconds":5,"approval_seconds":5,"transport_seconds":3,
                "text_bytes":1024,"run_text_bytes":2048,"scroll_notches":10,"modifiers":3,
                "grant_seconds":30},
            "warning":"Current focus/pointer are not verified. Input can cause irreversible effects. No image disclosure. Any future live transport may auto-wake; cancellation after admission cannot prove termination."
        }).to_string();
        let mut display = String::new();
        for ch in json.chars() {
            if ch.is_ascii() {
                display.push(ch);
            } else {
                use std::fmt::Write;
                for unit in ch.encode_utf16(&mut [0; 2]) {
                    write!(&mut display, "\\u{unit:04x}").expect("String write");
                }
            }
        }
        display
    }
}

/// No Clone/Serialize/Deserialize; one atomic consumption even across threads.
pub struct ActionGrant {
    binding: ApprovalRequest,
    expires: Instant,
    used: AtomicBool,
}
impl ActionGrant {
    pub fn approve(binding: ApprovalRequest, valid_for: Duration) -> Result<Self, Rejection> {
        binding.validate()?;
        if valid_for.is_zero() || valid_for > RUN_WINDOW {
            return Err(Rejection::Expired);
        }
        let expires = Instant::now()
            .checked_add(valid_for)
            .ok_or(Rejection::Expired)?;
        Ok(Self {
            binding,
            expires,
            used: AtomicBool::new(false),
        })
    }
    fn consume(&self, request: &ApprovalRequest, cancel: &Cancellation) -> Result<(), Rejection> {
        if cancel.is_cancelled() {
            return Err(Rejection::Cancelled);
        }
        if self.binding != *request {
            return Err(Rejection::Binding);
        }
        if Instant::now() >= self.expires {
            return Err(Rejection::Expired);
        }
        self.used
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| Rejection::Consumed)
    }
}

#[async_trait]
pub trait ActionApproval: Send + Sync {
    /// None denies. No local presets, exec or observe grants are consulted.
    async fn approve(&self, request: &ApprovalRequest) -> Option<ActionGrant>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportFailure;

#[async_trait]
pub trait ActionTransport: Send + Sync {
    /// Mock seam, no production implementation. Ok acknowledges INPUT only.
    /// Must yield/cooperate with drop, never block or start detached work. A local
    /// timeout drops this future; it cannot prove remote cancellation/termination.
    async fn act(&self, request: &ApprovalRequest) -> Result<(), TransportFailure>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    Binding,
    Arguments,
    StaleFrame,
    Geometry,
    Limit,
    Closed,
    Cancelled,
    Denied,
    ApprovalDeadline,
    Expired,
    Consumed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownReason {
    Cancelled,
    Deadline,
    Transport,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionReport {
    NotDispatched(Rejection),
    /// Observed acknowledgement only, NOT task success or verified UI effect.
    InputAcknowledged,
    Unknown(UnknownReason),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accounting {
    pub admitted_actions: usize,
    pub installed_receipts: usize,
    pub typed_bytes: usize,
    pub closed: bool,
    /// Remains true if an admitted perform future is dropped without an ack.
    pub outcome_unknown: bool,
}

struct Frame {
    id: FrameId,
    width: u32,
    height: u32,
    admitted_at: Instant,
}

/// One exclusive owner per run/target. No serializable history or image storage.
/// The trusted host must retain one controller per run; this is not a global
/// registry or protection against a host constructing duplicate run owners.
pub struct ActionController {
    owner: Owner,
    frame: Option<Frame>,
    first_observation: Option<Instant>,
    accounting: Accounting,
}
fn valid_geometry(width: u32, height: u32) -> bool {
    width > 0
        && height > 0
        && width <= 2560
        && height <= 2560
        && u64::from(width) * u64::from(height) <= 4_096_000
}
impl ActionController {
    pub fn new(owner: Owner) -> Result<Self, Rejection> {
        owner.validate()?;
        Ok(Self {
            owner,
            frame: None,
            first_observation: None,
            accounting: Accounting {
                admitted_actions: 0,
                installed_receipts: 0,
                typed_bytes: 0,
                closed: false,
                outcome_unknown: false,
            },
        })
    }
    pub fn accounting(&self) -> Accounting {
        self.accounting
    }
    pub fn invalidate_geometry(&mut self) {
        self.frame = None;
    }
    pub fn close(&mut self) {
        self.frame = None;
        self.accounting.closed = true;
    }

    /// Bounded host seam, not a publicly registered screenshot tool. At most two
    /// metadata installs, including replacements, without resetting run limits.
    pub fn record_observation(
        &mut self,
        metadata: ObservationMetadata,
        cancel: &Cancellation,
    ) -> Result<FrameId, Rejection> {
        self.frame = None;
        if cancel.is_cancelled() {
            self.close();
            return Err(Rejection::Cancelled);
        }
        if self.accounting.closed {
            return Err(Rejection::Closed);
        }
        if metadata.owner != self.owner {
            return Err(Rejection::Binding);
        }
        if !valid_geometry(metadata.width, metadata.height) {
            return Err(Rejection::Geometry);
        }
        let now = Instant::now();
        if now
            .checked_duration_since(metadata.admitted_at)
            .is_none_or(|age| age >= FRAME_AGE)
        {
            return Err(Rejection::StaleFrame);
        }
        if self.accounting.installed_receipts >= MAX_RECEIPTS
            || self
                .first_observation
                .is_some_and(|first| now.duration_since(first) >= RUN_WINDOW)
        {
            return Err(Rejection::Limit);
        }
        self.first_observation.get_or_insert(metadata.admitted_at);
        let id = FrameId(uuid::Uuid::new_v4());
        self.frame = Some(Frame {
            id,
            width: metadata.width,
            height: metadata.height,
            admitted_at: metadata.admitted_at,
        });
        self.accounting.installed_receipts += 1;
        Ok(id)
    }
    fn check(&self, request: &ApprovalRequest, cancel: &Cancellation) -> Result<(), Rejection> {
        if cancel.is_cancelled() {
            return Err(Rejection::Cancelled);
        }
        if self.accounting.closed {
            return Err(Rejection::Closed);
        }
        request.validate()?;
        if request.owner != self.owner {
            return Err(Rejection::Binding);
        }
        let frame = self.frame.as_ref().ok_or(Rejection::StaleFrame)?;
        if frame.id != request.args.frame_id
            || frame.width != request.width
            || frame.height != request.height
            || Instant::now()
                .checked_duration_since(frame.admitted_at)
                .is_none_or(|age| age >= FRAME_AGE)
        {
            return Err(Rejection::StaleFrame);
        }
        if self.accounting.admitted_actions >= MAX_ACTIONS
            || self.accounting.typed_bytes + request.args.action.text_bytes() > MAX_RUN_TEXT
            || self
                .first_observation
                .is_none_or(|first| Instant::now().duration_since(first) >= RUN_WINDOW)
        {
            return Err(Rejection::Limit);
        }
        Ok(())
    }
    fn reject(&mut self, reason: Rejection) -> ActionReport {
        // Never silently refresh/reuse approvals after failure or denial.
        self.frame = None;
        if reason == Rejection::Cancelled {
            self.close();
        }
        ActionReport::NotDispatched(reason)
    }
    /// Ask and execute one action. No automated loop or runtime/model registration.
    pub async fn perform(
        &mut self,
        tool_call_id: ToolCallId,
        provider_call_id: String,
        args: Arguments,
        broker: Option<&dyn ActionApproval>,
        transport: &dyn ActionTransport,
        cancel: &Cancellation,
    ) -> ActionReport {
        if cancel.is_cancelled() {
            return self.reject(Rejection::Cancelled);
        }
        if self.accounting.closed {
            return self.reject(Rejection::Closed);
        }
        let (width, height) = self.frame.as_ref().map_or((0, 0), |f| (f.width, f.height));
        if self.frame.is_none() {
            return self.reject(Rejection::StaleFrame);
        }
        let request = ApprovalRequest {
            owner: self.owner.clone(),
            tool_call_id,
            provider_call_id,
            args,
            width,
            height,
            policy: POLICY,
        };
        if let Err(e) = self.check(&request, cancel) {
            return self.reject(e);
        }
        let Some(broker) = broker else {
            return self.reject(Rejection::Denied);
        };
        let frame = self.frame.as_ref().expect("checked frame");
        let deadline = (frame.admitted_at + FRAME_AGE)
            .min(self.first_observation.expect("installed receipt") + RUN_WINDOW)
            .min(Instant::now() + FRAME_AGE);
        let grant = tokio::select! {
            biased;
            _ = cancel.cancelled() => return self.reject(Rejection::Cancelled),
            _ = tokio::time::sleep_until(deadline) => return self.reject(Rejection::ApprovalDeadline),
            grant = broker.approve(&request) => grant,
        };
        if cancel.is_cancelled() {
            return self.reject(Rejection::Cancelled);
        }
        let Some(grant) = grant else {
            return self.reject(Rejection::Denied);
        };
        // No await between final state/identity/budget checks, atomic consumption
        // and transport admission. &mut self excludes local replacement races.
        if let Err(e) = self
            .check(&request, cancel)
            .and_then(|_| grant.consume(&request, cancel))
        {
            return self.reject(e);
        }
        self.frame = None;
        self.accounting.admitted_actions += 1;
        self.accounting.typed_bytes += request.args.action.text_bytes();
        // Mark closed before awaiting: dropping perform itself cannot reopen this
        // run after uncertain admission. An observed ack alone can reopen it.
        self.accounting.closed = true;
        self.accounting.outcome_unknown = true;
        let deadline = (Instant::now() + ACTION_WAIT)
            .min(self.first_observation.expect("installed receipt") + RUN_WINDOW);
        tokio::select! {
            biased;
            _ = cancel.cancelled() => ActionReport::Unknown(UnknownReason::Cancelled),
            _ = tokio::time::sleep_until(deadline) => ActionReport::Unknown(UnknownReason::Deadline),
            result = transport.act(&request) => {
                // A mock may cancel/advance time and return in the same poll.
                // Recheck before interpreting an acknowledgement or reopening.
                if cancel.is_cancelled() { ActionReport::Unknown(UnknownReason::Cancelled) }
                else if Instant::now() >= deadline { ActionReport::Unknown(UnknownReason::Deadline) }
                else { match result {
                    Ok(()) => {
                        self.accounting.closed = false;
                        self.accounting.outcome_unknown = false;
                        ActionReport::InputAcknowledged
                    }
                    Err(_) => ActionReport::Unknown(UnknownReason::Transport),
                } }
            },
        }
    }
}

// Protect ordinary debugging from typed content, private targets and identities.
macro_rules! redacted_debug {
    ($($ty:ty),*) => { $(impl fmt::Debug for $ty {
        fn fmt(&self, f:&mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(concat!(stringify!($ty), " [private]")) }
    })* };
}
redacted_debug!(
    Action,
    Arguments,
    Owner,
    ObservationMetadata,
    ApprovalRequest,
    ActionGrant,
    ActionController
);

#[cfg(test)]
#[path = "desktop_tests.rs"]
mod tests;
