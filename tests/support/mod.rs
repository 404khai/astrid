// Different integration test binaries use different subsets of these helpers.
#![allow(dead_code)]
use astrid::{
    auth::{AuthError, Authentication, BearerToken},
    cancellation::Cancellation,
    events::{EventKind, ExecutionEvent},
    model::{ToolCall, ToolResult},
    openai::OpenAiProvider,
    runtime::{self, PermissionHandler, RunConfig, RunOutcome, RunResult},
    tools::{PermissionRequest, ToolExecution, ToolExecutor},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{
    io,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

pub struct TestAuth;
#[async_trait]
impl Authentication for TestAuth {
    async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
        BearerToken::new("test-only-credential".into())
    }
}

pub struct Confirmation {
    pub approved: bool,
    pub commands: Vec<String>,
}
impl Confirmation {
    pub fn new(approved: bool) -> Self {
        Self {
            approved,
            commands: Vec::new(),
        }
    }
}
#[async_trait]
impl PermissionHandler for Confirmation {
    async fn decide(&mut self, request: &PermissionRequest) -> io::Result<bool> {
        self.commands.push(request.command.clone());
        Ok(self.approved)
    }
}

#[derive(Default)]
pub struct Recording {
    pub events: Vec<ExecutionEvent>,
    pub observations: Vec<String>,
    pub results: Vec<ToolResult>,
    pub text: String,
}
impl Recording {
    pub fn record(&mut self, event: ExecutionEvent) {
        match &event.kind {
            EventKind::ModelTextDelta { text } => {
                self.text.push_str(text);
                self.observations.push("text".into());
            }
            EventKind::ModelCallStarted { number } => {
                self.observations.push(format!("model {number} started"))
            }
            EventKind::ModelCallCompleted { .. } => {
                let number = self
                    .events
                    .iter()
                    .filter(|e| matches!(e.kind, EventKind::ModelCallStarted { .. }))
                    .count();
                self.observations.push(format!("model {number} completed"));
            }
            EventKind::ToolCallStarted => {
                let requested = self
                    .events
                    .iter()
                    .find_map(|e| {
                        if e.tool_call_id == event.tool_call_id {
                            if let EventKind::ToolCallRequested { call } = &e.kind {
                                Some(call)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap();
                self.observations
                    .push(format!("tool {} started", requested.call_id));
            }
            EventKind::ToolCallCompleted { outcome }
            | EventKind::ToolCallFailed { outcome }
            | EventKind::ToolCallDenied { outcome }
            | EventKind::ToolCallTimedOut { outcome } => {
                let requested = self
                    .events
                    .iter()
                    .find_map(|e| {
                        if e.tool_call_id == event.tool_call_id {
                            if let EventKind::ToolCallRequested { call } = &e.kind {
                                Some(call)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap();
                self.observations
                    .push(format!("tool {} completed", requested.call_id));
                self.results.push(ToolResult {
                    call_id: requested.call_id.clone(),
                    name: requested.name.clone(),
                    outcome: outcome.clone(),
                });
            }
            _ => {}
        }
        self.events.push(event);
    }
}
#[derive(Debug)]
pub enum RunError {
    Runtime(RunOutcome),
    Configuration,
}
#[allow(clippy::too_many_arguments)]
pub async fn run(
    provider: &dyn astrid::model::ModelProvider,
    tools: &astrid::tools::Tools,
    confirmation: &mut Confirmation,
    recording: &mut Recording,
    model: &str,
    task: &str,
    max_model_calls: usize,
) -> Result<RunResult, RunError> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    let execution = runtime::run(
        provider,
        tools,
        confirmation,
        RunConfig {
            observability: None,
            context_budget: None,
            model: model.into(),
            task: task.into(),
            max_model_calls,
            permissions: Default::default(),
        },
        Cancellation::default(),
        Some(sender),
    );
    let consume = async {
        while let Some(event) = receiver.recv().await {
            recording.record(event);
        }
    };
    let (result, ()) = tokio::join!(execution, consume);
    let result = result.map_err(|_| RunError::Configuration)?;
    if result.outcome == RunOutcome::Completed {
        Ok(result)
    } else {
        Err(RunError::Runtime(result.outcome))
    }
}

// Direct tool tests use a test-only caller that supplies permission decisions.
// Production approval orchestration exists exclusively in runtime.rs.
pub struct CheckedTools(astrid::tools::Tools);
impl CheckedTools {
    pub fn workspace(&self) -> &astrid::workspace::Workspace {
        self.0.workspace()
    }
    pub fn new(
        workspace: astrid::workspace::Workspace,
        timeout: std::time::Duration,
    ) -> Result<Self, astrid::tools::ToolError> {
        astrid::tools::Tools::new(workspace, timeout).map(Self)
    }
    pub async fn execute(&self, call: &ToolCall, confirmation: &mut Confirmation) -> ToolResult {
        if let Some(request) = self.0.permission(call).unwrap()
            && !confirmation.decide(&request).await.unwrap()
        {
            return ToolResult {
                call_id: call.call_id.clone(),
                name: call.name.clone(),
                outcome: astrid::model::ToolOutcome::Error {
                    code: "permission_denied".into(),
                    message: "denied".into(),
                },
            };
        }
        match self.0.execute(call, &Cancellation::default()).await {
            ToolExecution::Finished(result) => result,
            other => panic!("unexpected tool execution: {other:?}"),
        }
    }
}

pub fn call(id: &str, name: &str, args: Value) -> ToolCall {
    ToolCall {
        call_id: id.into(),
        name: name.into(),
        arguments: args.to_string(),
    }
}
pub fn item(call: &ToolCall) -> Value {
    json!({"type":"function_call","id":format!("item_{}",call.call_id),"call_id":call.call_id,"name":call.name,"namespace":"astrid","arguments":call.arguments,"status":"completed"})
}
pub fn message(text: &str) -> Value {
    json!({"type":"message","id":"msg_test","role":"assistant","status":"completed","content":[{"type":"output_text","text":text,"annotations":[]}]})
}
pub fn completed(output: Vec<Value>) -> Value {
    json!({"type":"response.completed","response":{"id":"resp_test","status":"completed","output":output}})
}
pub fn reply(calls: &[ToolCall], text: &str) -> Vec<Value> {
    let mut events = Vec::new();
    let mut output = Vec::new();
    if !text.is_empty() {
        events.push(json!({"type":"response.output_text.delta","delta":text}));
        output.push(message(text));
    }
    for (index, call) in calls.iter().enumerate() {
        let mut added = item(call);
        added["arguments"] = json!("");
        added["status"] = json!("in_progress");
        events.push(json!({"type":"response.output_item.added","output_index":index,"item":added}));
        // Unicode-safe splits exercise fragmented argument assembly.
        for fragment in call.arguments.chars().collect::<Vec<_>>().chunks(3) {
            events.push(json!({"type":"response.function_call_arguments.delta","item_id":format!("item_{}",call.call_id),"delta":fragment.iter().collect::<String>()}));
        }
        events.push(json!({"type":"response.function_call_arguments.done","item_id":format!("item_{}",call.call_id),"arguments":call.arguments}));
        events.push(
            json!({"type":"response.output_item.done","output_index":index,"item":item(call)}),
        );
        output.push(item(call));
    }
    events.push(completed(output));
    events
}

pub struct Server {
    pub provider: OpenAiProvider,
    pub requests: Arc<Mutex<Vec<Value>>>,
    pub request_bytes: Arc<Mutex<Vec<usize>>>,
    task: JoinHandle<()>,
}
impl Server {
    pub async fn start(replies: Vec<Vec<Value>>) -> Self {
        Self::http(200, replies).await
    }
    pub async fn http(status: u16, replies: Vec<Vec<Value>>) -> Self {
        Self::with_content_type(status, replies, Some("text/event-stream")).await
    }
    pub async fn with_content_type(
        status: u16,
        replies: Vec<Vec<Value>>,
        content_type: Option<&str>,
    ) -> Self {
        let content_type = content_type.map(str::to_owned);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/v1/responses", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let request_bytes = Arc::new(Mutex::new(Vec::new()));
        let captured_bytes = request_bytes.clone();
        let task = tokio::spawn(async move {
            for events in replies {
                let (mut connection, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 4096];
                let header_end = loop {
                    let size = connection.read(&mut buffer).await.unwrap();
                    assert_ne!(size, 0);
                    request.extend_from_slice(&buffer[..size]);
                    if let Some(index) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                        break index + 4;
                    }
                };
                let headers = String::from_utf8(request[..header_end].to_vec())
                    .unwrap()
                    .to_lowercase();
                assert!(headers.starts_with("post /v1/responses "));
                assert!(headers.contains("authorization: bearer test-only-credential"));
                assert!(headers.contains("accept: text/event-stream"));
                let length = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                while request.len() < header_end + length {
                    let size = connection.read(&mut buffer).await.unwrap();
                    assert_ne!(size, 0);
                    request.extend_from_slice(&buffer[..size]);
                }
                captured_bytes.lock().unwrap().push(length);
                captured.lock().unwrap().push(
                    serde_json::from_slice(&request[header_end..header_end + length]).unwrap(),
                );
                let body = if status == 200 {
                    events
                        .into_iter()
                        .map(|event| format!("data: {event}\r\n\r\n"))
                        .collect::<String>()
                } else {
                    json!({"error":{"message":"test provider rejection"}}).to_string()
                };
                let kind = if status == 200 {
                    content_type.as_deref().unwrap_or("")
                } else {
                    "application/json"
                };
                let mime = if kind.is_empty() {
                    String::new()
                } else {
                    format!("Content-Type: {kind}\r\n")
                };
                let headers = format!(
                    "HTTP/1.1 {status} Test\r\n{mime}Content-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                connection.write_all(headers.as_bytes()).await.unwrap();
                for bytes in body.as_bytes().chunks(3) {
                    if connection.write_all(bytes).await.is_err() {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            }
        });
        Self {
            provider: OpenAiProvider::with_endpoint(Arc::new(TestAuth), endpoint).unwrap(),
            requests,
            request_bytes,
            task,
        }
    }
    pub async fn finish(self) -> Vec<Value> {
        self.task.await.unwrap();
        Arc::try_unwrap(self.requests)
            .unwrap()
            .into_inner()
            .unwrap()
    }
}
