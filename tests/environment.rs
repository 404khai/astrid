mod support;
use astrid::{
    cancellation::Cancellation,
    events::{EventKind, ExecutionEvent, ExecutionState, Status},
    model::{
        Message, ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink, ToolCall,
        ToolOutcome, ToolResult,
    },
    openai::completed_response,
    output::{CAPTURE_BYTES, CHUNK_BYTES, OutputStream},
    permissions::{PermissionAction, PermissionPolicy},
    runtime::{self, PermissionHandler, RunConfig, RunOutcome, RunResult},
    tools::{PermissionRequest, ToolError, ToolExecution, ToolExecutor, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::json;
use std::{
    collections::VecDeque,
    fs, io,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use support::{Confirmation, call, item};
use tokio::sync::mpsc;

struct Script(Mutex<VecDeque<ModelResponse>>);
impl Script {
    fn new(calls: Vec<ToolCall>) -> Self {
        Self(Mutex::new([
            completed_response(json!({"status":"completed","output":calls.iter().map(item).collect::<Vec<_>>()})).unwrap(),
            completed_response(json!({"status":"completed","output":[support::message("done")]})).unwrap(),
        ].into()))
    }
}
#[async_trait]
impl ModelProvider for Script {
    async fn generate(
        &self,
        _: &ModelRequest<'_>,
        _: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected model call"))
    }
}
fn config(policy: PermissionPolicy) -> RunConfig {
    RunConfig {
        model: "fixture".into(),
        task: "environment acceptance".into(),
        max_model_calls: 5,
        permissions: policy,
    }
}
fn allow() -> PermissionPolicy {
    PermissionPolicy {
        read: PermissionAction::Allow,
        write: PermissionAction::Allow,
        execute: PermissionAction::Allow,
    }
}
fn tools(root: &std::path::Path) -> Tools {
    Tools::new(Workspace::new(root).unwrap(), Duration::from_secs(5)).unwrap()
}
fn replay(result: &RunResult, events: &[ExecutionEvent]) {
    let mut state = ExecutionState::new(result.session.id, result.state.run_id);
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.sequence, index as u64 + 1);
        state.transition(event).unwrap();
    }
    assert_eq!(state, result.state);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(
                e.kind,
                EventKind::RunCompleted { .. }
                    | EventKind::RunCancelled
                    | EventKind::RunFailed { .. }
                    | EventKind::ModelCallLimitReached { .. }
            ))
            .count(),
        1
    );
}
async fn recorded(
    provider: &dyn ModelProvider,
    executor: &dyn ToolExecutor,
    handler: &mut dyn PermissionHandler,
    policy: PermissionPolicy,
    cancel: Cancellation,
) -> (RunResult, Vec<ExecutionEvent>) {
    let (tx, mut rx) = mpsc::channel::<ExecutionEvent>(1);
    let receive = async {
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }
        events
    };
    let (result, events) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            runtime::run(
                provider,
                executor,
                handler,
                config(policy),
                cancel,
                Some(tx)
            ),
            receive
        )
    })
    .await
    .expect("bounded runtime fixture");
    let result = result.unwrap();
    replay(&result, &events);
    (result, events)
}
async fn wait_for(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("fixture condition became ready");
}
// Failure-path fallback: terminate the fixture's own process group if a test
// panics before the runtime can cancel and reap it.
struct ProcessGuard(PathBuf);
impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if let Ok(text) = fs::read_to_string(&self.0)
            && let Ok(pid) = text.trim().parse::<i32>()
        {
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        }
    }
}

#[tokio::test]
async fn shell_cleanup_precedes_resuming_a_stalled_event_consumer() {
    let root = tempfile::tempdir().unwrap();
    let _guard = ProcessGuard(root.path().join("leader"));
    let command = "echo $$ > leader; printf partial-out; printf partial-err >&2; i=0; while [ \"$i\" -lt 6000 ]; do printf 0123456789012345678901234567890123456789012345678901234567890123; printf 9876543210987654321098765432109876543210987654321098765432109876 >&2; i=$((i+1)); done; (sleep 1; touch too_late) & echo $! > descendant; touch noisy_ready; wait";
    let provider = Script::new(vec![call("shell", "shell", json!({"command":command}))]);
    let executor = tools(root.path());
    let cancel = Cancellation::default();
    let mut handler = Confirmation::new(true);
    let (tx, mut rx) = mpsc::channel::<ExecutionEvent>(1);
    let consumer = async {
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            let started = matches!(event.kind, EventKind::ToolCallStarted);
            events.push(event);
            if started {
                break;
            }
        }
        // From here until cleanup verification the public queue is not drained.
        wait_for(|| root.path().join("noisy_ready").exists()).await;
        let leader: i32 = fs::read_to_string(root.path().join("leader"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(unsafe { libc::kill(leader, 0) }, 0);
        cancel.cancel();
        wait_for(|| unsafe { libc::kill(leader, 0) } == -1).await;
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
        // The descendant's deadline elapses before we resume event delivery.
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert!(!root.path().join("too_late").exists());
        while let Some(event) = rx.recv().await {
            events.push(event);
        }
        events
    };
    let (result, events) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            runtime::run(
                &provider,
                &executor,
                &mut handler,
                config(allow()),
                cancel.clone(),
                Some(tx)
            ),
            consumer
        )
    })
    .await
    .unwrap();
    let result = result.unwrap();
    replay(&result, &events);
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert!(
        !result
            .session
            .messages
            .iter()
            .any(|message| matches!(message, Message::Tool(_)))
    );
    let terminal = events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::ToolCallCancelled { output: Some(data) } => Some(data),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(terminal.len(), 1);
    let data = terminal[0];
    assert!(data["stdout"].as_str().unwrap().starts_with("partial-out"));
    assert!(data["stderr"].as_str().unwrap().starts_with("partial-err"));
    for stream in ["stdout", "stderr"] {
        assert!(data["output"][stream]["captured_bytes"].as_u64().unwrap() <= CAPTURE_BYTES as u64);
        assert!(
            data["output"][stream]["capture_omitted_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert!(
            data["output"][stream]["live_omitted_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(data["output"][stream]["complete"], false);
    }
    assert!(
        events
            .iter()
            .filter_map(|e| if let EventKind::ToolOutput { output } = &e.kind {
                Some(output)
            } else {
                None
            })
            .all(|output| output.bytes.len() <= CHUNK_BYTES)
    );
}

#[tokio::test]
async fn shell_output_handshake_proves_visibility_before_exit_and_preserves_stream_order() {
    let root = tempfile::tempdir().unwrap();
    let _guard = ProcessGuard(root.path().join("leader"));
    let provider = Script::new(vec![call(
        "shell",
        "shell",
        json!({"command":"echo $$ > leader; printf stdout-ready; printf stderr-ready >&2; while [ ! -f release ]; do sleep 0.01; done; printf stdout-after; printf stderr-after >&2; touch exited"}),
    )]);
    let executor = tools(root.path());
    let mut handler = Confirmation::new(true);
    let (tx, mut rx) = mpsc::channel::<ExecutionEvent>(1);
    let consumer = async {
        let mut events = Vec::new();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut released = false;
        while let Some(event) = rx.recv().await {
            if let EventKind::ToolOutput { output } = &event.kind {
                match output.stream {
                    OutputStream::Stdout => out.extend_from_slice(&output.bytes),
                    OutputStream::Stderr => err.extend_from_slice(&output.bytes),
                }
                if !released && out == b"stdout-ready" && err == b"stderr-ready" {
                    assert!(!root.path().join("exited").exists());
                    fs::write(root.path().join("release"), "").unwrap();
                    released = true;
                }
            }
            events.push(event);
        }
        assert!(released);
        assert_eq!(out, b"stdout-readystdout-after");
        assert_eq!(err, b"stderr-readystderr-after");
        events
    };
    let (result, events) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            runtime::run(
                &provider,
                &executor,
                &mut handler,
                config(allow()),
                Cancellation::default(),
                Some(tx)
            ),
            consumer
        )
    })
    .await
    .unwrap();
    let result = result.unwrap();
    replay(&result, &events);
    assert_eq!(result.outcome, RunOutcome::Completed);
    let data = events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::ToolCallCompleted {
                outcome: ToolOutcome::Success { data },
            } => Some(data),
            _ => None,
        })
        .unwrap();
    assert_eq!(data["stdout"], "stdout-readystdout-after");
    assert_eq!(data["stderr"], "stderr-readystderr-after");
    assert_eq!(data["output"]["stdout"]["live_omitted_bytes"], 0);
    assert_eq!(data["output"]["stderr"]["live_omitted_bytes"], 0);
    assert!(root.path().join("exited").exists());
}

struct Spy {
    workspace: Workspace,
    executed: AtomicUsize,
}
#[async_trait]
impl ToolExecutor for Spy {
    fn workspace(&self) -> &Workspace {
        &self.workspace
    }
    fn permission(&self, _: &ToolCall) -> Result<Option<PermissionRequest>, ToolError> {
        Ok(None)
    }
    async fn execute(&self, call: &ToolCall, _: &Cancellation) -> ToolExecution {
        self.executed.fetch_add(1, Ordering::SeqCst);
        ToolExecution::Finished(ToolResult {
            call_id: call.call_id.clone(),
            name: call.name.clone(),
            outcome: ToolOutcome::Success { data: json!({}) },
        })
    }
}
#[tokio::test]
async fn permission_matrix_enforces_read_write_and_execute_before_dispatch() {
    for name in ["read_file", "write_file", "shell"] {
        for action in [
            PermissionAction::Allow,
            PermissionAction::Ask,
            PermissionAction::Deny,
        ] {
            for approved in [false, true] {
                let root = tempfile::tempdir().unwrap();
                let executor = Spy {
                    workspace: Workspace::new(root.path()).unwrap(),
                    executed: AtomicUsize::new(0),
                };
                let provider = Script::new(vec![call("policy", name, json!({}))]);
                let mut handler = Confirmation::new(approved);
                let policy = PermissionPolicy {
                    read: action,
                    write: action,
                    execute: action,
                };
                let (result, events) = recorded(
                    &provider,
                    &executor,
                    &mut handler,
                    policy,
                    Cancellation::default(),
                )
                .await;
                assert_eq!(result.outcome, RunOutcome::Completed);
                let expected = action == PermissionAction::Allow
                    || (action == PermissionAction::Ask && approved);
                assert_eq!(
                    executor.executed.load(Ordering::SeqCst),
                    usize::from(expected),
                    "{name} {action:?} {approved}"
                );
                assert_eq!(
                    handler.commands.len(),
                    usize::from(action == PermissionAction::Ask)
                );
                assert_eq!(
                    events
                        .iter()
                        .filter(|e| matches!(e.kind, EventKind::PermissionPolicyEvaluated { .. }))
                        .count(),
                    1
                );
                assert_eq!(
                    events
                        .iter()
                        .filter(|e| matches!(e.kind, EventKind::ToolCallStarted))
                        .count(),
                    usize::from(expected)
                );
                if !expected {
                    assert!(
                        events
                            .iter()
                            .any(|e| matches!(e.kind, EventKind::ToolCallDenied { .. }))
                    );
                }
            }
        }
    }
}
struct CancelApproval(Cancellation);
#[async_trait]
impl PermissionHandler for CancelApproval {
    async fn decide(&mut self, _: &PermissionRequest) -> io::Result<bool> {
        self.0.cancel();
        Ok(true)
    }
}
#[tokio::test]
async fn cancelling_when_approval_returns_prevents_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let executor = Spy {
        workspace: Workspace::new(root.path()).unwrap(),
        executed: AtomicUsize::new(0),
    };
    let cancel = Cancellation::default();
    let mut handler = CancelApproval(cancel.clone());
    let provider = Script::new(vec![call("approve", "shell", json!({}))]);
    let (result, events) = recorded(
        &provider,
        &executor,
        &mut handler,
        PermissionPolicy::default(),
        cancel,
    )
    .await;
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert_eq!(executor.executed.load(Ordering::SeqCst), 0);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event.kind, EventKind::ToolCallStarted))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, EventKind::PermissionCancelled))
    );
}
struct CancelAfterEdit(Tools);
#[async_trait]
impl ToolExecutor for CancelAfterEdit {
    fn workspace(&self) -> &Workspace {
        self.0.workspace()
    }
    fn permission(&self, call: &ToolCall) -> Result<Option<PermissionRequest>, ToolError> {
        self.0.permission(call)
    }
    async fn execute(&self, call: &ToolCall, cancel: &Cancellation) -> ToolExecution {
        let result = self.0.execute(call, cancel).await;
        cancel.cancel();
        result
    }
}
#[tokio::test]
async fn committed_native_edit_is_retained_and_reported_when_the_run_is_cancelled() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("target"), "user-prefix\nbroken\n").unwrap();
    fs::write(root.path().join("user-work"), "existing user changes").unwrap();
    let provider = Script::new(vec![
        call(
            "edit",
            "edit_file",
            json!({"path":"target","old_text":"broken","new_text":"fixed"}),
        ),
        call(
            "later",
            "write_file",
            json!({"path":"later","content":"no","overwrite":false}),
        ),
    ]);
    let executor = CancelAfterEdit(tools(root.path()));
    let (result, events) = recorded(
        &provider,
        &executor,
        &mut Confirmation::new(false),
        allow(),
        Cancellation::default(),
    )
    .await;
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert_eq!(
        fs::read_to_string(root.path().join("target")).unwrap(),
        "user-prefix\nfixed\n"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("user-work")).unwrap(),
        "existing user changes"
    );
    assert!(!root.path().join("later").exists());
    assert_eq!(
        result
            .session
            .messages
            .iter()
            .filter(|m| matches!(m, Message::Tool(_)))
            .count(),
        1
    );
    assert_eq!(
        result
            .state
            .tools
            .values()
            .filter(|t| t.status == Status::Completed)
            .count(),
        1
    );
    let mutation = events
        .iter()
        .find_map(|event| {
            if let EventKind::NativeMutationRecorded { evidence } = &event.kind {
                Some((event.tool_call_id, evidence))
            } else {
                None
            }
        })
        .unwrap();
    assert!(mutation.0.is_some());
    assert_eq!(mutation.1.path, "utf8:target");
    let native_change = mutation.1.change.as_ref().unwrap();
    assert!(native_change.patch.as_ref().unwrap().contains("fixed"));
    assert!(
        result
            .changes
            .changes
            .iter()
            .any(|change| change.path == "utf8:target")
    );
    assert!(
        !result
            .changes
            .changes
            .iter()
            .any(|change| change.path == "utf8:user-work")
    );
    let final_evidence = events
        .iter()
        .position(|event| matches!(event.kind, EventKind::WorkspaceChanges { .. }))
        .unwrap();
    let terminal = events
        .iter()
        .position(|event| matches!(event.kind, EventKind::RunCancelled))
        .unwrap();
    assert!(final_evidence < terminal);
}
