//! Anthropic Messages API adapter (ADR 0030). Wire formats and replay data stay
//! inside this adapter; the runtime sees the same ModelResponse as for OpenAI.
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::{collections::HashSet, time::Duration};

use crate::{
    context::{ContextMeasurement, ContextSnapshot, ContextSource},
    model::{Message, ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink, ToolCall},
    openai::ResponseContinuation,
    routing::{ModelCapability, ReasoningEffort, ReplayMode},
    tools,
};

const VERSION: &str = "2023-06-01";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// Opaque continuation item holding Claude's raw assistant content blocks.
const OPAQUE: &str = "anthropic_content";
const MAX_TOKENS: u64 = 64_000;
const MAX_EVENT: usize = 4 * 1024 * 1024;

/// Exact model identifiers this adapter accepts; never prefix-matched.
pub fn supported_model(model: &str) -> bool {
    matches!(
        model,
        "claude-opus-5-5"
            | "claude-sonnet-5-5"
            | "claude-fable-5-1"
            | "claude-opus-5"
            | "claude-haiku-4-5"
    )
}

/// An API key from the Claude Console. Debug never prints it.
#[derive(Clone)]
pub struct ApiKey(String);
impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiKey([private])")
    }
}
impl ApiKey {
    pub fn new(key: String) -> Result<Self, String> {
        let key = key.trim().to_owned();
        if key.len() < 20
            || key.len() > 512
            || !key.chars().all(|c| c.is_ascii_graphic())
            || key.contains('"')
            || key.contains('\\')
        {
            return Err("that does not look like an Anthropic API key".into());
        }
        Ok(Self(key))
    }
    /// `ANTHROPIC_API_KEY` from the environment, else the Keychain item from
    /// `astrid claude login`. Never reads .env files.
    pub fn load() -> Result<Self, String> {
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.trim().is_empty() {
                return Self::new(key);
            }
        }
        keychain::read()?.map(Self::new).transpose()?.ok_or_else(|| {
            "no Claude API key: run `astrid claude login` (Console API key) or set ANTHROPIC_API_KEY".into()
        })
    }
}

pub struct ClaudeProvider {
    client: reqwest::Client,
    key: ApiKey,
    endpoint: String,
}

impl ClaudeProvider {
    pub fn new(key: ApiKey) -> Result<Self, ModelError> {
        Self::with_endpoint(key, "https://api.anthropic.com/v1/messages")
    }
    /// Transport seam for local protocol tests.
    pub fn with_endpoint(key: ApiKey, endpoint: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent(concat!("Astrid/", env!("CARGO_PKG_VERSION")))
                .retry(reqwest::retry::never())
                .connect_timeout(Duration::from_secs(30))
                .timeout(Duration::from_secs(900))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            key,
            endpoint: endpoint.into(),
        })
    }

    fn model_capability(model: &str) -> ModelCapability {
        use ReasoningEffort::*;
        ModelCapability {
            visible_history: true,
            reasoning_efforts: match model {
                // Thinking is always on for these; effort is the only depth control.
                "claude-opus-5-5" | "claude-fable-5-1" | "claude-sonnet-5-5" | "claude-opus-5" => {
                    vec![Low, Medium, High, Xhigh, Max]
                }
                _ => vec![],
            },
        }
    }

    fn uses_fallbacks(model: &str) -> bool {
        matches!(
            model,
            "claude-opus-5-5" | "claude-fable-5-1" | "claude-sonnet-5-5" | "claude-opus-5"
        )
    }

    /// Converts Astrid's typed history to alternating Messages API turns.
    pub fn body(request: &ModelRequest<'_>) -> Result<Value, ModelError> {
        if !supported_model(request.model) {
            return Err(ModelError::Provider(format!(
                "unsupported Claude model {:?}",
                request.model
            )));
        }
        if request.options.effort.is_some_and(|e| {
            !Self::model_capability(request.model)
                .reasoning_efforts
                .contains(&e)
        }) {
            return Err(ModelError::Provider(
                "explicit reasoning effort is unsupported for this model".into(),
            ));
        }
        let mut messages: Vec<Value> = Vec::new();
        let mut push = |role: &str, blocks: Vec<Value>| {
            if blocks.is_empty() {
                return;
            }
            match messages.last_mut() {
                Some(last) if last["role"] == role => last["content"]
                    .as_array_mut()
                    .expect("content array")
                    .extend(blocks),
                _ => messages.push(json!({"role":role,"content":blocks})),
            }
        };
        for message in request.messages {
            match message {
                Message::User(text) | Message::Summary(text) => {
                    push("user", vec![json!({"type":"text","text":text})])
                }
                Message::Assistant(response) => {
                    let native = request.options.replay != ReplayMode::VisibleHistory;
                    let raw = response
                        .continuation
                        .opaque(OPAQUE)
                        .and_then(|v| v["content"].as_array())
                        .filter(|_| native);
                    let blocks = match raw {
                        Some(blocks) => blocks.clone(),
                        None => {
                            let mut blocks = vec![];
                            if !response.text.is_empty() {
                                blocks.push(json!({"type":"text","text":response.text}));
                            }
                            for call in &response.tool_calls {
                                let input: Value = serde_json::from_str(&call.arguments)
                                    .unwrap_or_else(|_| json!({}));
                                blocks.push(json!({"type":"tool_use","id":call.call_id,"name":call.name,"input":input}));
                            }
                            blocks
                        }
                    };
                    push("assistant", blocks);
                }
                Message::Tool(result) => push(
                    "user",
                    vec![
                        json!({"type":"tool_result","tool_use_id":result.call_id,"is_error":result.is_error(),
                        "content":serde_json::to_string(&result.outcome).expect("JSON tool outcome is serializable")}),
                    ],
                ),
            }
        }
        let mut definitions = tools::definitions();
        if request.options.instabox_exec {
            definitions.push(crate::remote::definition());
        }
        if request.options.computer_use {
            return Err(ModelError::Provider(
                "screen tools are not yet supported on the Claude adapter".into(),
            ));
        }
        let mail: Vec<&str> = [
            (request.options.mail_apple, "apple"),
            (request.options.mail_gmail, "gmail"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect();
        if !mail.is_empty() {
            definitions.extend(crate::mail::definitions(&mail));
        }
        let tools: Vec<Value> = definitions
            .iter()
            .map(|d| json!({"name":d["name"],"description":d["description"],"input_schema":d["parameters"],"eager_input_streaming":true}))
            .collect();
        let mut body = json!({
            "model": request.model,
            "max_tokens": MAX_TOKENS,
            "stream": true,
            "system": request.instructions,
            "messages": messages,
            "tools": tools,
            "cache_control": {"type":"ephemeral"},
        });
        if Self::uses_fallbacks(request.model) {
            body["fallbacks"] = json!("default");
        }
        if let Some(effort) = request.options.effort {
            body["output_config"] = json!({"effort": effort});
        }
        Ok(body)
    }

    fn snapshot(body: &Value) -> ContextSnapshot {
        use ContextSource::*;
        let sources = [
            Instructions,
            UserMessages,
            AssistantContinuation,
            ToolResults,
            ToolDefinitions,
            RequestFraming,
            Summary,
        ];
        let mut m = sources.map(|source| ContextMeasurement {
            source,
            entries: 0,
            serialized_bytes: 0,
            contains_opaque_data: false,
        });
        m[0].entries = 1;
        m[0].serialized_bytes = bytes(&body["system"]);
        m[4].entries = 1;
        m[4].serialized_bytes = bytes(&body["tools"]);
        for message in body["messages"].as_array().into_iter().flatten() {
            for block in message["content"].as_array().into_iter().flatten() {
                let index = match (message["role"].as_str(), block["type"].as_str()) {
                    (Some("user"), Some("tool_result")) => 3,
                    (Some("user"), _) => 1,
                    _ => 2,
                };
                m[index].entries += 1;
                m[index].serialized_bytes += bytes(block);
                m[index].contains_opaque_data |= matches!(
                    block["type"].as_str(),
                    Some("thinking" | "redacted_thinking")
                );
            }
        }
        let total = bytes(body);
        m[5].entries = 1;
        m[5].serialized_bytes = total
            - m.iter()
                .enumerate()
                .filter(|(i, _)| *i != 5)
                .map(|(_, x)| x.serialized_bytes)
                .sum::<usize>();
        let non_opaque = m
            .iter()
            .filter(|x| !x.contains_opaque_data)
            .map(|x| x.serialized_bytes)
            .sum::<usize>();
        ContextSnapshot {
            measurements: m.into(),
            serialized_request_bytes: total,
            non_opaque_json_size_token_heuristic: non_opaque.div_ceil(4),
            provider_input_tokens: None,
        }
    }
}

fn bytes(value: &Value) -> usize {
    serde_json::to_vec(value).map(|v| v.len()).unwrap_or(0)
}

#[async_trait]
impl ModelProvider for ClaudeProvider {
    fn capabilities(&self, model: &str) -> ModelCapability {
        Self::model_capability(model)
    }
    fn provider_name(&self) -> Option<&'static str> {
        Some("anthropic_api")
    }
    fn measure_request(
        &self,
        request: &ModelRequest<'_>,
    ) -> Result<Option<ContextSnapshot>, ModelError> {
        Ok(Some(Self::snapshot(&Self::body(request)?)))
    }

    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        text: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        let body = Self::body(request)?;
        text.request_prepared(Self::snapshot(&body)).await?;
        let mut call = self
            .client
            .post(&self.endpoint)
            .header("x-api-key", &self.key.0)
            .header("anthropic-version", VERSION)
            .header(reqwest::header::ACCEPT, "text/event-stream");
        if body.get("fallbacks").is_some() {
            call = call.header("anthropic-beta", FALLBACK_BETA);
        }
        let response = call.json(&body).send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let raw = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
                .unwrap_or_else(|| "no structured provider error".into());
            return Err(ModelError::Http {
                status,
                message: message.chars().take(2000).collect(),
            });
        }
        let mut stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();
        let mut assembly = Assembly::default();
        while let Some(chunk) = stream.next().await {
            buffer.extend_from_slice(&chunk?);
            if buffer.len() > MAX_EVENT {
                return Err(ModelError::Protocol("stream event exceeds 4 MiB".into()));
            }
            while let Some(end) = find_event_end(&buffer) {
                let event: Vec<u8> = buffer.drain(..end.0).collect();
                buffer.drain(..end.1);
                if let Some(data) = sse_data(&event) {
                    let value: Value = serde_json::from_str(&data)
                        .map_err(|_| ModelError::Protocol("invalid stream JSON".into()))?;
                    if let Some(done) = assembly.event(value, text).await? {
                        return Ok(done);
                    }
                }
            }
        }
        Err(ModelError::Protocol(
            "stream ended before message_stop".into(),
        ))
    }
}

/// Index of the end of the first SSE event, and the separator length.
fn find_event_end(buffer: &[u8]) -> Option<(usize, usize)> {
    let lf = buffer.windows(2).position(|w| w == b"\n\n").map(|i| (i, 2));
    let crlf = buffer
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| (i, 4));
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (a, b) => a.or(b),
    }
}

fn sse_data(event: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(event);
    let data: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .map(|l| l.strip_prefix(' ').unwrap_or(l))
        .collect();
    (!data.is_empty()).then(|| data.join("\n"))
}

/// Assembles streamed content blocks into one completed response.
#[derive(Default)]
struct Assembly {
    blocks: Vec<Value>,
    partial_json: Vec<String>,
    stop_reason: Option<String>,
}

impl Assembly {
    async fn event(
        &mut self,
        event: Value,
        text: &mut dyn TextSink,
    ) -> Result<Option<ModelResponse>, ModelError> {
        let index = || {
            event["index"]
                .as_u64()
                .map(|i| i as usize)
                .ok_or_else(|| ModelError::Protocol("missing block index".into()))
        };
        match event["type"].as_str().unwrap_or("") {
            "message_start" | "ping" | "content_block_stop" => {}
            "content_block_start" => {
                let i = index()?;
                if i != self.blocks.len() {
                    return Err(ModelError::Protocol("noncontiguous content block".into()));
                }
                self.blocks.push(event["content_block"].clone());
                self.partial_json.push(String::new());
            }
            "content_block_delta" => {
                let i = index()?;
                let block = self
                    .blocks
                    .get_mut(i)
                    .ok_or_else(|| ModelError::Protocol("delta for unknown block".into()))?;
                let delta = &event["delta"];
                match delta["type"].as_str().unwrap_or("") {
                    "text_delta" => {
                        let piece = delta["text"].as_str().unwrap_or("");
                        let current = block["text"].as_str().unwrap_or("").to_owned() + piece;
                        block["text"] = json!(current);
                        text.delta(piece).await?;
                    }
                    "input_json_delta" => {
                        self.partial_json[i].push_str(delta["partial_json"].as_str().unwrap_or(""))
                    }
                    "thinking_delta" => {
                        let current = block["thinking"].as_str().unwrap_or("").to_owned()
                            + delta["thinking"].as_str().unwrap_or("");
                        block["thinking"] = json!(current);
                    }
                    "signature_delta" => block["signature"] = delta["signature"].clone(),
                    _ => {}
                }
            }
            "message_delta" => {
                if let Some(reason) = event["delta"]["stop_reason"].as_str() {
                    self.stop_reason = Some(reason.to_owned());
                }
            }
            "error" => {
                let message = event["error"]["message"].as_str().unwrap_or("stream error");
                return Err(ModelError::Provider(message.chars().take(2000).collect()));
            }
            "message_stop" => return self.finish().map(Some),
            _ => {}
        }
        Ok(None)
    }

    fn finish(&mut self) -> Result<ModelResponse, ModelError> {
        match self.stop_reason.as_deref() {
            Some("end_turn" | "tool_use" | "stop_sequence") => {}
            Some("refusal") => {
                return Err(ModelError::Provider(
                    "Claude declined this request (refusal)".into(),
                ));
            }
            Some("max_tokens") => {
                return Err(ModelError::Provider(
                    "response hit max_tokens before completing".into(),
                ));
            }
            other => {
                return Err(ModelError::Protocol(format!(
                    "unsupported stop reason {other:?}"
                )));
            }
        }
        let mut text = String::new();
        let mut calls = Vec::new();
        let mut ids = HashSet::new();
        for (block, partial) in self.blocks.iter_mut().zip(&self.partial_json) {
            match block["type"].as_str().unwrap_or("") {
                "text" => text.push_str(block["text"].as_str().unwrap_or("")),
                "tool_use" => {
                    // Eager input streaming: the client validates the accumulated JSON.
                    let raw = if partial.is_empty() {
                        block["input"].to_string()
                    } else {
                        partial.clone()
                    };
                    let input: Value = serde_json::from_str(&raw)
                        .map_err(|_| ModelError::Protocol("tool input is not valid JSON".into()))?;
                    if !input.is_object() {
                        return Err(ModelError::Protocol("tool input must be an object".into()));
                    }
                    block["input"] = input.clone();
                    let id = block["id"].as_str().unwrap_or("").to_owned();
                    let name = block["name"].as_str().unwrap_or("").to_owned();
                    if id.is_empty() || name.is_empty() || !ids.insert(id.clone()) {
                        return Err(ModelError::Protocol(
                            "empty or duplicated tool_use id".into(),
                        ));
                    }
                    calls.push(ToolCall {
                        call_id: id,
                        name,
                        arguments: input.to_string(),
                    });
                }
                _ => {} // thinking, redacted_thinking, fallback: kept verbatim for replay
            }
        }
        let opaque = json!({"type": OPAQUE, "content": self.blocks});
        Ok(ModelResponse {
            continuation: ResponseContinuation::from_parts(&text, &calls, Some(opaque)),
            text,
            tool_calls: calls,
        })
    }
}

/// API key storage in the login Keychain, written via `security -i` on stdin.
pub mod keychain {
    use std::io::Write;
    const SERVICE: &str = "dev.astrid.anthropic";
    const ACCOUNT: &str = "api-key";

    pub fn store(key: &super::ApiKey) -> Result<(), String> {
        let mut child = std::process::Command::new("/usr/bin/security")
            .arg("-i")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| "Keychain unavailable")?;
        let script = format!(
            "add-generic-password -U -s {SERVICE} -a {ACCOUNT} -l \"Astrid Claude API key\" -w \"{}\"\n",
            key.0
        );
        child
            .stdin
            .take()
            .ok_or("Keychain unavailable")?
            .write_all(script.as_bytes())
            .map_err(|_| "Keychain write failed")?;
        if child.wait().map_err(|_| "Keychain write failed")?.success() {
            Ok(())
        } else {
            Err("Keychain write failed".into())
        }
    }
    pub fn read() -> Result<Option<String>, String> {
        let output = std::process::Command::new("/usr/bin/security")
            .args(["find-generic-password", "-s", SERVICE, "-a", ACCOUNT, "-w"])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .map_err(|_| "Keychain unavailable")?;
        Ok(output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .filter(|k| !k.is_empty()))
    }
    pub fn delete() -> bool {
        std::process::Command::new("/usr/bin/security")
            .args(["delete-generic-password", "-s", SERVICE, "-a", ACCOUNT])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
}

#[cfg(test)]
#[path = "anthropic_tests.rs"]
mod tests;
