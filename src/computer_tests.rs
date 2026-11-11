use super::*;
use crate::{
    auth::{AuthError, Authentication, BearerToken},
    events::{EventKind, ExecutionEvent, ExecutionState},
    model::Message,
    openai::OpenAiProvider,
    runtime::{self, PermissionHandler, RunConfig, RunOutcome},
    tools::{PermissionRequest, Tools},
    workspace::Workspace,
};
use std::io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const ID: &str = "m_0123456789abcdefghijklmnop";
fn fixture() -> Vec<u8> {
    let mut v = Vec::new();
    {
        let mut e = png::Encoder::new(&mut v, 2, 2);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().unwrap();
        w.write_image_data(&[80; 12]).unwrap();
    }
    v
}
struct MockHttp {
    origin: String,
    requests: Arc<Mutex<Vec<(String, Value)>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for MockHttp {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl MockHttp {
    async fn start(
        mut reply: impl FnMut(usize, &str, &Value) -> (u16, &'static str, String) + Send + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let out = requests.clone();
        let task = tokio::spawn(async move {
            let mut n = 0;
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                while !bytes.ends_with(b"\r\n\r\n") {
                    let mut b = [0];
                    socket.read_exact(&mut b).await.unwrap();
                    bytes.push(b[0]);
                    assert!(bytes.len() < 16_384);
                }
                let header = String::from_utf8(bytes).unwrap();
                let path = header.lines().next().unwrap().to_owned();
                let length = header
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|n| n.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                assert!(length < 2_000_000);
                let mut body = vec![0; length];
                socket.read_exact(&mut body).await.unwrap();
                let body: Value = if body.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice(&body).unwrap()
                };
                out.lock().unwrap().push((path.clone(), body.clone()));
                let (status, content_type, response) = reply(n, &path, &body);
                n += 1;
                let wire = format!(
                    "HTTP/1.1 {status} Mock\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                    response.len()
                );
                let _ = socket.write_all(wire.as_bytes()).await;
            }
        });
        Self {
            origin,
            requests,
            task,
        }
    }
}
fn completed(output: Vec<Value>) -> String {
    format!(
        "data: {}\n\n",
        json!({"type":"response.completed","response":{"id":"resp_cu","status":"completed","output":output}})
    )
}
fn call(id: &str, name: &str, args: Value) -> Value {
    json!({"type":"function_call","id":format!("item_{id}"),"call_id":id,"name":name,"namespace":"astrid","arguments":args.to_string(),"status":"completed"})
}
fn message(text: &str) -> Value {
    json!({"type":"message","id":"msg_cu","role":"assistant","status":"completed","content":[{"type":"output_text","text":text,"annotations":[]}]})
}
struct Auth;
#[async_trait]
impl Authentication for Auth {
    async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
        BearerToken::new("test-only-provider-credential".into())
    }
}
#[derive(Clone, Copy, Default)]
enum DisclosureFixture {
    #[default]
    Normal,
    ExpiredYes,
    InputError,
}
struct Answers {
    disclosure: DisclosureFixture,
    values: Vec<bool>,
    commands: Vec<String>,
}
#[async_trait]
impl PermissionHandler for Answers {
    async fn decide(&mut self, request: &PermissionRequest) -> io::Result<bool> {
        self.commands.push(request.command.clone());
        if request.command.contains("image.disclose") {
            match self.disclosure {
                DisclosureFixture::ExpiredYes => {
                    tokio::time::pause();
                    tokio::time::advance(Duration::from_secs(10)).await;
                    return Ok(true);
                }
                DisclosureFixture::InputError => {
                    return Err(io::Error::other("private terminal error"));
                }
                DisclosureFixture::Normal => {}
            }
        }
        Ok(self
            .values
            .get(self.commands.len() - 1)
            .copied()
            .unwrap_or(false))
    }
}
async fn wire_server(action_status: u16) -> MockHttp {
    let image = STANDARD.encode(fixture());
    MockHttp::start(move|n,path,body|{
        if n==0{assert_eq!(path,format!("GET /v1/machines/{ID} HTTP/1.1"));(200,"application/json",json!({"id":ID,"state":"running","secret":"machine-secret"}).to_string())}
        else if n==1{assert_eq!(body["cmd"],crate::computer_wire::CAPTURE);assert_eq!(body["cwd"],"/data/home");assert_eq!(body["timeoutSeconds"],2);assert!(body.get("stdin").is_none());
            (200,"application/json",json!({"stdout":image,"stderr":"capture-error-sentinel","exitCode":0,"timedOut":false,"truncated":false}).to_string())}
        else{assert_eq!(n,2,"unexpected retry");assert_eq!(body["cmd"],"export DISPLAY=:1; xdotool mousemove 1 1 click 1");
            (action_status,"application/json",json!({"stdout":"","stderr":"remote-secret","exitCode":0,"timedOut":false,"truncated":false}).to_string())}
    }).await
}
async fn provider_server() -> MockHttp {
    MockHttp::start(move |n, _, body| {
        let response = match n {
            0 => completed(vec![call(
                "capture",
                "instabox_observe",
                json!({"kind":"screenshot"}),
            )]),
            1 => {
                let frame = body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|v| v["type"] == "function_call_output")
                    .find_map(|v| {
                        let value: Value = serde_json::from_str(v["output"].as_str()?).ok()?;
                        value["data"]["observed"]["frame_id"]
                            .as_str()
                            .map(str::to_string)
                    })
                    .unwrap();
                completed(vec![call(
                    "click",
                    "instabox_action",
                    json!({"frame_id":frame,"action":{"kind":"click","x":1,"y":1,"button":"left"}}),
                )])
            }
            2 => completed(vec![message(
                "Input acknowledged; UI effect is unverified.",
            )]),
            _ => panic!("unexpected provider retry or continuation"),
        };
        (200, "text/event-stream", response)
    })
    .await
}
fn service(wire: &MockHttp) -> Computer {
    Computer::new(
        "test-principal".into(),
        ID.into(),
        Arc::new(ComputerWire::at(
            "ibk_test_only_computer_credential",
            wire.origin.clone(),
        )),
        Arc::new(DecodeWorker::inline()),
    )
    .unwrap()
}
type ComputerCase = (
    runtime::RunResult,
    Vec<ExecutionEvent>,
    Answers,
    MockHttp,
    MockHttp,
    tempfile::TempDir,
    tempfile::TempDir,
);
async fn execute_case(
    answers: Vec<bool>,
    action_status: u16,
    record: bool,
    byte_limit: Option<usize>,
) -> ComputerCase {
    execute_case_with_disclosure(
        answers,
        action_status,
        record,
        byte_limit,
        DisclosureFixture::Normal,
    )
    .await
}
async fn execute_case_with_disclosure(
    answers: Vec<bool>,
    action_status: u16,
    record: bool,
    byte_limit: Option<usize>,
    disclosure: DisclosureFixture,
) -> ComputerCase {
    let wire = wire_server(action_status).await;
    let api = provider_server().await;
    let workspace = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let tools = Tools::new(
        Workspace::new(workspace.path()).unwrap(),
        Duration::from_secs(1),
    )
    .unwrap()
    .with_computer(service(&wire));
    let provider =
        OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{}/v1/responses", api.origin))
            .unwrap();
    let mut answers = Answers {
        disclosure,
        values: answers,
        commands: Vec::new(),
    };
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    let config = RunConfig {
        routing: None,
        observability: record
            .then(|| crate::observability::Options::new(logs.path().join("traces"))),
        context_budget: Some(crate::context::ContextBudget {
            max_request_bytes: byte_limit
                .unwrap_or_else(|| crate::context::ContextBudget::default().max_request_bytes),
            ..Default::default()
        }),
        model: "gpt-5.6-sol".into(),
        task: "Click the scratch target once; report ack only".into(),
        max_model_calls: 3,
        permissions: Default::default(),
    };
    let execution = runtime::run(
        &provider,
        &tools,
        &mut answers,
        config,
        Cancellation::default(),
        Some(tx),
    );
    let collect = async {
        let mut events = Vec::new();
        while let Some(e) = rx.recv().await {
            events.push(e);
        }
        events
    };
    let (result, events) = tokio::join!(execution, collect);
    let result = result.unwrap();
    let mut state = ExecutionState::new(result.session.id, result.state.run_id);
    for event in &events {
        state.transition(event).unwrap();
    }
    assert_eq!(state, result.state);
    (result, events, answers, wire, api, logs, workspace)
}

#[tokio::test]
async fn real_adapter_runtime_capture_disclosure_action_and_text_only_followup() {
    let (result, events, answers, wire, api, _, _) =
        execute_case(vec![true, true, true], 200, false, None).await;
    assert_eq!(result.outcome, RunOutcome::Completed);
    assert_eq!(result.model_calls, 3);
    assert_eq!(result.tool_calls, 2);
    assert_eq!(answers.commands.len(), 3);
    assert!(answers.commands[0].contains("gui.observe"));
    assert!(answers.commands[1].contains("image.disclose"));
    assert!(answers.commands[2].contains("gui.act"));
    let requests = api.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(!requests[0].1.to_string().contains("data:image"));
    let image = requests[1].1["input"].as_array().unwrap().last().unwrap()["content"][1].clone();
    assert_eq!(image["detail"], "original");
    assert_eq!(
        image["image_url"],
        format!(
            "data:image/png;base64,{}",
            STANDARD.encode(crate::computer_png::canonicalize(&fixture()).unwrap().bytes)
        )
    );
    assert!(!requests[2].1.to_string().contains("data:image"));
    assert_eq!(requests[1].1["store"], false);
    let serial = serde_json::to_string(&result.session).unwrap();
    for forbidden in [
        "data:image",
        "ibk_test_only",
        "test-only-provider-credential",
        "remote-secret",
        "capture-error-sentinel",
    ] {
        assert!(!serial.contains(forbidden));
    }
    assert_eq!(wire.requests.lock().unwrap().len(), 3);
    assert!(events.iter().any(
        |e| matches!(e.kind, EventKind::PermissionRequested { .. }) && e.tool_call_id.is_none()
    ));
    assert_eq!(
        result.session.replay_mode(),
        crate::routing::ReplayMode::VisibleHistory
    );
    assert!(
        result
            .session
            .messages
            .iter()
            .filter_map(|m| if let Message::Tool(t) = m {
                Some(t)
            } else {
                None
            })
            .all(|t| !t.is_error())
    );
}

#[tokio::test]
async fn capture_and_image_denial_never_send_unapproved_images_or_inputs() {
    for answers in [vec![false], vec![true, false]] {
        let (result, _, _, wire, api, _, _) = execute_case(answers.clone(), 200, false, None).await;
        assert!(matches!(result.outcome, RunOutcome::Failed { .. }));
        assert_eq!(api.requests.lock().unwrap().len(), 1);
        assert_eq!(
            wire.requests.lock().unwrap().len(),
            if answers.len() == 1 { 0 } else { 2 }
        );
        assert!(
            !serde_json::to_string(&result.session)
                .unwrap()
                .contains("data:image")
        );
    }
}

#[tokio::test]
async fn failed_input_stops_without_provider_continuation_retry_or_false_success() {
    let (result, _, _, wire, api, _, _) =
        execute_case(vec![true, true, true], 503, false, None).await;
    assert!(matches!(result.outcome, RunOutcome::Failed { .. }));
    assert_eq!(api.requests.lock().unwrap().len(), 2);
    assert_eq!(wire.requests.lock().unwrap().len(), 3);
    let actual = result
        .session
        .messages
        .iter()
        .rev()
        .find_map(|m| {
            if let Message::Tool(t) = m {
                Some(t)
            } else {
                None
            }
        })
        .unwrap();
    assert!(actual.is_error());
    let data = match &actual.outcome {
        ToolOutcome::Success { data } => data,
        _ => panic!("actual report"),
    };
    assert_eq!(data["remote_outcome"], "unknown");
}

#[tokio::test]
async fn persisted_metadata_and_session_snapshot_never_retain_image_payloads() {
    let (result, _, _, _, _, logs, root) =
        execute_case(vec![true, true, true], 200, true, None).await;
    assert_eq!(result.outcome, RunOutcome::Completed);
    let paths = crate::observability::list_traces(&logs.path().join("traces")).unwrap();
    assert!(!paths.is_empty());
    for path in paths {
        let text = std::fs::read_to_string(path).unwrap();
        for forbidden in [
            ID,
            "test-principal",
            "data:image",
            "capture-error-sentinel",
            "remote-secret",
            "ibk_test_only_computer_credential",
        ] {
            assert!(!text.contains(forbidden));
        }
    }
    let workspace = Workspace::new(root.path()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = crate::session_store::SessionStore::open(dir.path(), &workspace).unwrap();
    let snapshot = crate::session_store::SessionSnapshot::new(
        &workspace,
        0,
        vec![crate::session_store::StoredSession {
            session: result.session,
            runs: 1,
            outcome: Some(result.outcome),
        }],
    );
    store.save(&snapshot).unwrap();
    let loaded = store.load().unwrap().unwrap();
    let json = serde_json::to_string(&loaded).unwrap();
    assert!(!json.contains("data:image"));
    assert!(!json.contains("image_input"));
}

#[tokio::test(start_paused = true)]
async fn idle_expiry_real_receipts_disclosure_binding_and_single_use() {
    // No network in this test. This metadata comes from actual maintained PNG
    // validation, not the public mock ObservationMetadata seam.
    let wire = MockHttp::start(|_, _, _| panic!("unexpected network")).await;
    let mut run = service(&wire).begin(RunId::default());
    let cancel = Cancellation::default();
    let observed = run
        .install(
            crate::computer_png::canonicalize(&fixture()).unwrap(),
            Instant::now(),
            &cancel,
        )
        .unwrap();
    assert_eq!(observed["width"], 2);
    let image = run
        .take_vision(
            ModelCallId::default(),
            "openai_codex_subscription",
            "gpt-5.6-sol",
        )
        .unwrap()
        .unwrap();
    assert_eq!(image.estimate(), 3);
    let grant = image.approve();
    assert!(grant.consume(&image, "gpt-5.6-terra", &cancel).is_err());
    assert!(grant.consume(&image, "gpt-5.6-sol", &cancel).is_ok());
    assert!(grant.consume(&image, "gpt-5.6-sol", &cancel).is_err());
    assert!(
        run.take_vision(
            ModelCallId::default(),
            "openai_codex_subscription",
            "gpt-5.6-sol"
        )
        .unwrap()
        .is_none()
    );
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert!(run.frame.lock().unwrap().is_none());
    assert!(image.data_url().is_err());
}

#[test]
fn typing_is_stdin_and_fixed_key_scroll_plans_cannot_inject_shell() {
    let text = "  ' $(danger) `injection` café\n世界  ";
    let plan = Plan::action(&desktop::Action::Type { text: text.into() }).unwrap();
    assert!(!plan.cmd.contains(text));
    assert_eq!(plan.stdin.as_deref(), Some(text));
    assert_eq!(plan.timeout_seconds, 2);
    assert!(
        Plan::action(&desktop::Action::Key {
            key: "a;evil".into(),
            modifiers: vec![]
        })
        .is_err()
    );
    let plan = Plan::action(&desktop::Action::Key {
        key: "Enter".into(),
        modifiers: vec![desktop::Modifier::Ctrl],
    })
    .unwrap();
    assert_eq!(plan.cmd, "export DISPLAY=:1; xdotool key -- 'ctrl+Return'");
    assert!(
        Plan::action(&desktop::Action::Scroll {
            direction: desktop::Direction::Up,
            notches: 11
        })
        .is_err()
    );
}

#[tokio::test]
async fn image_byte_overflow_stops_before_disclosure_and_second_provider_request() {
    let (_, _, _, _, api, _, _) = execute_case(vec![true, true, true], 200, false, None).await;
    let cap = {
        let requests = api.requests.lock().unwrap();
        let mut text_only = requests[1].1.clone();
        text_only["input"].as_array_mut().unwrap().pop();
        serde_json::to_vec(&text_only).unwrap().len() + 32
    };
    let (result, _, answers, wire, api, _, _) =
        execute_case(vec![true, true, true], 200, false, Some(cap)).await;
    assert!(matches!(result.outcome, RunOutcome::Failed { .. }));
    assert_eq!(answers.commands.len(), 1);
    assert_eq!(wire.requests.lock().unwrap().len(), 2);
    assert_eq!(api.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn action_denial_never_dispatches_input_or_continues_inference() {
    let (result, _, answers, wire, api, _, _) =
        execute_case(vec![true, true, false], 200, false, None).await;
    assert!(matches!(result.outcome, RunOutcome::Failed { .. }));
    assert_eq!(answers.commands.len(), 3);
    assert_eq!(wire.requests.lock().unwrap().len(), 2);
    assert_eq!(api.requests.lock().unwrap().len(), 2);
}

fn observe_call(kind: &str) -> ToolCall {
    ToolCall {
        call_id: "observed-call".into(),
        name: "instabox_observe".into(),
        arguments: json!({"kind":kind}).to_string(),
    }
}
#[tokio::test]
async fn observation_binding_is_single_use_and_pre_cancel_has_zero_http() {
    let wire = MockHttp::start(|_, _, _| panic!("unapproved network")).await;
    let mut run = service(&wire).begin(RunId::default());
    let cancel = Cancellation::default();
    let prepared = run
        .prepare(ToolCallId::default(), &observe_call("screenshot"), &cancel)
        .unwrap();
    let Prepared::Observe(request) = &prepared else {
        panic!("observe")
    };
    let Permit::Observe(grant) = prepared.approve().unwrap() else {
        panic!("grant")
    };
    let mut changed = request.clone();
    changed.id = ToolCallId::default();
    assert!(grant.consume(&changed, &cancel).is_err());
    let wins = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| grant.consume(request, &cancel).is_ok()))
            .collect();
        handles
            .into_iter()
            .map(|h| usize::from(h.join().unwrap()))
            .sum::<usize>()
    });
    assert_eq!(wins, 1);
    cancel.cancel();
    let result = run.execute(prepared, Permit::Observe(grant), &cancel).await;
    assert!(matches!(result, ToolExecution::CancelledWithOutput { .. }));
    assert!(wire.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn status_returns_only_booleans_and_running_target_is_exact() {
    for state in ["running", "stopped", "wrong-id"] {
        let wire = MockHttp::start(move |n,path,body| {
            if n==0 {(200,"application/json",json!({"id":if state=="wrong-id"{"m_wrong"}else{ID},"state":state,"secret":"never-retain"}).to_string())}
            else {assert!(path.contains("/desktop "));assert_eq!(body,&json!({"action":"status"}));(200,"application/json",json!({"installed":true,"running":true,"url":"secret-viewer","password":"never-retain"}).to_string())}
        }).await;
        let mut run = service(&wire).begin(RunId::default());
        let cancel = Cancellation::default();
        let prepared = run
            .prepare(ToolCallId::default(), &observe_call("status"), &cancel)
            .unwrap();
        let permit = prepared.approve().unwrap();
        let result = run.execute(prepared, permit, &cancel).await;
        let ToolExecution::Finished(result) = result else {
            panic!("finished")
        };
        let ToolOutcome::Success { data } = result.outcome else {
            panic!("report")
        };
        assert_eq!(data["successful"], state == "running");
        assert_eq!(
            wire.requests.lock().unwrap().len(),
            if state == "running" { 2 } else { 1 }
        );
        if state == "running" {
            assert_eq!(data["observed"], json!({"installed":true,"running":true}));
        }
        assert!(!data.to_string().contains("secret"));
        assert!(!data.to_string().contains("never-retain"));
    }
}

#[tokio::test(start_paused = true)]
async fn stale_real_frame_rejected_without_refresh_and_replacement_revokes_disclosure() {
    let wire = MockHttp::start(|_, _, _| panic!("no automatic refresh/input")).await;
    let mut run = service(&wire).begin(RunId::default());
    let cancel = Cancellation::default();
    let observed = run
        .install(
            crate::computer_png::canonicalize(&fixture()).unwrap(),
            Instant::now(),
            &cancel,
        )
        .unwrap();
    let image = run
        .take_vision(
            ModelCallId::default(),
            "openai_codex_subscription",
            "gpt-5.6-sol",
        )
        .unwrap()
        .unwrap();
    let grant = image.approve();
    let call=ToolCall{call_id:"action-call".into(),name:"instabox_action".into(),arguments:json!({"frame_id":observed["frame_id"],"action":{"kind":"click","x":1,"y":1,"button":"left"}}).to_string()};
    let prepared = run.prepare(ToolCallId::default(), &call, &cancel).unwrap();
    let permit = prepared.approve().unwrap();
    tokio::time::advance(Duration::from_secs(5)).await;
    assert!(run.prepare(ToolCallId::default(), &call, &cancel).is_err());
    let result = run.execute(prepared, permit, &cancel).await;
    let ToolExecution::Finished(result) = result else {
        panic!("result")
    };
    assert!(result.is_error());
    assert!(run.stopped());
    assert!(grant.consume(&image, "gpt-5.6-sol", &cancel).is_err());
    assert!(wire.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn disclosure_requires_fixed_destination_exact_model_current_frame_and_atomic_grant() {
    use crate::model::{ModelProvider, ModelRequest};
    let wire = MockHttp::start(|_, _, _| panic!("no provider or machine request")).await;
    let mut run = service(&wire).begin(RunId::default());
    let cancel = Cancellation::default();
    run.install(
        crate::computer_png::canonicalize(&fixture()).unwrap(),
        Instant::now(),
        &cancel,
    )
    .unwrap();
    let image = run
        .take_vision(
            ModelCallId::default(),
            "openai_codex_subscription",
            "gpt-5.6-sol",
        )
        .unwrap()
        .unwrap();
    let request = ModelRequest {
        model: "gpt-5.6-sol",
        instructions: "test",
        messages: &[],
        options: Default::default(),
    };
    let provider =
        OpenAiProvider::with_endpoint(Arc::new(Auth), "https://unapproved.example/v1/responses")
            .unwrap();
    assert!(provider.measure_visual_request(&request, &image).is_err());
    let grant = image.approve();
    let wins = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| grant.consume(&image, "gpt-5.6-sol", &cancel).is_ok()))
            .collect();
        handles
            .into_iter()
            .map(|h| usize::from(h.join().unwrap()))
            .sum::<usize>()
    });
    assert_eq!(wins, 1);
    let unused = image.approve();
    cancel.cancel();
    assert!(unused.consume(&image, "gpt-5.6-sol", &cancel).is_err());
    run.invalidate();
    assert!(
        unused
            .consume(&image, "gpt-5.6-sol", &Cancellation::default())
            .is_err()
    );
    assert!(image.data_url().is_err());
    assert!(!supported_model("gpt-5.6-sol-unverified-snapshot"));
    assert!(!supported_model("gpt-4.1"));
}

#[tokio::test]
async fn capture_failures_keep_distinct_safe_categories_without_stderr_or_retries() {
    let normal = json!({"stdout":"","stderr":"private-guest-screen-or-error-content","exitCode":0,"timedOut":false,"truncated":false});
    let cases = [
        (
            401,
            normal.clone(),
            "capture HTTP authentication or permission rejected",
        ),
        (
            403,
            normal.clone(),
            "capture HTTP authentication or permission rejected",
        ),
        (429, normal.clone(), "capture HTTP request rejected"),
        (
            200,
            json!({"unexpected":"private-response-content"}),
            "capture response invalid",
        ),
        (
            200,
            {
                let mut v = normal.clone();
                v["timedOut"] = json!(true);
                v["exitCode"] = json!(124);
                v
            },
            "capture command timed out (2-second limit)",
        ),
        (
            200,
            {
                let mut v = normal.clone();
                v["truncated"] = json!(true);
                v
            },
            "capture output truncated by provider (base64 may exceed stdout limit)",
        ),
        (
            200,
            {
                let mut v = normal.clone();
                v["exitCode"] = json!(127);
                v
            },
            "capture command not found (exit 127)",
        ),
        (
            200,
            {
                let mut v = normal.clone();
                v["exitCode"] = json!(1);
                v
            },
            "capture command exited unsuccessfully",
        ),
        (
            200,
            {
                let mut v = normal.clone();
                v["stdout"] = json!("invalid@@base64-private-content");
                v
            },
            "capture stdout is not valid base64",
        ),
        (
            200,
            {
                let mut v = normal.clone();
                v["stdout"] = json!(STANDARD.encode(vec![0; MAX_PNG + 1]));
                v
            },
            "capture PNG byte limit exceeded",
        ),
        (
            200,
            {
                let mut v = normal.clone();
                v["stderr"] = json!("x".repeat(1_500_001));
                v
            },
            "capture HTTP response byte limit exceeded",
        ),
    ];
    for (status, response, expected) in cases {
        let wire = MockHttp::start(move |n, _, _| {
            assert!(n < 2, "no retry");
            if n == 0 {
                (
                    200,
                    "application/json",
                    json!({"id":ID,"state":"running"}).to_string(),
                )
            } else {
                (status, "application/json", response.to_string())
            }
        })
        .await;
        let mut run = service(&wire).begin(RunId::default());
        let cancel = Cancellation::default();
        let prepared = run
            .prepare(ToolCallId::default(), &observe_call("screenshot"), &cancel)
            .unwrap();
        let permit = prepared.approve().unwrap();
        let ToolExecution::Finished(result) = run.execute(prepared, permit, &cancel).await else {
            panic!("finished")
        };
        let ToolOutcome::Success { data } = result.outcome else {
            panic!("report")
        };
        assert_eq!(data["failure"], expected);
        assert_eq!(data["remote_outcome"], "unknown");
        assert_eq!(data["successful"], false);
        assert!(!data.to_string().contains("private-"));
        assert!(run.frame.lock().unwrap().is_none());
        assert!(run.stopped());
        assert_eq!(wire.requests.lock().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn late_yes_is_expiry_not_denial_and_never_sends_an_image() {
    let (result, events, answers, wire, api, _, _) = execute_case_with_disclosure(
        vec![true, true, true],
        200,
        false,
        None,
        DisclosureFixture::ExpiredYes,
    )
    .await;
    assert!(
        matches!(result.outcome,RunOutcome::Failed{ref code,..} if code=="image_permission_expired")
    );
    assert_eq!(answers.commands.len(), 2);
    assert!(events.iter().any(
        |e| matches!(&e.kind,EventKind::PermissionFailed{message} if message.contains("expired"))
    ));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::PermissionDenied))
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ImageInputAdmitted { .. }))
    );
    assert_eq!(api.requests.lock().unwrap().len(), 1);
    assert_eq!(wire.requests.lock().unwrap().len(), 2);
    assert!(
        !serde_json::to_string(&result.session)
            .unwrap()
            .contains("data:image")
    );
}

#[tokio::test]
async fn disclosure_input_failure_is_not_denial_and_does_not_echo_terminal_error() {
    let (result, events, _, wire, api, _, _) = execute_case_with_disclosure(
        vec![true, true, true],
        200,
        false,
        None,
        DisclosureFixture::InputError,
    )
    .await;
    assert!(
        matches!(result.outcome,RunOutcome::Failed{ref code,..} if code=="image_permission_failed")
    );
    assert!(events.iter().any(|e|matches!(&e.kind,EventKind::PermissionFailed{message} if message.contains("input unavailable"))));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::PermissionDenied))
    );
    assert!(
        !serde_json::to_string(&events)
            .unwrap()
            .contains("private terminal error")
    );
    assert_eq!(api.requests.lock().unwrap().len(), 1);
    assert_eq!(wire.requests.lock().unwrap().len(), 2);
}

#[path = "visual_response_tests.rs"]
mod visual_response;
