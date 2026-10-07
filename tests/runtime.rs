mod support;
use astrid::{
    auth::{AuthError, Authentication, BearerToken},
    cancellation::Cancellation,
    events::*,
    model::{
        Message, ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink, ToolCall,
        ToolOutcome,
    },
    openai::{OpenAiProvider, completed_response},
    runtime::{self, PermissionHandler, RunConfig, RunOutcome, RunResult},
    tools::{PermissionRequest, ToolError, ToolExecution, ToolExecutor, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::json;
use std::{
    collections::{BTreeMap, VecDeque},
    fs, io,
    sync::{Arc, Mutex},
    time::Duration,
};
use support::{Confirmation, call, item};
use tokio::sync::{Notify, mpsc};

fn config(limit: usize) -> RunConfig {
    RunConfig {
        context_budget: None,
        model: "test-model".into(),
        task: "task".into(),
        max_model_calls: limit,
        permissions: Default::default(),
    }
}
fn tools(root: &std::path::Path) -> Tools {
    Tools::new(Workspace::new(root).unwrap(), Duration::from_secs(30)).unwrap()
}
fn response(calls: Vec<ToolCall>, text: &str) -> ModelResponse {
    let mut output = calls.iter().map(item).collect::<Vec<_>>();
    if !text.is_empty() {
        output.push(support::message(text));
    }
    completed_response(json!({"status":"completed","output":output})).unwrap()
}
struct Script(Mutex<VecDeque<ModelResponse>>);
impl Script {
    fn new(responses: Vec<ModelResponse>) -> Self {
        Self(Mutex::new(responses.into()))
    }
}
#[async_trait]
impl ModelProvider for Script {
    async fn generate(
        &self,
        _: &ModelRequest<'_>,
        sink: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        let response = self
            .0
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected model call");
        if !response.text.is_empty() {
            sink.delta(&response.text).await?;
        }
        Ok(response)
    }
}
async fn recorded(
    provider: &dyn ModelProvider,
    tools: &dyn ToolExecutor,
    permissions: &mut dyn PermissionHandler,
    cancel: Cancellation,
    limit: usize,
) -> (RunResult, Vec<ExecutionEvent>) {
    let (sender, mut receiver) = mpsc::channel(2);
    let consumer = async {
        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event)
        }
        events
    };
    let (result, events) = tokio::join!(
        runtime::run(
            provider,
            tools,
            permissions,
            config(limit),
            cancel,
            Some(sender)
        ),
        consumer
    );
    (result.unwrap(), events)
}
fn check_history(result: &RunResult, events: &[ExecutionEvent]) {
    assert!(!events.is_empty());
    let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
    let mut tool_terminal_counts = BTreeMap::new();
    let mut requested = Vec::new();
    let mut committed = Vec::new();
    let mut run_terminals = 0;
    let mut tool_projection = BTreeMap::new();
    let mut turn_projection = BTreeMap::new();
    let mut model_projection = BTreeMap::new();
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.sequence, index as u64 + 1);
        let serialized = serde_json::to_string(event).unwrap();
        let decoded: ExecutionEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(*event, decoded);
        replay.transition(&decoded).unwrap();
        // Independent consumer projection uses event semantics, not the runtime
        // transition implementation, to reconstruct terminal execution states.
        let terminal = match event.kind {
            EventKind::ToolCallCompleted { .. }
            | EventKind::TurnCompleted
            | EventKind::ModelCallCompleted { .. } => Some(Status::Completed),
            EventKind::ToolCallFailed { .. }
            | EventKind::TurnFailed { .. }
            | EventKind::ModelCallFailed { .. } => Some(Status::Failed),
            EventKind::ToolCallCancelled { .. }
            | EventKind::TurnCancelled
            | EventKind::ModelCallCancelled => Some(Status::Cancelled),
            EventKind::ToolCallDenied { .. } => Some(Status::Denied),
            EventKind::ToolCallTimedOut { .. } => Some(Status::TimedOut),
            EventKind::ToolCallSkipped { .. } => Some(Status::Skipped),
            _ => None,
        };
        if let Some(status) = terminal {
            if let Some(id) = event.tool_call_id {
                tool_projection.insert(id, status);
            } else if matches!(
                event.kind,
                EventKind::TurnCompleted | EventKind::TurnFailed { .. } | EventKind::TurnCancelled
            ) {
                turn_projection.insert(event.turn_id.unwrap(), status);
            } else {
                model_projection.insert(event.model_call_id.unwrap(), status);
            }
        }

        if event.tool_call_id.is_some() {
            assert!(event.turn_id.is_some() && event.model_call_id.is_some());
        }
        match &event.kind {
            EventKind::ToolCallRequested { call } => {
                assert_ne!(
                    serde_json::to_value(event.tool_call_id.unwrap()).unwrap(),
                    json!(call.call_id)
                );
                requested.push(event.tool_call_id.unwrap());
            }
            EventKind::ToolCallCompleted { .. }
            | EventKind::ToolCallFailed { .. }
            | EventKind::ToolCallDenied { .. }
            | EventKind::ToolCallTimedOut { .. }
            | EventKind::ToolCallCancelled { .. }
            | EventKind::ToolCallSkipped { .. } => {
                *tool_terminal_counts
                    .entry(event.tool_call_id.unwrap())
                    .or_insert(0) += 1;
            }
            EventKind::ModelCallCompleted { text } => committed.push(text.clone()),
            EventKind::RunCompleted { .. }
            | EventKind::RunFailed { .. }
            | EventKind::RunCancelled
            | EventKind::ModelCallLimitReached { .. } => run_terminals += 1,
            _ => {}
        }
    }
    assert_eq!(replay, result.state);
    assert_eq!(
        tool_projection,
        result
            .state
            .tools
            .iter()
            .map(|(id, t)| (*id, t.status.clone()))
            .collect()
    );
    assert_eq!(
        turn_projection,
        result
            .state
            .turns
            .iter()
            .map(|(id, t)| (*id, t.status.clone()))
            .collect()
    );
    assert_eq!(
        model_projection,
        result
            .state
            .models
            .iter()
            .map(|(id, t)| (*id, t.status.clone()))
            .collect()
    );

    assert_eq!(run_terminals, 1);
    assert_eq!(requested.len(), tool_terminal_counts.len());
    assert!(requested.iter().all(|id| tool_terminal_counts[id] == 1));
    let accepted = result
        .session
        .messages
        .iter()
        .filter_map(|m| {
            if let Message::Assistant(r) = m {
                Some(r.text.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(accepted, committed);
}

#[tokio::test]
async fn batch_requests_precede_execution_and_permissions_are_not_execution() {
    let root = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![
        response(
            vec![
                call(
                    "write",
                    "write_file",
                    json!({"path":"created","content":"hello","overwrite":false}),
                ),
                call("shell", "shell", json!({"command":"exit 101"})),
                call("read", "read_file", json!({"path":"created"})),
            ],
            "working",
        ),
        response(vec![], "done"),
    ]);
    let (result, events) = recorded(
        &provider,
        &tools(root.path()),
        &mut Confirmation::new(true),
        Cancellation::default(),
        20,
    )
    .await;
    assert_eq!(result.outcome, RunOutcome::Completed);
    check_history(&result, &events);
    let first_start = events
        .iter()
        .position(|e| matches!(e.kind, EventKind::ToolCallStarted))
        .unwrap();
    assert_eq!(
        events[..first_start]
            .iter()
            .filter(|e| matches!(e.kind, EventKind::ToolCallRequested { .. }))
            .count(),
        3
    );
    let permission = events
        .iter()
        .position(|e| matches!(e.kind, EventKind::PermissionRequested { .. }))
        .unwrap();
    assert!(matches!(
        events[permission + 1].kind,
        EventKind::PermissionGranted
    ));
    assert!(matches!(
        events[permission + 2].kind,
        EventKind::ToolCallStarted
    ));
    assert!(events.iter().any(|e|matches!(&e.kind,EventKind::ToolCallCompleted{outcome:ToolOutcome::Success{data}} if data["exit_code"]==101)));
    assert_eq!(result.state.turns.len(), 2);
    assert_eq!(result.state.models.len(), 2);
}

#[tokio::test]
async fn denial_has_no_start_and_is_returned_to_the_model() {
    let root = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![
        response(
            vec![call("shell", "shell", json!({"command":"touch forbidden"}))],
            "",
        ),
        response(vec![], "denied"),
    ]);
    let (result, events) = recorded(
        &provider,
        &tools(root.path()),
        &mut Confirmation::new(false),
        Cancellation::default(),
        20,
    )
    .await;
    check_history(&result, &events);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ToolCallStarted))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ToolCallDenied { .. }))
    );
    assert!(
        result
            .session
            .messages
            .iter()
            .any(|m| matches!(m,Message::Tool(r) if r.is_error()))
    );
    assert!(!root.path().join("forbidden").exists());
}

struct Streaming {
    entered: Arc<Notify>,
    fail: bool,
}
#[async_trait]
impl ModelProvider for Streaming {
    async fn generate(
        &self,
        _: &ModelRequest<'_>,
        sink: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        sink.delta("provisional").await?;
        self.entered.notify_one();
        if self.fail {
            Err(ModelError::Protocol("interrupted".into()))
        } else {
            std::future::pending().await
        }
    }
}
#[tokio::test]
async fn cancelled_and_interrupted_streams_never_commit_text_or_accept_tools() {
    for fail in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let entered = Arc::new(Notify::new());
        let provider = Streaming {
            entered: entered.clone(),
            fail,
        };
        let cancel = Cancellation::default();
        let stop = async {
            entered.notified().await;
            if !fail {
                cancel.cancel();
            }
        };
        let ((result, events), ()) = tokio::join!(
            async {
                recorded(
                    &provider,
                    &tools(root.path()),
                    &mut Confirmation::new(false),
                    cancel.clone(),
                    20,
                )
                .await
            },
            stop
        );
        check_history(&result, &events);
        assert_eq!(result.session.messages.len(), 1);
        assert!(result.state.tools.is_empty());
        assert!(
            events
                .iter()
                .any(|e| matches!(&e.kind,EventKind::ModelTextDelta{text} if text=="provisional"))
        );
        assert!(result.state.committed_assistant_text.is_empty());
        assert_eq!(
            result.state.status,
            if fail {
                Status::Failed
            } else {
                Status::Cancelled
            }
        );
    }
}
struct PendingPermission(Arc<Notify>);
#[async_trait]
impl PermissionHandler for PendingPermission {
    async fn decide(&mut self, _: &PermissionRequest) -> io::Result<bool> {
        self.0.notify_one();
        std::future::pending().await
    }
}
#[tokio::test]
async fn cancellation_during_permission_cancels_current_tool_and_skips_rest() {
    let root = tempfile::tempdir().unwrap();
    let entered = Arc::new(Notify::new());
    let provider = Script::new(vec![response(
        vec![
            call("shell", "shell", json!({"command":"touch forbidden"})),
            call(
                "write",
                "write_file",
                json!({"path":"later","content":"no","overwrite":false}),
            ),
        ],
        "",
    )]);
    let cancel = Cancellation::default();
    let mut permission = PendingPermission(entered.clone());
    let stop = async {
        entered.notified().await;
        cancel.cancel();
    };
    let ((result, events), ()) = tokio::join!(
        async {
            recorded(
                &provider,
                &tools(root.path()),
                &mut permission,
                cancel.clone(),
                20,
            )
            .await
        },
        stop
    );
    check_history(&result, &events);
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, EventKind::PermissionCancelled))
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ToolCallStarted))
    );
    assert_eq!(result.session.messages.len(), 2);
    assert!(!root.path().join("later").exists());
    assert_eq!(
        result
            .state
            .tools
            .values()
            .filter(|t| t.status == Status::Skipped)
            .count(),
        1
    );
}
struct CancelAfterExecution {
    inner: Tools,
    cleanup_failure: bool,
}
#[async_trait]
impl ToolExecutor for CancelAfterExecution {
    fn workspace(&self) -> &Workspace {
        self.inner.workspace()
    }
    fn permission(&self, call: &ToolCall) -> Result<Option<PermissionRequest>, ToolError> {
        self.inner.permission(call)
    }
    async fn execute(&self, call: &ToolCall, cancel: &Cancellation) -> ToolExecution {
        if self.cleanup_failure {
            cancel.cancel();
            return ToolExecution::CleanupFailed("injected process cleanup failure".into());
        }
        let outcome = self.inner.execute(call, cancel).await;
        cancel.cancel();
        outcome
    }
}
#[tokio::test]
async fn committed_mutation_remains_completed_and_later_tools_are_skipped() {
    let root = tempfile::tempdir().unwrap();
    let tools = CancelAfterExecution {
        inner: tools(root.path()),
        cleanup_failure: false,
    };
    let provider = Script::new(vec![response(
        vec![
            call(
                "first",
                "write_file",
                json!({"path":"first","content":"committed","overwrite":false}),
            ),
            call(
                "later",
                "write_file",
                json!({"path":"later","content":"no","overwrite":false}),
            ),
        ],
        "",
    )]);
    let (result, events) = recorded(
        &provider,
        &tools,
        &mut Confirmation::new(false),
        Cancellation::default(),
        20,
    )
    .await;
    check_history(&result, &events);
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert_eq!(
        fs::read_to_string(root.path().join("first")).unwrap(),
        "committed"
    );
    assert!(!root.path().join("later").exists());
    assert_eq!(
        result
            .state
            .tools
            .values()
            .filter(|t| t.status == Status::Completed)
            .count(),
        1
    );
    assert_eq!(
        result
            .state
            .tools
            .values()
            .filter(|t| t.status == Status::Skipped)
            .count(),
        1
    );
    assert_eq!(
        result
            .session
            .messages
            .iter()
            .filter(|m| matches!(m, Message::Tool(_)))
            .count(),
        1
    );
    let ack = events
        .iter()
        .position(|e| matches!(e.kind, EventKind::CancellationRequested))
        .unwrap();
    assert!(!events[ack..].iter().any(|e| matches!(
        e.kind,
        EventKind::ToolCallStarted | EventKind::ModelCallStarted { .. }
    )));
}
#[tokio::test]
async fn cleanup_failure_is_run_failure_with_cancellation_history() {
    let root = tempfile::tempdir().unwrap();
    let tools = CancelAfterExecution {
        inner: tools(root.path()),
        cleanup_failure: true,
    };
    let provider = Script::new(vec![response(
        vec![
            call("first", "read_file", json!({"path":"file"})),
            call("later", "read_file", json!({"path":"file"})),
        ],
        "",
    )]);
    let (result, events) = recorded(
        &provider,
        &tools,
        &mut Confirmation::new(false),
        Cancellation::default(),
        20,
    )
    .await;
    check_history(&result, &events);
    assert!(
        matches!(result.outcome,RunOutcome::Failed{ref code,..} if code=="cancellation_cleanup_failed")
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, EventKind::CancellationRequested))
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::RunCancelled))
    );
}
#[tokio::test]
async fn shell_cancellation_kills_group_and_prevents_delayed_mutation() {
    let root = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![response(
        vec![call(
            "shell",
            "shell",
            json!({"command":"echo $$ > leader; (sleep 1; echo escaped > escaped) & echo $! > child; wait"}),
        )],
        "",
    )]);
    let cancel = Cancellation::default();
    let stop = async {
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if root.path().join("child").exists() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        cancel.cancel();
    };
    let ((result, events), ()) = tokio::join!(
        async {
            recorded(
                &provider,
                &tools(root.path()),
                &mut Confirmation::new(true),
                cancel.clone(),
                20,
            )
            .await
        },
        stop
    );
    check_history(&result, &events);
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    let leader: i32 = fs::read_to_string(root.path().join("leader"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(leader, 0) }, -1);
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert!(!root.path().join("escaped").exists());
}
#[tokio::test]
async fn timeout_is_distinct_from_cancellation_and_results_are_recoverable() {
    let root = tempfile::tempdir().unwrap();
    let tools = Tools::new(
        Workspace::new(root.path()).unwrap(),
        Duration::from_millis(30),
    )
    .unwrap();
    let provider = Script::new(vec![
        response(
            vec![call("shell", "shell", json!({"command":"sleep 30"}))],
            "",
        ),
        response(vec![], "timed out"),
    ]);
    let (result, events) = recorded(
        &provider,
        &tools,
        &mut Confirmation::new(true),
        Cancellation::default(),
        20,
    )
    .await;
    check_history(&result, &events);
    assert_eq!(result.outcome, RunOutcome::Completed);
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ToolCallTimedOut { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::CancellationRequested))
    );
}
#[tokio::test]
async fn headless_and_disconnected_consumers_do_not_cancel_execution() {
    for disconnected in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let provider = Script::new(vec![response(vec![], "done")]);
        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);
        let result = runtime::run(
            &provider,
            &tools(root.path()),
            &mut Confirmation::new(false),
            config(20),
            Cancellation::default(),
            if disconnected { Some(sender) } else { None },
        )
        .await
        .unwrap();
        assert_eq!(result.outcome, RunOutcome::Completed);
        assert_eq!(result.final_text, "done");
    }
}
#[tokio::test]
async fn attached_channel_applies_backpressure_and_detaches_when_closed() {
    let root = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![response(vec![], "done")]);
    let (sender, mut receiver) = mpsc::channel(1);
    let mut permission = Confirmation::new(false);
    let tools = tools(root.path());
    let execution = runtime::run(
        &provider,
        &tools,
        &mut permission,
        config(20),
        Cancellation::default(),
        Some(sender),
    );
    tokio::pin!(execution);
    // Poll until full. It cannot complete while the attached receiver is undrained.
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut execution)
            .await
            .is_err()
    );
    assert!(matches!(
        receiver.recv().await.unwrap().kind,
        EventKind::RunStarted { .. }
    ));
    drop(receiver);
    assert_eq!(execution.await.unwrap().outcome, RunOutcome::Completed);
}
#[tokio::test]
async fn ceiling_executes_final_batch_and_exposes_uninspected_results() {
    let root = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![response(
        vec![call(
            "write",
            "write_file",
            json!({"path":"created","content":"ok","overwrite":false}),
        )],
        "",
    )]);
    let (result, events) = recorded(
        &provider,
        &tools(root.path()),
        &mut Confirmation::new(false),
        Cancellation::default(),
        1,
    )
    .await;
    check_history(&result, &events);
    assert_eq!(
        result.outcome,
        RunOutcome::ModelCallLimitReached {
            limit: 1,
            uninspected_tool_results: true
        }
    );
    assert!(root.path().join("created").exists());
}
struct WaitingAuth(Arc<Notify>);

struct FailingAuth(std::sync::atomic::AtomicUsize);
#[async_trait]
impl Authentication for FailingAuth {
    async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(AuthError::Invalid("test authentication failure".into()))
    }
}

struct AnnouncePreparation {
    provider: OpenAiProvider,
    entered: Arc<Notify>,
}
struct PreparationSink<'a> {
    sink: &'a mut dyn TextSink,
    entered: Arc<Notify>,
}
#[async_trait]
impl TextSink for PreparationSink<'_> {
    async fn request_prepared(
        &mut self,
        snapshot: astrid::context::ContextSnapshot,
    ) -> io::Result<()> {
        self.entered.notify_one();
        self.sink.request_prepared(snapshot).await
    }
    async fn delta(&mut self, text: &str) -> io::Result<()> {
        self.sink.delta(text).await
    }
}
#[async_trait]
impl ModelProvider for AnnouncePreparation {
    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        sink: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        self.provider
            .generate(
                request,
                &mut PreparationSink {
                    sink,
                    entered: self.entered.clone(),
                },
            )
            .await
    }
}

#[tokio::test]
async fn cancelled_stalled_context_publication_never_enters_authentication() {
    let root = tempfile::tempdir().unwrap();
    let auth = Arc::new(FailingAuth(std::sync::atomic::AtomicUsize::new(0)));
    let entered = Arc::new(Notify::new());
    let provider = AnnouncePreparation {
        provider: OpenAiProvider::new(auth.clone()).unwrap(),
        entered: entered.clone(),
    };
    let tools = tools(root.path());
    let mut permission = Confirmation::new(false);
    let cancel = Cancellation::default();
    let (sender, mut receiver) = mpsc::channel(1);
    let execution = runtime::run(
        &provider,
        &tools,
        &mut permission,
        config(2),
        cancel.clone(),
        Some(sender),
    );
    tokio::pin!(execution);
    let mut history = Vec::new();
    loop {
        tokio::select! {
            result = &mut execution => panic!("unexpected completion: {result:?}"),
            event = receiver.recv() => history.push(event.unwrap()),
        }
        if matches!(history.last().unwrap().kind, EventKind::TurnStarted { .. }) {
            break;
        }
    }
    // ModelCallStarted fills the single slot; ContextPrepared waits for delivery.
    tokio::select! {
        biased;
        result = &mut execution => panic!("unexpected completion: {result:?}"),
        () = entered.notified() => {},
    }
    assert_eq!(auth.0.load(std::sync::atomic::Ordering::SeqCst), 0);
    cancel.cancel();
    let drain = async {
        while let Some(event) = receiver.recv().await {
            history.push(event);
        }
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(execution, drain)
    })
    .await
    .unwrap();
    let result = result.unwrap();
    check_history(&result, &history);
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert_eq!(auth.0.load(std::sync::atomic::Ordering::SeqCst), 0);
    let prepared = history
        .iter()
        .position(|e| matches!(e.kind, EventKind::ContextPrepared { .. }))
        .unwrap();
    let acknowledged = history
        .iter()
        .position(|e| matches!(e.kind, EventKind::CancellationRequested))
        .unwrap();
    assert!(prepared < acknowledged);
    assert_eq!(
        history
            .iter()
            .filter(|e| matches!(e.kind, EventKind::ModelCallCancelled))
            .count(),
        1
    );
}

#[tokio::test]
async fn prepared_context_remains_visible_when_authentication_fails() {
    let root = tempfile::tempdir().unwrap();
    let auth = Arc::new(FailingAuth(std::sync::atomic::AtomicUsize::new(0)));
    let provider = OpenAiProvider::new(auth.clone()).unwrap();
    let (result, history) = recorded(
        &provider,
        &tools(root.path()),
        &mut Confirmation::new(false),
        Cancellation::default(),
        2,
    )
    .await;
    check_history(&result, &history);
    assert!(matches!(result.outcome, RunOutcome::Failed { .. }));
    assert_eq!(auth.0.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(
        history
            .iter()
            .any(|e| matches!(e.kind, EventKind::ContextPrepared { .. }))
    );
    assert!(
        !history
            .iter()
            .any(|e| matches!(e.kind, EventKind::ModelTextDelta { .. }))
    );
}
#[async_trait]
impl Authentication for WaitingAuth {
    async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
        self.0.notify_one();
        std::future::pending().await
    }
}
#[tokio::test]
async fn cancellation_interrupts_authentication_waiting() {
    let root = tempfile::tempdir().unwrap();
    let entered = Arc::new(Notify::new());
    let provider = OpenAiProvider::new(Arc::new(WaitingAuth(entered.clone()))).unwrap();
    let cancel = Cancellation::default();
    let stop = async {
        entered.notified().await;
        cancel.cancel();
    };
    let ((result, events), ()) = tokio::join!(
        async {
            recorded(
                &provider,
                &tools(root.path()),
                &mut Confirmation::new(false),
                cancel.clone(),
                20,
            )
            .await
        },
        stop
    );
    check_history(&result, &events);
    assert_eq!(result.outcome, RunOutcome::Cancelled);
}
#[tokio::test]
async fn pre_cancelled_run_has_one_terminal_outcome_without_model_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let cancel = Cancellation::default();
    cancel.cancel();
    let (result, events) = recorded(
        &Script::new(vec![]),
        &tools(root.path()),
        &mut Confirmation::new(false),
        cancel,
        20,
    )
    .await;
    check_history(&result, &events);
    assert_eq!(result.model_calls, 0);
    assert_eq!(result.outcome, RunOutcome::Cancelled);
}
#[tokio::test]
async fn illegal_transitions_and_duplicate_terminal_events_leave_state_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![response(vec![], "done")]);
    let (result, events) = recorded(
        &provider,
        &tools(root.path()),
        &mut Confirmation::new(false),
        Cancellation::default(),
        20,
    )
    .await;
    let mut state = ExecutionState::new(result.session.id, result.state.run_id);
    assert!(state.transition(&events[1]).is_err());
    assert_eq!(state.sequence, 0);
    for event in &events {
        state.transition(event).unwrap();
    }
    let before = state.clone();
    let mut duplicate = events.last().unwrap().clone();
    duplicate.sequence += 1;
    assert!(state.transition(&duplicate).is_err());
    assert_eq!(state, before);
}

#[tokio::test]
async fn cancellation_during_backpressured_text_keeps_sequence_lossless() {
    for stop_after_model_start in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let provider = Streaming {
            entered: Arc::new(Notify::new()),
            fail: false,
        };
        let tools = tools(root.path());
        let mut permission = Confirmation::new(false);
        let cancel = Cancellation::default();
        let (sender, mut receiver) = mpsc::channel(1);
        let execution = runtime::run(
            &provider,
            &tools,
            &mut permission,
            config(20),
            cancel.clone(),
            Some(sender),
        );
        tokio::pin!(execution);
        let mut history = Vec::new();
        loop {
            tokio::select! {result=&mut execution=>panic!("unexpected completion: {result:?}"),event=receiver.recv()=>history.push(event.unwrap())}
            if matches!(
                history.last().unwrap().kind,
                EventKind::ModelCallStarted { .. }
            ) || (!stop_after_model_start
                && matches!(history.last().unwrap().kind, EventKind::TurnStarted { .. }))
            {
                break;
            }
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut execution)
                .await
                .is_err()
        );
        cancel.cancel();
        let consumer = async {
            while let Some(event) = receiver.recv().await {
                history.push(event);
            }
        };
        let (result, ()) = tokio::join!(execution, consumer);
        let result = result.unwrap();
        check_history(&result, &history);
        assert_eq!(result.outcome, RunOutcome::Cancelled);
        assert!(
            history
                .iter()
                .any(|e| matches!(e.kind, EventKind::ModelTextDelta { .. }))
        );
    }
}

struct CancelOnApproval(Cancellation);
#[async_trait]
impl PermissionHandler for CancelOnApproval {
    async fn decide(&mut self, _: &PermissionRequest) -> io::Result<bool> {
        self.0.cancel();
        Ok(true)
    }
}
#[tokio::test]
async fn approval_racing_with_cancellation_does_not_dispatch_shell() {
    let root = tempfile::tempdir().unwrap();
    let cancel = Cancellation::default();
    let provider = Script::new(vec![response(
        vec![call("shell", "shell", json!({"command":"touch forbidden"}))],
        "",
    )]);
    let (result, events) = recorded(
        &provider,
        &tools(root.path()),
        &mut CancelOnApproval(cancel.clone()),
        cancel,
        20,
    )
    .await;
    check_history(&result, &events);
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert!(!events.iter().any(|e| matches!(
        e.kind,
        EventKind::PermissionGranted | EventKind::ToolCallStarted
    )));
    assert!(!root.path().join("forbidden").exists());
}
#[tokio::test]
async fn ui_failure_after_commit_cannot_relabel_the_mutation() {
    let root = tempfile::tempdir().unwrap();
    let cancel = Cancellation::default();
    let provider = Script::new(vec![
        response(
            vec![call(
                "write",
                "write_file",
                json!({"path":"committed","content":"yes","overwrite":false}),
            )],
            "",
        ),
        response(vec![], "done"),
    ]);
    let tools = tools(root.path());
    let mut permission = Confirmation::new(false);
    let (sender, mut receiver) = mpsc::channel(1);
    let execution = runtime::run(
        &provider,
        &tools,
        &mut permission,
        config(20),
        cancel.clone(),
        Some(sender),
    );
    let consumer = async {
        while let Some(event) = receiver.recv().await {
            if matches!(event.kind, EventKind::ToolCallCompleted { .. }) {
                // The application chooses cancellation on a rendering failure.
                cancel.cancel();
                drop(receiver);
                break;
            }
        }
    };
    let (result, ()) = tokio::join!(execution, consumer);
    let result = result.unwrap();
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert_eq!(
        fs::read_to_string(root.path().join("committed")).unwrap(),
        "yes"
    );
    assert_eq!(
        result.state.tools.values().next().unwrap().status,
        Status::Completed
    );
}
#[tokio::test]
async fn wrong_parent_ids_and_second_tool_terminal_are_rejected_atomically() {
    let root = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![
        response(
            vec![call("read", "list_directory", json!({"path":"."}))],
            "",
        ),
        response(vec![], "done"),
    ]);
    let (result, events) = recorded(
        &provider,
        &tools(root.path()),
        &mut Confirmation::new(false),
        Cancellation::default(),
        20,
    )
    .await;
    let mut state = ExecutionState::new(result.session.id, result.state.run_id);
    for event in &events {
        let mut bad = event.clone();
        bad.run_id = RunId::default();
        let before = state.clone();
        assert!(state.transition(&bad).is_err());
        assert_eq!(state, before);
        if matches!(event.kind, EventKind::ToolCallStarted) {
            bad = event.clone();
            bad.model_call_id = Some(ModelCallId::default());
            assert!(state.transition(&bad).is_err());
            assert_eq!(state, before);
        }
        state.transition(event).unwrap();
        if matches!(event.kind, EventKind::ToolCallCompleted { .. }) {
            let mut duplicate = event.clone();
            duplicate.sequence = state.sequence + 1;
            let before = state.clone();
            assert!(state.transition(&duplicate).is_err());
            assert_eq!(state, before);
        }
    }
}

#[tokio::test]
async fn replay_rejects_tool_start_without_matching_policy_and_approval() {
    use astrid::permissions::{Capability, PermissionAction};
    let root = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![
        response(vec![call("shell", "shell", json!({"command":"true"}))], ""),
        response(vec![], "done"),
    ]);
    let (result, events) = recorded(
        &provider,
        &tools(root.path()),
        &mut Confirmation::new(true),
        Cancellation::default(),
        20,
    )
    .await;
    let mut state = ExecutionState::new(result.session.id, result.state.run_id);
    let start = events
        .iter()
        .find(|event| matches!(event.kind, EventKind::ToolCallStarted))
        .unwrap();
    for event in &events {
        if matches!(
            event.kind,
            EventKind::PermissionPolicyEvaluated { .. } | EventKind::PermissionRequested { .. }
        ) {
            let before = state.clone();
            let mut unauthorized = start.clone();
            unauthorized.sequence = state.sequence + 1;
            assert!(state.transition(&unauthorized).is_err());
            assert_eq!(state, before);
        }
        if matches!(event.kind, EventKind::PermissionPolicyEvaluated { .. }) {
            let before = state.clone();
            let mut forged = event.clone();
            forged.kind = EventKind::PermissionPolicyEvaluated {
                capability: Capability::Execute,
                action: PermissionAction::Allow,
                reason: "forged allow".into(),
            };
            assert!(state.transition(&forged).is_err());
            assert_eq!(state, before);
            forged.kind = EventKind::PermissionPolicyEvaluated {
                capability: Capability::Read,
                action: PermissionAction::Ask,
                reason: "forged capability".into(),
            };
            assert!(state.transition(&forged).is_err());
            assert_eq!(state, before);
        }
        state.transition(event).unwrap();
    }
    assert_eq!(state, result.state);
}
