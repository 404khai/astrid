//! Owned public events. Provider continuation and credentials never appear here.
use crate::{
    changes::{GitState, MutationEvidence, WorkspaceReport},
    model::{ToolCall, ToolOutcome},
    output::ToolOutput,
    permissions::{self, Capability, PermissionAction, PermissionPolicy},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

macro_rules! identity {
    ($($name:ident),*) => {$ (
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(uuid::Uuid);
        impl std::fmt::Display for $name { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { self.0.fmt(f) } }
        impl Default for $name { fn default() -> Self { Self(uuid::Uuid::new_v4()) } }
    )*};
}
identity!(SessionId, RunId, TurnId, ModelCallId, ToolCallId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    Running,
    WaitingPermission,
    PermissionDenied,
    Granted,
    Denied,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    Skipped,
    ModelCallLimitReached,
}
impl Status {
    pub fn terminal(&self) -> bool {
        matches!(
            self,
            Self::Denied
                | Self::Completed
                | Self::Failed
                | Self::Cancelled
                | Self::TimedOut
                | Self::Skipped
                | Self::ModelCallLimitReached
        )
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    ContextSelected {
        selection: crate::context::ContextSelection,
    },
    ContextItemAdded {
        item: crate::context::ContextItem,
    },
    RunStarted {
        task: String,
        model: String,
        workspace: String,
        max_model_calls: usize,
    },
    WorkspaceBaseline {
        git: GitState,
        complete: bool,
        errors: Vec<String>,
    },
    WorkspaceChanges {
        report: WorkspaceReport,
    },
    PermissionsConfigured {
        policy: PermissionPolicy,
    },
    CancellationRequested,
    TurnStarted {
        number: usize,
    },
    ModelCallStarted {
        number: usize,
    },
    ContextPrepared {
        snapshot: crate::context::ContextSnapshot,
    },
    ModelFirstTextDelta,
    ModelTextDelta {
        text: String,
    },
    ModelCallCompleted {
        text: String,
    },
    ModelCallFailed {
        message: String,
    },
    ModelCallCancelled,
    ToolCallRequested {
        call: ToolCall,
    },
    PermissionRequested {
        command: String,
        workspace: String,
    },
    PermissionGranted,
    PermissionDenied,
    PermissionCancelled,
    PermissionFailed {
        message: String,
    },
    PermissionPolicyEvaluated {
        capability: Capability,
        action: PermissionAction,
        reason: String,
    },
    NativeMutationRecorded {
        evidence: MutationEvidence,
    },
    ToolCallStarted,
    ToolOutput {
        output: ToolOutput,
    },
    ToolCallCompleted {
        outcome: ToolOutcome,
    },
    ToolCallFailed {
        outcome: ToolOutcome,
    },
    ToolCallDenied {
        outcome: ToolOutcome,
    },
    ToolCallTimedOut {
        outcome: ToolOutcome,
    },
    ToolCallCancelled {
        output: Option<Value>,
    },
    ToolCallSkipped {
        reason: String,
    },
    TurnCompleted,
    TurnFailed {
        message: String,
    },
    TurnCancelled,
    RunCompleted {
        final_text: String,
    },
    RunFailed {
        code: String,
        message: String,
    },
    RunCancelled,
    ModelCallLimitReached {
        limit: usize,
        uninspected_tool_results: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub sequence: u64,
    pub turn_id: Option<TurnId>,
    pub model_call_id: Option<ModelCallId>,
    pub tool_call_id: Option<ToolCallId>,
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnState {
    pub status: Status,
    pub number: usize,
    pub context_selected: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelState {
    pub status: Status,
    pub turn_id: TurnId,
    pub first_text: bool,
    pub context: Option<crate::context::ContextSnapshot>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolState {
    pub status: Status,
    pub turn_id: TurnId,
    pub model_call_id: ModelCallId,
    pub call: ToolCall,
    pub permission: Option<PermissionAction>,
    pub cancelled_output: Option<Value>,
    pub mutation: Option<MutationEvidence>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionState {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub status: Status,
    pub sequence: u64,
    pub cancellation_acknowledged: bool,
    pub turns: BTreeMap<TurnId, TurnState>,
    pub models: BTreeMap<ModelCallId, ModelState>,
    pub tools: BTreeMap<ToolCallId, ToolState>,
    pub committed_assistant_text: Vec<String>,
    pub permission_policy: Option<PermissionPolicy>,
    pub workspace_baseline: Option<GitState>,
    pub workspace_changes: Option<WorkspaceReport>,
    pub context_items: Vec<crate::context::ContextItem>,
    pub context_selection: Option<crate::context::ContextSelection>,
}
/// Internal transition request; runtime state changes before an event envelope
/// is constructed. Event replay is only a consumer/validation facility.
pub(crate) struct TransitionRequest<'a> {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub sequence: u64,
    pub turn_id: Option<TurnId>,
    pub model_call_id: Option<ModelCallId>,
    pub tool_call_id: Option<ToolCallId>,
    pub kind: &'a EventKind,
}

#[derive(Debug, thiserror::Error)]
#[error("illegal runtime transition: {0}")]
pub struct TransitionError(pub String);
impl ExecutionState {
    pub fn new(session_id: SessionId, run_id: RunId) -> Self {
        Self {
            session_id,
            run_id,
            status: Status::Pending,
            sequence: 0,
            cancellation_acknowledged: false,
            turns: BTreeMap::new(),
            models: BTreeMap::new(),
            tools: BTreeMap::new(),
            committed_assistant_text: Vec::new(),
            permission_policy: None,
            workspace_baseline: None,
            workspace_changes: None,
            context_items: Vec::new(),
            context_selection: None,
        }
    }
    /// Validate on a copy so rejected transitions cannot partially change state.
    pub fn transition(&mut self, event: &ExecutionEvent) -> Result<(), TransitionError> {
        self.advance(TransitionRequest {
            session_id: event.session_id,
            run_id: event.run_id,
            sequence: event.sequence,
            turn_id: event.turn_id,
            model_call_id: event.model_call_id,
            tool_call_id: event.tool_call_id,
            kind: &event.kind,
        })
    }
    pub(crate) fn advance(
        &mut self,
        request: TransitionRequest<'_>,
    ) -> Result<(), TransitionError> {
        let mut next = self.clone();
        next.apply(&request)?;
        *self = next;
        Ok(())
    }
    fn apply(&mut self, e: &TransitionRequest<'_>) -> Result<(), TransitionError> {
        let require = |ok: bool| {
            if ok {
                Ok(())
            } else {
                Err(TransitionError(format!("{:?}", e.kind)))
            }
        };
        require(
            e.session_id == self.session_id
                && e.run_id == self.run_id
                && e.sequence == self.sequence + 1
                && !self.status.terminal(),
        )?;
        if let Some(id) = e.turn_id {
            require(
                matches!(e.kind, EventKind::TurnStarted { .. }) || self.turns.contains_key(&id),
            )?;
        }
        if let Some(id) = e.model_call_id
            && !matches!(e.kind, EventKind::ModelCallStarted { .. })
        {
            let m = self
                .models
                .get(&id)
                .ok_or_else(|| TransitionError("unknown model".into()))?;
            require(Some(m.turn_id) == e.turn_id)?;
        }
        if let Some(id) = e.tool_call_id
            && !matches!(e.kind, EventKind::ToolCallRequested { .. })
        {
            let t = self
                .tools
                .get(&id)
                .ok_or_else(|| TransitionError("unknown tool".into()))?;
            require(Some(t.turn_id) == e.turn_id && Some(t.model_call_id) == e.model_call_id)?;
        }
        match &e.kind {
            EventKind::ContextItemAdded { item } => {
                use crate::context::ContextOrigin;
                require(item.id.0 == self.context_items.len())?;
                require(!item.added_reason.is_empty())?;
                let is_message = matches!(
                    item.origin,
                    ContextOrigin::UserTask
                        | ContextOrigin::Assistant { .. }
                        | ContextOrigin::ToolResult { .. }
                );
                let next_message = self
                    .context_items
                    .iter()
                    .filter(|item| item.message_index.is_some())
                    .count();
                require(item.message_index == is_message.then_some(next_message))?;
                require(
                    !self
                        .context_items
                        .iter()
                        .any(|existing| existing.origin == item.origin),
                )?;
                match &item.origin {
                    ContextOrigin::OperatingInstructions
                    | ContextOrigin::RepositoryInstructions { .. }
                    | ContextOrigin::UserTask => {
                        require(
                            e.turn_id.is_none()
                                && e.model_call_id.is_none()
                                && e.tool_call_id.is_none()
                                && self.turns.is_empty(),
                        )?;
                    }
                    ContextOrigin::Assistant { model_call_id } => {
                        require(
                            e.model_call_id == Some(*model_call_id) && e.tool_call_id.is_none(),
                        )?;
                        require(self.models[model_call_id].status == Status::Completed)?;
                    }
                    ContextOrigin::ToolResult {
                        model_call_id,
                        tool_call_id,
                        requested_path,
                        path_truncated,
                    } => {
                        require(
                            e.model_call_id == Some(*model_call_id)
                                && e.tool_call_id == Some(*tool_call_id),
                        )?;
                        require(matches!(
                            self.tools[tool_call_id].status,
                            Status::Completed | Status::Failed | Status::Denied | Status::TimedOut
                        ))?;
                        require(
                            crate::context::requested_path(&self.tools[tool_call_id].call)
                                == (requested_path.clone(), *path_truncated),
                        )?;
                    }
                }
            }
            EventKind::RunStarted { .. }
            | EventKind::PermissionsConfigured { .. }
            | EventKind::WorkspaceBaseline { .. }
            | EventKind::WorkspaceChanges { .. }
            | EventKind::RunCompleted { .. }
            | EventKind::RunFailed { .. }
            | EventKind::RunCancelled
            | EventKind::ModelCallLimitReached { .. } => require(
                e.turn_id.is_none() && e.model_call_id.is_none() && e.tool_call_id.is_none(),
            )?,
            EventKind::TurnStarted { .. } | EventKind::ContextSelected { .. } => require(
                e.turn_id.is_some() && e.model_call_id.is_none() && e.tool_call_id.is_none(),
            )?,
            EventKind::TurnCompleted | EventKind::TurnFailed { .. } | EventKind::TurnCancelled => {
                require(e.turn_id.is_some() && e.tool_call_id.is_none())?
            }
            EventKind::ModelCallStarted { .. }
            | EventKind::ContextPrepared { .. }
            | EventKind::ModelFirstTextDelta
            | EventKind::ModelTextDelta { .. }
            | EventKind::ModelCallCompleted { .. }
            | EventKind::ModelCallFailed { .. }
            | EventKind::ModelCallCancelled => require(
                e.turn_id.is_some() && e.model_call_id.is_some() && e.tool_call_id.is_none(),
            )?,
            EventKind::CancellationRequested => {}
            _ => require(
                e.turn_id.is_some() && e.model_call_id.is_some() && e.tool_call_id.is_some(),
            )?,
        }
        if !matches!(e.kind, EventKind::RunStarted { .. }) {
            require(self.status == Status::Running)?;
        }
        let turn = || {
            e.turn_id
                .ok_or_else(|| TransitionError("missing turn ID".into()))
        };
        let model = || {
            e.model_call_id
                .ok_or_else(|| TransitionError("missing model ID".into()))
        };
        let tool = || {
            e.tool_call_id
                .ok_or_else(|| TransitionError("missing tool ID".into()))
        };
        match &e.kind {
            EventKind::ContextSelected { selection } => {
                require(!self.turns[&turn()?].context_selected)?;
                require(
                    self.turns[&turn()?].status == Status::Running
                        && !self.cancellation_acknowledged,
                )?;
                require(
                    !self
                        .models
                        .values()
                        .any(|model| model.turn_id == turn().unwrap()),
                )?;
                require(selection.budget.admits(&selection.snapshot))?;
                require(selection.decisions.len() == self.context_items.len())?;
                require(selection.decisions.iter().zip(&self.context_items).all(
                    |(decision, item)| decision.item_id == item.id && !decision.reason.is_empty(),
                ))?;
                let mut exchange_retention = BTreeMap::new();
                require(selection.decisions.iter().zip(&self.context_items).all(
                    |(decision, item)| {
                        let model = match item.origin {
                            crate::context::ContextOrigin::Assistant { model_call_id }
                            | crate::context::ContextOrigin::ToolResult { model_call_id, .. } => {
                                Some(model_call_id)
                            }
                            _ => None,
                        };
                        model.is_none_or(|model| {
                            exchange_retention
                                .insert(model, decision.retained)
                                .is_none_or(|previous| previous == decision.retained)
                        })
                    },
                ))?;
                let latest = self.context_items.iter().rev().find_map(|item| {
                    if let crate::context::ContextOrigin::Assistant { model_call_id } = item.origin
                    {
                        Some(model_call_id)
                    } else {
                        None
                    }
                });
                require(selection.decisions.iter().zip(&self.context_items).all(
                    |(decision, item)| {
                        let protected = match item.origin {
                            crate::context::ContextOrigin::OperatingInstructions
                            | crate::context::ContextOrigin::RepositoryInstructions { .. }
                            | crate::context::ContextOrigin::UserTask => true,
                            crate::context::ContextOrigin::Assistant { model_call_id }
                            | crate::context::ContextOrigin::ToolResult { model_call_id, .. } => {
                                Some(model_call_id) == latest
                            }
                        };
                        !protected || decision.retained
                    },
                ))?;
                if let Some(summary) = &selection.summary {
                    let omitted = selection
                        .decisions
                        .iter()
                        .filter(|decision| !decision.retained)
                        .map(|decision| decision.item_id)
                        .collect::<Vec<_>>();
                    require(
                        summary.incomplete
                            && !omitted.is_empty()
                            && summary.source_items == omitted
                            && summary.text_bytes == summary.text.len()
                            && summary.text_bytes <= selection.budget.max_summary_bytes,
                    )?;
                }
                self.turns
                    .get_mut(&turn()?)
                    .expect("known turn")
                    .context_selected = true;
                self.context_selection = Some(selection.clone());
            }
            EventKind::ContextItemAdded { item } => {
                self.context_items.push(item.clone());
            }
            EventKind::RunStarted { .. } => {
                require(self.status == Status::Pending && e.turn_id.is_none())?;
                self.status = Status::Running;
            }
            EventKind::WorkspaceBaseline { git, .. } => {
                require(self.turns.is_empty() && self.workspace_baseline.is_none())?;
                self.workspace_baseline = Some(git.clone());
            }
            EventKind::WorkspaceChanges { report } => {
                require(
                    self.workspace_changes.is_none()
                        && self.turns.values().all(|turn| turn.status.terminal()),
                )?;
                self.workspace_changes = Some(report.clone());
            }
            EventKind::PermissionsConfigured { policy } => {
                require(
                    self.turns.is_empty()
                        && self.permission_policy.is_none()
                        && !self.cancellation_acknowledged,
                )?;
                self.permission_policy = Some(*policy);
            }
            EventKind::CancellationRequested => {
                require(!self.cancellation_acknowledged)?;
                self.cancellation_acknowledged = true;
            }
            EventKind::TurnStarted { number } => {
                require(
                    self.status == Status::Running
                        && self.permission_policy.is_some()
                        && !self.cancellation_acknowledged
                        && self.turns.values().all(|t| t.status.terminal())
                        && !self.turns.contains_key(&turn()?),
                )?;
                require(*number == self.turns.len() + 1)?;
                self.turns.insert(
                    turn()?,
                    TurnState {
                        status: Status::Running,
                        number: *number,
                        context_selected: false,
                    },
                );
            }
            EventKind::ModelCallStarted { number } => {
                require(
                    !self.cancellation_acknowledged
                        && self.turns[&turn()?].status == Status::Running
                        && !self.models.contains_key(&model()?)
                        && !self.models.values().any(|m| m.turn_id == turn().unwrap()),
                )?;
                require(*number == self.turns[&turn()?].number)?;
                self.models.insert(
                    model()?,
                    ModelState {
                        status: Status::Running,
                        turn_id: turn()?,
                        first_text: false,
                        context: None,
                    },
                );
            }
            EventKind::ContextPrepared { snapshot } => {
                if self.turns[&turn()?].context_selected {
                    require(
                        self.context_selection
                            .as_ref()
                            .is_some_and(|selection| &selection.snapshot == snapshot),
                    )?;
                }
                let m = self.models.get_mut(&model()?).expect("known model");
                require(
                    m.status == Status::Running
                        && !m.first_text
                        && m.context.is_none()
                        && !self.cancellation_acknowledged,
                )?;
                m.context = Some(snapshot.clone());
            }
            EventKind::ModelFirstTextDelta
            | EventKind::ModelTextDelta { .. }
            | EventKind::ModelCallCompleted { .. }
            | EventKind::ModelCallFailed { .. }
            | EventKind::ModelCallCancelled => {
                let m = self
                    .models
                    .get_mut(&model()?)
                    .ok_or_else(|| TransitionError("unknown model".into()))?;
                require(m.status == Status::Running)?;
                match &e.kind {
                    EventKind::ModelFirstTextDelta => {
                        require(!m.first_text)?;
                        m.first_text = true;
                    }
                    EventKind::ModelTextDelta { .. } => {
                        require(m.first_text)?;
                    }
                    EventKind::ModelCallCompleted { text } => {
                        m.status = Status::Completed;
                        self.committed_assistant_text.push(text.clone());
                    }
                    EventKind::ModelCallFailed { .. } => m.status = Status::Failed,
                    EventKind::ModelCallCancelled => {
                        require(self.cancellation_acknowledged)?;
                        m.status = Status::Cancelled;
                    }
                    _ => unreachable!(),
                }
            }
            EventKind::ToolCallRequested { call } => {
                require(
                    self.models[&model()?].status == Status::Completed
                        && !self.tools.contains_key(&tool()?)
                        && self.turns[&turn()?].status == Status::Running,
                )?;
                self.tools.insert(
                    tool()?,
                    ToolState {
                        status: Status::Pending,
                        turn_id: turn()?,
                        model_call_id: model()?,
                        call: call.clone(),
                        permission: None,
                        cancelled_output: None,
                        mutation: None,
                    },
                );
            }
            EventKind::PermissionPolicyEvaluated { .. }
            | EventKind::ToolOutput { .. }
            | EventKind::NativeMutationRecorded { .. }
            | EventKind::PermissionRequested { .. }
            | EventKind::PermissionGranted
            | EventKind::PermissionDenied
            | EventKind::PermissionCancelled
            | EventKind::PermissionFailed { .. }
            | EventKind::ToolCallStarted
            | EventKind::ToolCallCompleted { .. }
            | EventKind::ToolCallFailed { .. }
            | EventKind::ToolCallDenied { .. }
            | EventKind::ToolCallTimedOut { .. }
            | EventKind::ToolCallCancelled { .. }
            | EventKind::ToolCallSkipped { .. } => {
                let id = tool()?;
                if matches!(e.kind, EventKind::ToolCallStarted) {
                    require(
                        !self.cancellation_acknowledged
                            && self
                                .tools
                                .iter()
                                .all(|(other, t)| *other == id || t.status != Status::Running),
                    )?;
                }
                let t = self
                    .tools
                    .get_mut(&id)
                    .ok_or_else(|| TransitionError("unknown tool".into()))?;
                require(!t.status.terminal())?;
                t.status = match &e.kind {
                    EventKind::PermissionPolicyEvaluated {
                        capability, action, ..
                    } => {
                        require(
                            t.status == Status::Pending
                                && t.permission.is_none()
                                && !self.cancellation_acknowledged,
                        )?;
                        require(
                            permissions::capability(&t.call.name) == Some(*capability)
                                && self
                                    .permission_policy
                                    .is_some_and(|policy| policy.action(*capability) == *action),
                        )?;
                        t.permission = Some(*action);
                        if *action == PermissionAction::Deny {
                            Status::PermissionDenied
                        } else {
                            Status::Pending
                        }
                    }
                    EventKind::NativeMutationRecorded { evidence } => {
                        require(t.status == Status::Running && t.mutation.is_none())?;
                        t.mutation = Some(evidence.clone());
                        Status::Running
                    }
                    EventKind::ToolOutput { .. } => {
                        require(t.status == Status::Running)?;
                        Status::Running
                    }
                    EventKind::PermissionRequested { .. } => {
                        require(
                            t.status == Status::Pending
                                && t.permission == Some(PermissionAction::Ask)
                                && !self.cancellation_acknowledged,
                        )?;
                        Status::WaitingPermission
                    }
                    EventKind::PermissionGranted => {
                        require(
                            t.status == Status::WaitingPermission
                                && !self.cancellation_acknowledged,
                        )?;
                        Status::Granted
                    }
                    EventKind::PermissionDenied => {
                        require(t.status == Status::WaitingPermission)?;
                        Status::PermissionDenied
                    }
                    EventKind::PermissionCancelled | EventKind::PermissionFailed { .. } => {
                        require(t.status == Status::WaitingPermission)?;
                        Status::Pending
                    }
                    EventKind::ToolCallStarted => {
                        require(
                            (t.status == Status::Pending
                                && t.permission == Some(PermissionAction::Allow))
                                || (t.status == Status::Granted
                                    && t.permission == Some(PermissionAction::Ask)),
                        )?;
                        Status::Running
                    }
                    EventKind::ToolCallCompleted { .. } => {
                        require(t.status == Status::Running)?;
                        Status::Completed
                    }
                    EventKind::ToolCallTimedOut { .. } => {
                        require(t.status == Status::Running)?;
                        Status::TimedOut
                    }
                    EventKind::ToolCallFailed { .. } => {
                        require(matches!(t.status, Status::Pending | Status::Running))?;
                        Status::Failed
                    }
                    EventKind::ToolCallDenied { .. } => {
                        require(t.status == Status::PermissionDenied)?;
                        Status::Denied
                    }
                    EventKind::ToolCallCancelled { output } => {
                        require(self.cancellation_acknowledged)?;
                        t.cancelled_output = output.clone();
                        Status::Cancelled
                    }
                    EventKind::ToolCallSkipped { reason } => {
                        require(
                            t.status == Status::Pending
                                && ((reason == "run_cancelled" && self.cancellation_acknowledged)
                                    || reason == "run_failed"),
                        )?;
                        Status::Skipped
                    }
                    _ => unreachable!(),
                };
            }
            EventKind::TurnCompleted | EventKind::TurnFailed { .. } | EventKind::TurnCancelled => {
                let id = turn()?;
                if matches!(e.kind, EventKind::TurnCancelled) {
                    require(self.cancellation_acknowledged)?;
                }
                if matches!(e.kind, EventKind::TurnCompleted) {
                    require(
                        self.models.values().filter(|m| m.turn_id == id).count() == 1
                            && self
                                .models
                                .values()
                                .filter(|m| m.turn_id == id)
                                .all(|m| m.status == Status::Completed),
                    )?;
                }
                require(
                    self.turns[&id].status == Status::Running
                        && self
                            .models
                            .values()
                            .filter(|m| m.turn_id == id)
                            .all(|m| m.status.terminal())
                        && self
                            .tools
                            .values()
                            .filter(|t| t.turn_id == id)
                            .all(|t| t.status.terminal()),
                )?;
                self.turns.get_mut(&id).unwrap().status = match e.kind {
                    EventKind::TurnCompleted => Status::Completed,
                    EventKind::TurnCancelled => Status::Cancelled,
                    _ => Status::Failed,
                };
            }
            EventKind::RunCompleted { .. }
            | EventKind::RunFailed { .. }
            | EventKind::RunCancelled
            | EventKind::ModelCallLimitReached { .. } => {
                require(
                    self.status == Status::Running
                        && self.turns.values().all(|t| t.status.terminal())
                        && self.tools.values().all(|t| t.status.terminal()),
                )?;
                self.status = match e.kind {
                    EventKind::RunCompleted { .. } => {
                        require(
                            !self.cancellation_acknowledged
                                && !self.turns.is_empty()
                                && self.turns.values().all(|t| t.status == Status::Completed),
                        )?;
                        Status::Completed
                    }
                    EventKind::RunCancelled => {
                        require(self.cancellation_acknowledged)?;
                        Status::Cancelled
                    }
                    EventKind::ModelCallLimitReached { .. } => Status::ModelCallLimitReached,
                    _ => Status::Failed,
                };
            }
        }
        self.sequence = e.sequence;
        Ok(())
    }
}
