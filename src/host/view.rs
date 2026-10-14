//! Allowlisted visible projections. Never serialize executable sessions/provider replay.
use crate::{events::*, model::Message, permissions::PermissionPolicy};
use serde::{Deserialize, Serialize};

pub const TEXT_BYTES: usize = 2048;
pub const PAGE_BYTES: usize = 16 * 1024;
pub const RING_BYTES: usize = 256 * 1024;
#[derive(Debug, Clone, Serialize)]
pub struct Text {
    pub value: String,
    pub omitted_bytes: usize,
}
impl Text {
    pub fn new(value: &str) -> Self {
        let mut end = value.len().min(TEXT_BYTES);
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        Self {
            value: value[..end].into(),
            omitted_bytes: value.len() - end,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Activity {
    pub ordinal: u64,
    pub session_id: SessionId,
    pub run_id: RunId,
    pub sequence: u64,
    pub turn_id: Option<TurnId>,
    pub model_call_id: Option<ModelCallId>,
    pub tool_call_id: Option<ToolCallId>,
    pub kind: String,
    pub text: Text,
}
impl Activity {
    pub fn project(e: &ExecutionEvent, ordinal: u64) -> Self {
        use EventKind::*;
        let (kind, text) = match &e.kind {
            RunStarted { task, .. } => ("run_started", task.clone()),
            PermissionsConfigured { policy } => (
                "permissions_configured",
                format!(
                    "read={:?} write={:?} execute={:?}",
                    policy.read, policy.write, policy.execute
                ),
            ),
            TurnStarted { number } => ("turn_started", format!("Turn {number}")),
            ModelCallStarted { number } => ("model_call_started", format!("Model call {number}")),
            ModelTextDelta { text } => ("model_text_delta", text.clone()),
            ModelCallCompleted { text } => ("model_call_completed", text.clone()),
            ToolCallRequested { call } => (
                "tool_call_requested",
                format!(
                    "{} {}",
                    call.name.chars().take(128).collect::<String>(),
                    call.arguments
                ),
            ),
            ToolCallStarted => ("tool_call_started", String::new()),
            ToolCallCompleted { outcome } => (
                "tool_call_completed",
                serde_json::to_string(outcome).unwrap_or_default(),
            ),
            ToolCallFailed { outcome } => (
                "tool_call_failed",
                serde_json::to_string(outcome).unwrap_or_default(),
            ),
            ToolCallDenied { outcome } => (
                "tool_call_denied",
                serde_json::to_string(outcome).unwrap_or_default(),
            ),
            ToolCallTimedOut { outcome } => (
                "tool_call_timed_out",
                serde_json::to_string(outcome).unwrap_or_default(),
            ),
            ToolCallCancelled { .. } => ("tool_call_cancelled", String::new()),
            ToolCallSkipped { reason } => ("tool_call_skipped", reason.clone()),
            ToolOutput { output } => (
                "tool_output",
                format!(
                    "{:?}: {}",
                    output.stream,
                    String::from_utf8_lossy(&output.bytes)
                ),
            ),
            PermissionRequested { command, .. } => ("permission_requested", command.clone()),
            PermissionGranted => ("permission_granted", String::new()),
            PermissionDenied => ("permission_denied", String::new()),
            PermissionCancelled => ("permission_cancelled", String::new()),
            PermissionFailed { .. } => ("permission_failed", "Permission handler failed".into()),
            CancellationRequested => ("cancellation_requested", String::new()),
            RunCompleted { .. } => (
                "run_completed",
                "Run completed; task success requires supporting evidence".into(),
            ),
            RunCancelled => ("run_cancelled", String::new()),
            RunFailed { code, .. } => (
                "run_failed",
                format!(
                    "Run failed ({}) — check provider/sign-in and session validity",
                    code.chars().take(128).collect::<String>()
                ),
            ),
            ModelCallFailed { .. } => (
                "model_call_failed",
                "Model request failed; check provider/sign-in".into(),
            ),
            ModelCallCancelled => ("model_call_cancelled", String::new()),
            ModelCallLimitReached { limit, .. } => (
                "model_call_limit_reached",
                format!("Model-call limit {limit}; final tool results may be uninspected"),
            ),
            ContextPrepared { snapshot } => (
                "context_prepared",
                format!("Context estimate: {:?}", snapshot),
            ),
            ContextSelected { .. } => (
                "context_selected",
                "Context selected; original history is retained privately".into(),
            ),
            WorkspaceChanges { report } => (
                "workspace_changes",
                format!(
                    "{} observed changes; complete={}; attribution is not established",
                    report.changes.len(),
                    report.complete
                ),
            ),
            WorkspaceBaseline { complete, .. } => (
                "workspace_baseline",
                format!("Workspace evidence complete={complete}"),
            ),
            _ => ("runtime_activity", String::new()),
        };
        Self {
            ordinal,
            session_id: e.session_id,
            run_id: e.run_id,
            sequence: e.sequence,
            turn_id: e.turn_id,
            model_call_id: e.model_call_id,
            tool_call_id: e.tool_call_id,
            kind: kind.into(),
            text: Text::new(&text),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct MessageView {
    pub index: usize,
    pub role: &'static str,
    pub text: Text,
}
impl MessageView {
    pub fn project(index: usize, message: &Message) -> Self {
        let (role, text) = match message {
            Message::User(text) => ("user", text.clone()),
            Message::Summary(text) => ("summary", text.clone()),
            Message::Assistant(response) => ("assistant", response.text.clone()),
            Message::Tool(result) => (
                "tool",
                format!(
                    "{} {}",
                    result.name,
                    serde_json::to_string(&result.outcome).unwrap_or_default()
                ),
            ),
        };
        Self {
            index,
            role,
            text: Text::new(&text),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct PermissionView {
    pub id: uuid::Uuid,
    pub run_id: RunId,
    pub tool_call_id: ToolCallId,
    pub command: Text,
    pub workspace: Text,
    pub can_approve: bool,
    pub authority: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct RunView {
    pub request_id: uuid::Uuid,
    pub run_id: Option<RunId>,
    pub session_id: SessionId,
    pub status: String,
    pub cancellation_requested: bool,
    pub permission: Option<PermissionView>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SessionView {
    pub id: SessionId,
    pub label: String,
    pub runs: usize,
    pub messages: usize,
    pub outcome: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Workflow {
    Notes,
    CodingAsk,
    CodingAuto,
    CodingUnbound,
}
impl Workflow {
    pub fn policy(self) -> PermissionPolicy {
        use crate::permissions::PermissionMode;
        match self {
            Self::Notes => PermissionPolicy::read_only(),
            Self::CodingAsk => PermissionMode::Ask.policy(),
            Self::CodingAuto => PermissionMode::Auto.policy(),
            Self::CodingUnbound => PermissionMode::Unbound.policy(),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub version: u32,
    pub workspace: Option<Text>,
    pub model: String,
    pub provider: &'static str,
    pub workflow: Workflow,
    pub policy: PermissionPolicy,
    pub selected_session: Option<SessionId>,
    pub sessions: Vec<SessionView>,
    pub run: Option<RunView>,
    pub save_error: Option<String>,
    pub quitting: bool,
}
#[derive(Debug, Serialize)]
pub struct HistoryPage {
    pub version: u32,
    pub messages: Vec<MessageView>,
    pub next: Option<usize>,
}
#[derive(Debug, Serialize)]
pub struct Batch {
    pub version: u32,
    pub batch_id: u64,
    pub events: Vec<Activity>,
    pub coverage_gap: bool,
}
