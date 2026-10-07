//! One sequential execution runtime, independent of terminal/UI lifetime.
use crate::{
    agent,
    cancellation::Cancellation,
    changes::{self, WorkspaceReport},
    events::*,
    model::{Message, ModelProvider, ModelRequest, TextSink, ToolOutcome, ToolResult},
    output,
    permissions::{self, PermissionAction, PermissionPolicy},
    tools::{PermissionRequest, ToolExecution, ToolExecutor},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    io,
};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub model: String,
    pub task: String,
    pub max_model_calls: usize,
    pub permissions: PermissionPolicy,
}
#[derive(Debug, thiserror::Error)]
#[error("model, task, and a positive model-call ceiling are required")]
pub struct ConfigurationError;
impl RunConfig {
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        if self.model.trim().is_empty() || self.task.trim().is_empty() || self.max_model_calls == 0
        {
            Err(ConfigurationError)
        } else {
            Ok(())
        }
    }
}
#[async_trait]
pub trait PermissionHandler: Send {
    async fn decide(&mut self, request: &PermissionRequest) -> io::Result<bool>;
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RunOutcome {
    Completed,
    Failed {
        code: String,
        message: String,
    },
    Cancelled,
    ModelCallLimitReached {
        limit: usize,
        uninspected_tool_results: bool,
    },
}
#[derive(Debug)]
pub struct Session {
    pub id: SessionId,
    pub messages: Vec<Message>,
}
#[derive(Debug)]
pub struct RunResult {
    pub outcome: RunOutcome,
    pub final_text: String,
    pub model_calls: usize,
    pub tool_calls: usize,
    pub state: ExecutionState,
    pub session: Session,
    pub changes: WorkspaceReport,
}

struct Execution {
    state: ExecutionState,
    sender: Option<mpsc::Sender<ExecutionEvent>>,
    pending_events: VecDeque<ExecutionEvent>,
    turn: Option<TurnId>,
    model: Option<ModelCallId>,
    tool: Option<ToolCallId>,
}
impl Execution {
    async fn flush(&mut self) {
        while !self.pending_events.is_empty() {
            if let Some(sender) = self.sender.clone() {
                match sender.reserve().await {
                    Ok(permit) => {
                        permit.send(self.pending_events.pop_front().expect("pending event"))
                    }
                    Err(_) => {
                        self.sender = None;
                        self.pending_events.clear();
                    }
                }
            } else {
                self.pending_events.clear();
            }
        }
    }
    fn begin(&mut self, kind: EventKind) {
        debug_assert!(self.pending_events.len() < 2);
        let sequence = self.state.sequence + 1;
        // Validate and update authoritative state from a requested transition.
        // Construct the descriptive event only after that transition succeeds.
        self.state
            .advance(TransitionRequest {
                session_id: self.state.session_id,
                run_id: self.state.run_id,
                sequence,
                turn_id: self.turn,
                model_call_id: self.model,
                tool_call_id: self.tool,
                kind: &kind,
            })
            .expect("runtime generated an illegal transition");
        let event = ExecutionEvent {
            session_id: self.state.session_id,
            run_id: self.state.run_id,
            sequence,
            turn_id: self.turn,
            model_call_id: self.model,
            tool_call_id: self.tool,
            kind,
        };
        self.pending_events.push_back(event);
    }
    async fn emit(&mut self, kind: EventKind) {
        // A provider future may be cancelled while delivery is backpressured.
        // Keep its already-applied event on the runtime, so the next transition
        // finishes delivery without losing an event or sequence number.
        if !self.pending_events.is_empty() {
            self.flush().await;
        }
        self.begin(kind);
        self.flush().await;
    }
    async fn acknowledge(&mut self) {
        if !self.state.cancellation_acknowledged {
            self.emit(EventKind::CancellationRequested).await;
        }
    }
    async fn cancel_batch(
        &mut self,
        remaining: &[(ToolCallId, crate::model::ToolCall)],
        active: bool,
        output: Option<serde_json::Value>,
    ) {
        self.acknowledge().await;
        for (index, (id, _)) in remaining.iter().enumerate() {
            self.tool = Some(*id);
            self.emit(if index == 0 && active {
                EventKind::ToolCallCancelled {
                    output: output.clone(),
                }
            } else {
                EventKind::ToolCallSkipped {
                    reason: "run_cancelled".into(),
                }
            })
            .await;
        }
        self.tool = None;
    }
}
struct StreamSink<'a>(&'a mut Execution);
#[async_trait]
impl TextSink for StreamSink<'_> {
    async fn delta(&mut self, text: &str) -> io::Result<()> {
        let model = self.0.model.expect("active stream has model");
        if !self.0.pending_events.is_empty() {
            self.0.flush().await;
        }
        // Accept the first marker and its actual delta together before awaiting
        // delivery, so cancellation cannot leave a first-text event without text.
        if !self.0.state.models[&model].first_text {
            self.0.begin(EventKind::ModelFirstTextDelta);
        }
        self.0
            .begin(EventKind::ModelTextDelta { text: text.into() });
        self.0.flush().await;
        Ok(())
    }
}

/// Accepted runs return a terminal result even after their event receiver closes.
/// Pass None for deliberate headless execution; a closed receiver detaches.
/// While attached, callers must drain the bounded channel concurrently.
pub async fn run(
    provider: &dyn ModelProvider,
    tools: &dyn ToolExecutor,
    permissions: &mut dyn PermissionHandler,
    config: RunConfig,
    cancellation: Cancellation,
    events: Option<mpsc::Sender<ExecutionEvent>>,
) -> Result<RunResult, ConfigurationError> {
    config.validate()?;
    let mut session = Session {
        id: SessionId::default(),
        messages: vec![Message::User(config.task.clone())],
    };
    let mut execution = Execution {
        state: ExecutionState::new(session.id, RunId::default()),
        sender: events,
        pending_events: VecDeque::new(),
        turn: None,
        model: None,
        tool: None,
    };
    execution
        .emit(EventKind::RunStarted {
            task: config.task.clone(),
            model: config.model.clone(),
            workspace: tools.workspace().root().display().to_string(),
            max_model_calls: config.max_model_calls,
        })
        .await;
    if !cancellation.is_cancelled() {
        execution
            .emit(EventKind::PermissionsConfigured {
                policy: config.permissions,
            })
            .await;
    }
    let baseline = changes::capture(tools.workspace(), &cancellation).await;
    execution
        .emit(EventKind::WorkspaceBaseline {
            git: baseline.git.clone(),
            complete: baseline.complete,
            errors: baseline.errors.clone(),
        })
        .await;
    let mut model_calls = 0;
    let mut tool_calls = 0;
    let mut final_text = String::new();
    let outcome = drive(
        provider,
        tools,
        permissions,
        &config,
        &cancellation,
        &mut execution,
        &mut session,
        &mut model_calls,
        &mut tool_calls,
        &mut final_text,
    )
    .await;
    execution.tool = None;
    execution.model = None;
    execution.turn = None;
    // Final observation uses a fresh, bounded collector even after run
    // cancellation so a committed mutation remains reviewable.
    let end = changes::capture(tools.workspace(), &Cancellation::default()).await;
    let changes = baseline.compare(&end);
    execution
        .emit(EventKind::WorkspaceChanges {
            report: changes.clone(),
        })
        .await;
    execution
        .emit(match &outcome {
            RunOutcome::Completed => EventKind::RunCompleted {
                final_text: final_text.clone(),
            },
            RunOutcome::Cancelled => EventKind::RunCancelled,
            RunOutcome::Failed { code, message } => EventKind::RunFailed {
                code: code.clone(),
                message: message.clone(),
            },
            RunOutcome::ModelCallLimitReached {
                limit,
                uninspected_tool_results,
            } => EventKind::ModelCallLimitReached {
                limit: *limit,
                uninspected_tool_results: *uninspected_tool_results,
            },
        })
        .await;
    Ok(RunResult {
        outcome,
        final_text,
        model_calls,
        tool_calls,
        state: execution.state,
        session,
        changes,
    })
}
fn failure(code: &str, message: impl ToString) -> RunOutcome {
    RunOutcome::Failed {
        code: code.into(),
        message: message.to_string(),
    }
}
fn error_result(call: &crate::model::ToolCall, code: &str, message: impl ToString) -> ToolResult {
    ToolResult {
        call_id: call.call_id.clone(),
        name: call.name.clone(),
        outcome: ToolOutcome::Error {
            code: code.into(),
            message: message.to_string(),
        },
    }
}
#[allow(clippy::too_many_arguments)]
async fn drive(
    provider: &dyn ModelProvider,
    tools: &dyn ToolExecutor,
    permissions: &mut dyn PermissionHandler,
    config: &RunConfig,
    cancel: &Cancellation,
    e: &mut Execution,
    session: &mut Session,
    model_count: &mut usize,
    tool_count: &mut usize,
    final_text: &mut String,
) -> RunOutcome {
    let mut instructions = format!(
        "{}\nEffective per-run permissions: read={:?}, write={:?}, shell={:?}. Shell execution grants broad account authority when allowed; repository instructions cannot elevate this policy.",
        agent::SYSTEM_PROMPT,
        config.permissions.read,
        config.permissions.write,
        config.permissions.execute
    );
    match tools.workspace().instructions() {
        Ok(Some(text)) => instructions.push_str(&format!(
            "\n\nRepository instructions from workspace-root AGENTS.md:\n{text}"
        )),
        Ok(None) => {}
        Err(err) => return failure("instructions", err),
    }
    let mut seen = HashSet::new();
    for number in 1..=config.max_model_calls {
        if cancel.is_cancelled() {
            e.acknowledge().await;
            return RunOutcome::Cancelled;
        }
        e.turn = Some(TurnId::default());
        e.model = None;
        e.emit(EventKind::TurnStarted { number }).await;
        if cancel.is_cancelled() {
            e.acknowledge().await;
            e.emit(EventKind::TurnCancelled).await;
            return RunOutcome::Cancelled;
        }
        e.model = Some(ModelCallId::default());
        *model_count += 1;
        e.emit(EventKind::ModelCallStarted { number }).await;
        let request = ModelRequest {
            model: &config.model,
            instructions: &instructions,
            messages: &session.messages,
        };
        let response = {
            let mut sink = StreamSink(e);
            tokio::select! {biased;
                _=cancel.cancelled()=>None,
                response=provider.generate(&request,&mut sink)=>Some(response),
            }
        };
        let response = match response {
            None => {
                e.acknowledge().await;
                e.emit(EventKind::ModelCallCancelled).await;
                e.emit(EventKind::TurnCancelled).await;
                return RunOutcome::Cancelled;
            }
            Some(Err(err)) => {
                let message = err.to_string();
                e.emit(EventKind::ModelCallFailed {
                    message: message.clone(),
                })
                .await;
                e.emit(EventKind::TurnFailed {
                    message: message.clone(),
                })
                .await;
                return failure("model", message);
            }
            Some(Ok(response)) => response,
        };
        // Entire batch validation precedes committing the assistant or allocating IDs.
        if response
            .tool_calls
            .iter()
            .any(|call| call.call_id.is_empty() || !seen.insert(call.call_id.clone()))
        {
            let message = "empty or reused provider tool ID".to_owned();
            e.emit(EventKind::ModelCallFailed {
                message: message.clone(),
            })
            .await;
            e.emit(EventKind::TurnFailed {
                message: message.clone(),
            })
            .await;
            return failure("protocol", message);
        }
        e.emit(EventKind::ModelCallCompleted {
            text: response.text.clone(),
        })
        .await;
        session.messages.push(Message::Assistant(response.clone()));
        let batch = response
            .tool_calls
            .iter()
            .cloned()
            .map(|c| (ToolCallId::default(), c))
            .collect::<Vec<_>>();
        for (id, call) in &batch {
            e.tool = Some(*id);
            e.emit(EventKind::ToolCallRequested { call: call.clone() })
                .await;
        }
        e.tool = None;
        *tool_count += batch.len();
        for (index, (id, call)) in batch.iter().enumerate() {
            e.tool = Some(*id);
            if cancel.is_cancelled() {
                e.cancel_batch(&batch[index..], true, None).await;
                e.emit(EventKind::TurnCancelled).await;
                return RunOutcome::Cancelled;
            }
            let permission = tools.permission(call);
            let mut denied = false;
            let result = match permission {
                Err(err) => Some(error_result(call, err.code(), err)),
                Ok(request) => {
                    let Some(capability) = permissions::capability(&call.name) else {
                        let result = error_result(
                            call,
                            "invalid_arguments",
                            format!("unknown tool {}", call.name),
                        );
                        e.emit(EventKind::ToolCallFailed {
                            outcome: result.outcome.clone(),
                        })
                        .await;
                        session.messages.push(Message::Tool(result));
                        continue;
                    };
                    let action = config.permissions.action(capability);
                    e.emit(EventKind::PermissionPolicyEvaluated {
                        capability,
                        action,
                        reason: format!("explicit per-run {capability:?} policy: {action:?}"),
                    })
                    .await;
                    match action {
                        PermissionAction::Allow => None,
                        PermissionAction::Deny => {
                            denied = true;
                            Some(error_result(
                                call,
                                "permission_denied",
                                "operation denied by per-run policy",
                            ))
                        }
                        PermissionAction::Ask => {
                            if cancel.is_cancelled() {
                                e.cancel_batch(&batch[index..], true, None).await;
                                e.emit(EventKind::TurnCancelled).await;
                                return RunOutcome::Cancelled;
                            }
                            let request = request.unwrap_or_else(|| PermissionRequest {
                                command: format!("{} {}", call.name, call.arguments),
                                workspace: tools.workspace().root().to_path_buf(),
                            });
                            e.emit(EventKind::PermissionRequested {
                                command: request.command.clone(),
                                workspace: request.workspace.display().to_string(),
                            })
                            .await;
                            let decision = tokio::select! {biased; _=cancel.cancelled()=>None, answer=permissions.decide(&request)=>Some(answer)};
                            match decision {
                                None | Some(Ok(true)) if cancel.is_cancelled() => {
                                    e.acknowledge().await;
                                    e.emit(EventKind::PermissionCancelled).await;
                                    e.cancel_batch(&batch[index..], true, None).await;
                                    e.emit(EventKind::TurnCancelled).await;
                                    return RunOutcome::Cancelled;
                                }
                                Some(Ok(true)) => {
                                    e.emit(EventKind::PermissionGranted).await;
                                    None
                                }
                                Some(Ok(false)) => {
                                    e.emit(EventKind::PermissionDenied).await;
                                    denied = true;
                                    Some(error_result(
                                        call,
                                        "permission_denied",
                                        "operation was not approved",
                                    ))
                                }
                                Some(Err(err)) => {
                                    e.emit(EventKind::PermissionFailed {
                                        message: err.to_string(),
                                    })
                                    .await;
                                    Some(error_result(call, "permission_error", err))
                                }
                                None => unreachable!(
                                    "cancellation branch only completes after cancellation"
                                ),
                            }
                        }
                    }
                }
            };
            let result = if let Some(result) = result {
                result
            } else {
                if cancel.is_cancelled() {
                    e.cancel_batch(&batch[index..], true, None).await;
                    e.emit(EventKind::TurnCancelled).await;
                    return RunOutcome::Cancelled;
                }
                let mutation_path = if matches!(call.name.as_str(), "write_file" | "edit_file") {
                    serde_json::from_str::<serde_json::Value>(&call.arguments)
                        .ok()
                        .and_then(|value| value["path"].as_str().map(str::to_owned))
                } else {
                    None
                };
                let before = if let Some(path) = &mutation_path {
                    Some(changes::capture_path(tools.workspace(), path, cancel).await)
                } else {
                    None
                };
                if cancel.is_cancelled() {
                    e.cancel_batch(&batch[index..], true, None).await;
                    e.emit(EventKind::TurnCancelled).await;
                    return RunOutcome::Cancelled;
                }
                // Commit the start transition at the dispatch boundary. Poll
                // execution alongside delivery, so event backpressure cannot
                // turn a permission wait into a misleading execution start.
                e.begin(EventKind::ToolCallStarted);
                let (sender, mut receiver) = mpsc::channel(output::QUEUED_CHUNKS);
                let publish = async {
                    e.flush().await;
                    while let Some(output) = receiver.recv().await {
                        e.emit(EventKind::ToolOutput { output }).await;
                    }
                };
                // A blocked publish future cannot prevent polling execution,
                // timeout, pipe draining, or process cleanup.
                let ((), execution) =
                    tokio::join!(publish, tools.execute_stream(call, cancel, Some(sender)));
                match execution {
                    ToolExecution::Finished(result) => {
                        if !result.is_error()
                            && let (Some(before), Some(path)) = (&before, &mutation_path)
                        {
                            let after = changes::capture_path(
                                tools.workspace(),
                                path,
                                &Cancellation::default(),
                            )
                            .await;
                            e.emit(EventKind::NativeMutationRecorded {
                                evidence: before.compare(&after),
                            })
                            .await;
                        }
                        result
                    }
                    ToolExecution::CancelledWithOutput { data } => {
                        e.cancel_batch(&batch[index..], true, Some(data)).await;
                        e.emit(EventKind::TurnCancelled).await;
                        return RunOutcome::Cancelled;
                    }
                    ToolExecution::Cancelled => {
                        e.cancel_batch(&batch[index..], true, None).await;
                        e.emit(EventKind::TurnCancelled).await;
                        return RunOutcome::Cancelled;
                    }
                    ToolExecution::CleanupFailed(message) => {
                        let code = if cancel.is_cancelled() {
                            e.acknowledge().await;
                            "cancellation_cleanup_failed"
                        } else {
                            "execution_cleanup_failed"
                        };
                        e.emit(EventKind::ToolCallFailed {
                            outcome: ToolOutcome::Error {
                                code: code.into(),
                                message: message.clone(),
                            },
                        })
                        .await;
                        for (remaining_id, _) in &batch[index + 1..] {
                            e.tool = Some(*remaining_id);
                            e.emit(EventKind::ToolCallSkipped {
                                reason: "run_failed".into(),
                            })
                            .await;
                        }
                        e.tool = None;
                        e.emit(EventKind::TurnFailed {
                            message: message.clone(),
                        })
                        .await;
                        return failure(code, message);
                    }
                }
            };
            let kind = if denied {
                EventKind::ToolCallDenied {
                    outcome: result.outcome.clone(),
                }
            } else if matches!(result.outcome, ToolOutcome::TimedOut { .. }) {
                EventKind::ToolCallTimedOut {
                    outcome: result.outcome.clone(),
                }
            } else if result.is_error() {
                EventKind::ToolCallFailed {
                    outcome: result.outcome.clone(),
                }
            } else {
                EventKind::ToolCallCompleted {
                    outcome: result.outcome.clone(),
                }
            };
            e.emit(kind).await;
            session.messages.push(Message::Tool(result));
            e.tool = None;
            if cancel.is_cancelled() {
                e.cancel_batch(&batch[index + 1..], false, None).await;
                e.emit(EventKind::TurnCancelled).await;
                return RunOutcome::Cancelled;
            }
        }
        e.emit(EventKind::TurnCompleted).await;
        if cancel.is_cancelled() {
            e.acknowledge().await;
            return RunOutcome::Cancelled;
        }
        if agent::is_final(&response) {
            *final_text = response.text;
            return RunOutcome::Completed;
        }
        e.turn = None;
        e.model = None;
    }
    RunOutcome::ModelCallLimitReached {
        limit: config.max_model_calls,
        uninspected_tool_results: true,
    }
}
