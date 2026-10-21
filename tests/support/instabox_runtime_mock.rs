use super::*;
use crate::{
    events::{EventKind, ExecutionEvent, ExecutionState, Status},
    instabox::tests::{ID, client_at, request_server},
    model::{Message, ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink},
    permissions::{PermissionMode, PermissionPolicy},
    runtime::{self, PermissionHandler, RunConfig, RunOutcome},
    tools::{PermissionRequest, Tools},
    workspace::Workspace,
};
use std::{collections::VecDeque, io, sync::Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{Notify, mpsc},
};

const GOOD: &str = r#"{"stdout":"actual out","stderr":"actual err","exitCode":0,"timedOut":false,"truncated":false}"#;
fn call() -> ToolCall {
    ToolCall { call_id: "provider-call".into(), name: "instabox_exec".into(),
        arguments: json!({"command":"printf 'private-command'", "cwd":"/private-path", "timeout_seconds":1}).to_string() }
}
fn running() -> String {
    format!(r#"{{"id":"{ID}","state":"running","name":"PRIVATE-METADATA"}}"#)
}
fn result_data(execution: ToolExecution) -> Value {
    match execution {
        ToolExecution::Finished(ToolResult {
            outcome: ToolOutcome::Success { data } | ToolOutcome::TimedOut { data },
            ..
        })
        | ToolExecution::CancelledWithOutput { data } => data,
        other => panic!("unexpected {other:?}"),
    }
}

#[derive(Clone)]
enum Decision {
    Allow,
    Deny,
    Expired,
    Changed,
    Cancel(Cancellation),
    Wait,
}
struct Broker {
    decision: Decision,
    requests: Mutex<Vec<ApprovalRequest>>,
    waiting: Notify,
    release: Notify,
}
impl Broker {
    fn new(decision: Decision) -> Arc<Self> {
        Arc::new(Self {
            decision,
            requests: Mutex::new(vec![]),
            waiting: Notify::new(),
            release: Notify::new(),
        })
    }
}
#[async_trait]
impl RemoteApproval for Broker {
    async fn approve(&self, request: &ApprovalRequest) -> Option<RemoteGrant> {
        self.requests.lock().unwrap().push(request.clone());
        self.waiting.notify_one();
        match &self.decision {
            Decision::Deny => None,
            Decision::Expired => {
                Some(RemoteGrant::approve(request.clone(), Duration::ZERO).unwrap())
            }
            Decision::Changed => {
                let mut changed = request.clone();
                changed.args.command.push_str(" changed");
                Some(RemoteGrant::approve(changed, Duration::from_secs(60)).unwrap())
            }
            Decision::Cancel(cancel) => {
                cancel.cancel();
                Some(RemoteGrant::approve(request.clone(), Duration::from_secs(60)).unwrap())
            }
            Decision::Wait => {
                self.release.notified().await;
                Some(RemoteGrant::approve(request.clone(), Duration::from_secs(60)).unwrap())
            }
            Decision::Allow => {
                Some(RemoteGrant::approve(request.clone(), Duration::from_secs(60)).unwrap())
            }
        }
    }
}
fn tool(client: DiscoveryClient, broker: Arc<Broker>) -> InstaboxTool {
    InstaboxTool::new(
        "local-principal".into(),
        ID.into(),
        Arc::new(client),
        broker,
    )
    .unwrap()
}
fn binding(tool: &InstaboxTool) -> ApprovalRequest {
    tool.request(RunId::default(), ToolCallId::default(), &call())
        .unwrap()
}

#[tokio::test]
async fn canonical_arguments_reject_target_credentials_and_bind_every_identity() {
    let (client, server) = request_server(vec![]).await;
    let tool = tool(client, Broker::new(Decision::Allow));
    let original = binding(&tool);
    let mut reordered = call();
    reordered.arguments =
        r#"{ "timeout_seconds":1, "cwd":"/private-path", "command":"printf 'private-command'" }"#
            .into();
    assert_eq!(
        tool.request(original.run_id, original.tool_call_id, &reordered)
            .unwrap(),
        original
    );
    for field in [
        "machine_id",
        "target",
        "key",
        "credentials",
        "principal",
        "run_id",
        "env",
        "stdin",
    ] {
        let mut changed = call();
        let mut args: Value = serde_json::from_str(&changed.arguments).unwrap();
        args[field] = json!("injected");
        changed.arguments = args.to_string();
        assert!(
            tool.request(original.run_id, original.tool_call_id, &changed)
                .is_err()
        );
    }
    for (field, value) in [
        ("command", json!(" ")),
        ("command", json!("bad\0command")),
        ("cwd", json!("relative")),
        ("timeout_seconds", json!(0)),
        ("timeout_seconds", json!(601)),
        ("timeout_seconds", json!(1.5)),
    ] {
        let mut changed = call();
        let mut args: Value = serde_json::from_str(&changed.arguments).unwrap();
        args[field] = value;
        changed.arguments = args.to_string();
        assert!(
            tool.request(original.run_id, original.tool_call_id, &changed)
                .is_err()
        );
    }
    let grant = RemoteGrant::approve(original.clone(), Duration::from_secs(60)).unwrap();
    for index in 0..8 {
        let mut changed = original.clone();
        match index {
            0 => changed.principal.push('x'),
            1 => changed.run_id = RunId::default(),
            2 => changed.tool_call_id = ToolCallId::default(),
            3 => changed.call_id.push('x'),
            4 => changed.machine_id = "m_abcdefghijklmnopqrstuvwxyz".into(),
            5 => changed.args.command.push(' '),
            6 => changed.args.cwd.push('/'),
            _ => changed.args.timeout_seconds += 1,
        }
        let data = result_data(
            tool.execute(&changed, &grant, &Cancellation::default())
                .await,
        );
        assert_eq!(data["remote_outcome"], "not_dispatched");
        assert_eq!(data["failure"], "permission_denied");
    }
    assert!(server.await.unwrap().is_empty());
    let schema = definition().to_string();
    for secret in ["ibk_test_only", "local-principal", ID] {
        assert!(!schema.contains(secret));
    }
    assert!(original.description().contains("auto-wake"));
}

#[tokio::test]
async fn grant_consumption_is_atomic_and_cannot_be_reused_after_an_attempt() {
    let (client, server) = request_server(vec![(200, running()), (200, GOOD.into())]).await;
    let tool = tool(client, Broker::new(Decision::Allow));
    let request = binding(&tool);
    let grant = Arc::new(RemoteGrant::approve(request.clone(), Duration::from_secs(60)).unwrap());
    let gate = Arc::new(std::sync::Barrier::new(12));
    let tasks = (0..12)
        .map(|_| {
            let (grant, gate, request) = (grant.clone(), gate.clone(), request.clone());
            std::thread::spawn(move || {
                gate.wait();
                grant.consume(&request, &Cancellation::default()).is_ok()
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        tasks
            .into_iter()
            .filter_map(|t| t.join().ok())
            .filter(|x| *x)
            .count(),
        1
    );
    let fresh = RemoteGrant::approve(request.clone(), Duration::from_secs(60)).unwrap();
    assert_eq!(
        result_data(
            tool.execute(&request, &fresh, &Cancellation::default())
                .await
        )["successful"],
        true
    );
    assert_eq!(
        result_data(
            tool.execute(&request, &fresh, &Cancellation::default())
                .await
        )["failure"],
        "grant_consumed"
    );
    assert_eq!(server.await.unwrap().len(), 2);
}

struct Script {
    responses: Mutex<VecDeque<ModelResponse>>,
    inputs: Mutex<Vec<String>>,
    advertised: Mutex<Vec<bool>>,
}
impl Script {
    fn new() -> Self {
        Self::with_call_id("provider-call")
    }
    fn with_call_id(id: &str) -> Self {
        let mut request = call();
        request.call_id = id.into();
        let response = crate::openai::completed_response(json!({"status":"completed", "output":[
            {"type":"function_call","id":"item","status":"completed","namespace":"astrid","name":request.name,"call_id":request.call_id,"arguments":request.arguments}
        ]})).unwrap();
        let done = crate::openai::completed_response(json!({"status":"completed", "output":[
            {"type":"message","id":"done","role":"assistant","status":"completed","content":[{"type":"output_text","text":"inspect actual report","annotations":[]}]}
        ]})).unwrap();
        Self {
            responses: Mutex::new(VecDeque::from([response, done])),
            inputs: Mutex::new(vec![]),
            advertised: Mutex::new(vec![]),
        }
    }
}
#[async_trait]
impl ModelProvider for Script {
    fn capabilities(&self, _: &str) -> crate::routing::ModelCapability {
        crate::routing::ModelCapability {
            visible_history: true,
            ..Default::default()
        }
    }
    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        _: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        self.inputs.lock().unwrap().push(format!(
            "{} {}",
            request.instructions,
            serde_json::to_string(request.messages).unwrap()
        ));
        self.advertised
            .lock()
            .unwrap()
            .push(request.options.instabox_exec);
        Ok(self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("no automatic retry/inference"))
    }
}
struct LocalApproval;
#[async_trait]
impl PermissionHandler for LocalApproval {
    async fn decide(&mut self, _: &PermissionRequest) -> io::Result<bool> {
        panic!("local approval must not authorize remote tool")
    }
}
fn config(policy: PermissionPolicy, traces: Option<std::path::PathBuf>) -> RunConfig {
    RunConfig {
        routing: None,
        observability: traces.map(crate::observability::Options::new),
        model: "test".into(),
        task: "mock remote request".into(),
        max_model_calls: 2,
        permissions: policy,
        context_budget: None,
    }
}
async fn run_recorded(
    provider: &Script,
    tools: &Tools,
    config: RunConfig,
    cancel: Cancellation,
) -> (runtime::RunResult, Vec<ExecutionEvent>) {
    let (tx, mut rx) = mpsc::channel(2);
    let collect = async {
        let mut all = vec![];
        while let Some(event) = rx.recv().await {
            all.push(event);
        }
        all
    };
    let mut local = LocalApproval;
    let (result, events) = tokio::join!(
        runtime::run(provider, tools, &mut local, config, cancel, Some(tx)),
        collect
    );
    let result = result.unwrap();
    let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
    for event in &events {
        replay.transition(event).unwrap();
    }
    assert_eq!(replay, result.state);
    (result, events)
}

#[tokio::test]
async fn every_local_preset_and_read_only_workers_lack_remote_authority() {
    for mode in [
        PermissionMode::Ask,
        PermissionMode::Auto,
        PermissionMode::Unbound,
    ] {
        for installed in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let (client, server) = request_server(vec![]).await;
            let broker = Broker::new(Decision::Deny);
            let tools =
                Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1)).unwrap();
            let tools = if installed {
                tools.with_instabox(tool(client, broker.clone()))
            } else {
                tools
            };
            let provider = Script::new();
            let (result, _) = run_recorded(
                &provider,
                &tools,
                config(mode.policy(), None),
                Cancellation::default(),
            )
            .await;
            assert_eq!(
                result.state.tools.values().next().unwrap().status,
                Status::Denied
            );
            assert!(server.await.unwrap().is_empty());
            assert_eq!(
                broker.requests.lock().unwrap().len(),
                usize::from(installed)
            );
        }
    }
    let root = tempfile::tempdir().unwrap();
    let (client, server) = request_server(vec![]).await;
    let broker = Broker::new(Decision::Allow);
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1))
        .unwrap()
        .with_instabox(tool(client, broker.clone()));
    let (result, _) = run_recorded(
        &Script::new(),
        &tools,
        config(PermissionPolicy::read_only(), None),
        Cancellation::default(),
    )
    .await;
    assert_eq!(
        result.state.tools.values().next().unwrap().status,
        Status::Denied
    );
    assert!(broker.requests.lock().unwrap().is_empty());
    assert!(server.await.unwrap().is_empty());
}

#[tokio::test]
async fn expired_modified_denied_and_cancelled_approval_never_dispatch() {
    let cancel = Cancellation::default();
    for decision in [
        Decision::Deny,
        Decision::Expired,
        Decision::Changed,
        Decision::Cancel(cancel.clone()),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (client, server) = request_server(vec![]).await;
        let broker = Broker::new(decision);
        let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1))
            .unwrap()
            .with_instabox(tool(client, broker));
        let (result, events) = run_recorded(
            &Script::new(),
            &tools,
            config(PermissionMode::Unbound.policy(), None),
            cancel.clone(),
        )
        .await;
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::ToolCallStarted))
        );
        if cancel.is_cancelled() {
            assert_eq!(result.outcome, RunOutcome::Cancelled);
            assert!(
                !result
                    .session
                    .messages
                    .iter()
                    .any(|m| matches!(m, Message::Tool(_)))
            );
        } else {
            assert_eq!(
                result.state.tools.values().next().unwrap().status,
                Status::Denied
            );
        }
        assert!(server.await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn waiting_approval_has_no_authenticated_request_and_cancellation_wins() {
    let root = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = client_at(listener.local_addr().unwrap());
    let broker = Broker::new(Decision::Wait);
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1))
        .unwrap()
        .with_instabox(tool(client, broker.clone()));
    let cancel = Cancellation::default();
    let provider = Script::new();
    let execute = run_recorded(
        &provider,
        &tools,
        config(PermissionMode::Unbound.policy(), None),
        cancel.clone(),
    );
    let check = async {
        broker.waiting.notified().await;
        assert!(
            tokio::time::timeout(Duration::from_millis(30), listener.accept())
                .await
                .is_err()
        );
        cancel.cancel();
        broker.release.notify_one();
    };
    let ((result, events), ()) = tokio::join!(execute, check);
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
}

#[tokio::test]
async fn runtime_result_wire_attribution_and_private_metadata_trace_remain_separate() {
    for trace_on in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let trace_path = root.path().join("traces");
        let (client, server) = request_server(vec![(200, running()), (200, GOOD.into())]).await;
        let broker = Broker::new(Decision::Allow);
        let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1))
            .unwrap()
            .with_instabox(tool(client, broker.clone()));
        let provider = Script::new();
        let (result, events) = run_recorded(
            &provider,
            &tools,
            config(
                PermissionMode::Ask.policy(),
                trace_on.then(|| trace_path.clone()),
            ),
            Cancellation::default(),
        )
        .await;
        assert_eq!(result.outcome, RunOutcome::Completed);
        let approval = broker.requests.lock().unwrap()[0].clone();
        assert_eq!(approval.run_id, result.state.run_id);
        let data = result
            .session
            .messages
            .iter()
            .find_map(|m| {
                if let Message::Tool(t) = m {
                    match &t.outcome {
                        ToolOutcome::Success { data } => Some(data),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(data["machine_id"], ID);
        assert_eq!(data["stdout"], "actual out");
        assert_eq!(data["stderr"], "actual err");
        assert_eq!(data["tool_call_id"], json!(approval.tool_call_id));
        assert_eq!(data["call_id"], "provider-call");
        for event in events.iter().filter(|e| e.tool_call_id.is_some()) {
            assert!(event.model_call_id.is_some());
            assert!(event.turn_id.is_some());
            assert_eq!(event.run_id, approval.run_id);
        }
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        let wire: Value = serde_json::from_str(requests[1].split_once('\n').unwrap().1).unwrap();
        assert_eq!(
            wire,
            json!({"cmd":"printf 'private-command'","cwd":"/private-path","timeoutSeconds":1})
        );
        for input in provider.inputs.lock().unwrap().iter() {
            assert!(!input.contains("ibk_test_only"));
            assert!(!input.contains("PRIVATE-METADATA"));
        }
        assert!(
            !serde_json::to_string(&result.session)
                .unwrap()
                .contains("ibk_test_only")
        );
        if trace_on {
            let trace =
                crate::observability::read_trace(&trace_path, &result.state.run_id.to_string())
                    .unwrap();
            assert!(trace.complete);
            let bytes =
                std::fs::read_to_string(trace_path.join(format!("{}.jsonl", result.state.run_id)))
                    .unwrap();
            for private in [
                ID,
                "local-principal",
                "private-command",
                "private-path",
                "actual out",
                "actual err",
                "PRIVATE-METADATA",
                "ibk_test_only",
            ] {
                assert!(!bytes.contains(private), "leaked {private}");
            }
            assert!(bytes.contains("instabox_exec"));
        } else {
            assert!(!trace_path.exists());
            assert_eq!(
                result.recording.status,
                crate::observability::RecordingStatus::Off
            );
        }
        // Resuming without explicit host capability cannot inherit authority.
        let plain =
            Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1)).unwrap();
        let resumed = runtime::run_in_session(
            &Script::with_call_id("resumed-call"),
            &plain,
            &mut LocalApproval,
            config(PermissionMode::Unbound.policy(), None),
            Cancellation::default(),
            None,
            result.session,
        )
        .await
        .unwrap();
        assert_eq!(
            resumed.state.tools.values().next().unwrap().status,
            Status::Denied
        );
    }
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut bytes = vec![];
    while !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).await.unwrap();
        bytes.push(byte[0]);
    }
    let headers = String::from_utf8(bytes).unwrap();
    let count = headers
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length: ")
                .map(str::to_owned)
        })
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0; count];
    stream.read_exact(&mut body).await.unwrap();
    headers
}
async fn send_body(stream: &mut tokio::net::TcpStream, body: &str) {
    stream
        .write_all(
            format!(
                "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn expiry_or_cancellation_after_metadata_prevents_post_and_does_not_consume() {
    for expired in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tool = tool(
            client_at(listener.local_addr().unwrap()),
            Broker::new(Decision::Allow),
        );
        let request = binding(&tool);
        let cancel = Cancellation::default();
        let grant = RemoteGrant::approve(request.clone(), Duration::from_millis(100)).unwrap();
        let server = async {
            let (mut stream, _) = listener.accept().await.unwrap();
            assert!(read_request(&mut stream).await.starts_with("GET "));
            if expired {
                tokio::time::sleep_until(grant.expires.into()).await;
            } else {
                cancel.cancel();
            }
            send_body(&mut stream, &running()).await;
            assert!(
                tokio::time::timeout(Duration::from_millis(30), listener.accept())
                    .await
                    .is_err()
            );
        };
        let (execution, ()) = tokio::join!(tool.execute(&request, &grant, &cancel), server);
        if expired {
            assert_eq!(result_data(execution)["failure"], "grant_expired");
        } else {
            assert!(matches!(execution, ToolExecution::Cancelled));
        }
        assert!(!grant.consumed.load(Ordering::Acquire));
    }
}

#[tokio::test]
async fn post_failures_nonzero_timeout_and_truncation_retain_actual_report_without_replay() {
    for (status, body, outcome, successful) in [
        (200, GOOD.to_owned(), "observed_completed", true),
        (
            200,
            GOOD.replace("\"exitCode\":0", "\"exitCode\":7"),
            "observed_completed",
            false,
        ),
        (
            200,
            GOOD.replace("\"timedOut\":false", "\"timedOut\":true")
                .replace("\"exitCode\":0", "\"exitCode\":124"),
            "observed_completed",
            false,
        ),
        (
            200,
            GOOD.replace("\"truncated\":false", "\"truncated\":true"),
            "observed_completed",
            true,
        ),
        (200, "MALFORMED-PRIVATE-ERROR".into(), "unknown", false),
        (200, "x".repeat(262_145), "unknown", false),
        (
            200,
            r#"{"stdout":"partial","stderr":"","timedOut":false,"truncated":false}"#.into(),
            "unknown",
            false,
        ),
        (403, "MALFORMED-PRIVATE-ERROR".into(), "unknown", false),
        (302, "MALFORMED-PRIVATE-ERROR".into(), "unknown", false),
        (503, "MALFORMED-PRIVATE-ERROR".into(), "unknown", false),
    ] {
        let (client, server) = request_server(vec![(200, running()), (status, body)]).await;
        let tool = tool(client, Broker::new(Decision::Allow));
        let request = binding(&tool);
        let grant = RemoteGrant::approve(request.clone(), Duration::from_secs(60)).unwrap();
        let execution = tool
            .execute(&request, &grant, &Cancellation::default())
            .await;
        if let ToolExecution::Finished(result) = &execution {
            assert_eq!(result.is_error(), !successful);
        }
        let data = result_data(execution);
        assert_eq!(data["remote_outcome"], outcome);
        assert_eq!(data["successful"], successful);
        assert_eq!(data["post_admitted"], true);
        assert_eq!(data["machine_id"], ID);
        if outcome == "unknown" {
            assert!(data["exit_status"].is_null());
            assert_eq!(data["output_available"], false);
        }
        let text = data.to_string();
        for secret in ["ibk_test_only", "MALFORMED-PRIVATE-ERROR"] {
            assert!(!text.contains(secret));
        }
        assert_eq!(
            result_data(
                tool.execute(&request, &grant, &Cancellation::default())
                    .await
            )["failure"],
            "grant_consumed"
        );
        assert_eq!(server.await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn metadata_and_post_body_deadlines_keep_distinct_dispatch_outcomes() {
    for post in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tool = tool(
            client_at(listener.local_addr().unwrap()),
            Broker::new(Decision::Allow),
        );
        let request = binding(&tool);
        let grant = RemoteGrant::approve(request.clone(), Duration::from_secs(60)).unwrap();
        let (sent, ready) = tokio::sync::oneshot::channel();
        let hold = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            read_request(&mut stream).await;
            if post {
                send_body(&mut stream, &running()).await;
                let (next, _) = listener.accept().await.unwrap();
                stream = next;
                read_request(&mut stream).await;
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n{")
                .await
                .unwrap();
            sent.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        let token = Cancellation::default();
        let execute = tool.execute_waits(
            &request,
            &grant,
            &token,
            (Duration::from_millis(100), Duration::from_millis(100)),
        );
        let (execution, ()) = tokio::join!(execute, async {
            ready.await.unwrap();
        });
        let data = result_data(execution);
        assert_eq!(
            data["remote_outcome"],
            if post { "unknown" } else { "not_dispatched" }
        );
        assert_eq!(data["post_admitted"], post);
        assert!(data["exit_status"].is_null());
        assert_eq!(grant.consumed.load(Ordering::Acquire), post);
        if post {
            assert_eq!(
                result_data(
                    tool.execute(&request, &grant, &Cancellation::default())
                        .await
                )["failure"],
                "grant_consumed"
            );
        }
        hold.abort();
        assert!(hold.await.unwrap_err().is_cancelled());
    }
}

#[tokio::test]
async fn dispatched_cancellation_retains_unknown_and_incomplete_batch_blocks_resume() {
    let root = tempfile::tempdir().unwrap();
    let traces = root.path().join("traces");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cancel = Cancellation::default();
    let signal = cancel.clone();
    let remote = tool(
        client_at(listener.local_addr().unwrap()),
        Broker::new(Decision::Allow),
    );
    let hold = tokio::spawn(async move {
        let (mut metadata, _) = listener.accept().await.unwrap();
        read_request(&mut metadata).await;
        send_body(&mut metadata, &running()).await;
        let (mut post, _) = listener.accept().await.unwrap();
        assert!(read_request(&mut post).await.starts_with("POST "));
        post.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n{")
            .await
            .unwrap();
        signal.cancel();
        std::future::pending::<()>().await;
    });
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1))
        .unwrap()
        .with_instabox(remote);
    let (result, events) = run_recorded(
        &Script::new(),
        &tools,
        config(PermissionMode::Unbound.policy(), Some(traces.clone())),
        cancel,
    )
    .await;
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    let data = events
        .iter()
        .find_map(|e| {
            if let EventKind::ToolCallCancelled { output: Some(data) } = &e.kind {
                Some(data)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(data["remote_outcome"], "unknown");
    assert_eq!(data["machine_id"], ID);
    assert_eq!(data["post_admitted"], true);
    assert!(
        !result
            .session
            .messages
            .iter()
            .any(|m| matches!(m, Message::Tool(_)))
    );
    assert_eq!(result.model_calls, 1);
    let trace =
        std::fs::read_to_string(traces.join(format!("{}.jsonl", result.state.run_id))).unwrap();
    for secret in [
        ID,
        "ibk_test_only",
        "private-command",
        "private-path",
        "local-principal",
    ] {
        assert!(!trace.contains(secret));
    }
    let before = serde_json::to_vec(&result.session).unwrap();
    let error = runtime::run_in_session(
        &Script::new(),
        &tools,
        &mut LocalApproval,
        config(PermissionMode::Unbound.policy(), None),
        Cancellation::default(),
        None,
        result.session,
    )
    .await
    .unwrap_err();
    assert!(error.message.contains("incomplete tool batch"));
    assert_eq!(serde_json::to_vec(&error.session).unwrap(), before);
    hold.abort();
    assert!(hold.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn concurrent_http_attempts_share_one_grant_but_admit_only_one_post() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tool = tool(
        client_at(listener.local_addr().unwrap()),
        Broker::new(Decision::Allow),
    );
    let request = binding(&tool);
    let grant = RemoteGrant::approve(request.clone(), Duration::from_secs(60)).unwrap();
    let server = async {
        let (mut first, _) = listener.accept().await.unwrap();
        assert!(read_request(&mut first).await.starts_with("GET "));
        let (mut second, _) = listener.accept().await.unwrap();
        assert!(read_request(&mut second).await.starts_with("GET "));
        send_body(&mut first, &running()).await;
        send_body(&mut second, &running()).await;
        let (mut post, _) = listener.accept().await.unwrap();
        assert!(read_request(&mut post).await.starts_with("POST "));
        send_body(&mut post, GOOD).await;
        assert!(
            tokio::time::timeout(Duration::from_millis(30), listener.accept())
                .await
                .is_err()
        );
    };
    let cancel = Cancellation::default();
    let (a, b, ()) = tokio::join!(
        tool.execute(&request, &grant, &cancel),
        tool.execute(&request, &grant, &cancel),
        server
    );
    let reports = [result_data(a), result_data(b)];
    assert_eq!(
        reports
            .iter()
            .filter(|r| r["post_admitted"] == true)
            .count(),
        1
    );
    assert_eq!(
        reports
            .iter()
            .filter(|r| r["failure"] == "grant_consumed")
            .count(),
        1
    );
}

#[tokio::test]
async fn transport_close_after_post_remains_unknown_and_burns_grant() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tool = tool(
        client_at(listener.local_addr().unwrap()),
        Broker::new(Decision::Allow),
    );
    let request = binding(&tool);
    let grant = RemoteGrant::approve(request.clone(), Duration::from_secs(60)).unwrap();
    let server = async {
        let (mut metadata, _) = listener.accept().await.unwrap();
        read_request(&mut metadata).await;
        send_body(&mut metadata, &running()).await;
        let (mut post, _) = listener.accept().await.unwrap();
        assert!(read_request(&mut post).await.starts_with("POST "));
        drop(post);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), listener.accept())
                .await
                .is_err()
        );
    };
    let cancel = Cancellation::default();
    let (execution, ()) = tokio::join!(tool.execute(&request, &grant, &cancel), server);
    let data = result_data(execution);
    assert_eq!(data["remote_outcome"], "unknown");
    assert_eq!(data["failure"], "transport");
    assert_eq!(
        result_data(tool.execute(&request, &grant, &cancel).await)["failure"],
        "grant_consumed"
    );
}

#[tokio::test]
async fn runtime_nonzero_and_unknown_reports_are_failed_lifecycle_with_actual_data() {
    for body in [
        GOOD.replace("\"exitCode\":0", "\"exitCode\":9"),
        "PRIVATE ERROR".into(),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (client, server) = request_server(vec![(200, running()), (200, body)]).await;
        let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1))
            .unwrap()
            .with_instabox(tool(client, Broker::new(Decision::Allow)));
        let (result, events) = run_recorded(
            &Script::new(),
            &tools,
            config(PermissionMode::Unbound.policy(), None),
            Cancellation::default(),
        )
        .await;
        assert_eq!(
            result.state.tools.values().next().unwrap().status,
            Status::Failed
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::ToolCallFailed { .. }))
        );
        let result = result
            .session
            .messages
            .iter()
            .find_map(|m| {
                if let Message::Tool(result) = m {
                    Some(result)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(result.is_error());
        assert!(
            !serde_json::to_string(result)
                .unwrap()
                .contains("PRIVATE ERROR")
        );
        assert_eq!(server.await.unwrap().len(), 2);
    }
}
