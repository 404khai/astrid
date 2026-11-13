//! OpenCode Go adapter (ADR 0031): OpenAI-compatible Chat Completions for the
//! catalog models that use that API. Selected with `--model opencode/<id>`.
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, time::Duration};

use crate::{
    context::{ContextMeasurement, ContextSnapshot, ContextSource},
    model::{Message, ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink, ToolCall},
    openai::ResponseContinuation,
    routing::{ModelCapability, ReasoningEffort, ReplayMode},
    tools,
};

pub const PREFIX: &str = "opencode/";
const CATALOG: &str = "https://opencode.ai/inference/go/v1/models";
const BASE: &str = "https://opencode.ai/inference/go/openai/v1";
/// Opaque continuation item holding the raw assistant message (with reasoning).
const OPAQUE: &str = "opencode_chat";
const MAX_EVENT: usize = 4 * 1024 * 1024;

/// One catalog model usable through Chat Completions with tools.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub efforts: Vec<String>,
    pub reasoning_field: Option<String>,
}

/// Parses the public catalog, keeping only Chat Completions models with tools.
pub fn parse_catalog(catalog: &Value) -> Vec<ModelInfo> {
    catalog["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["capabilities"]["tools"]["supported"] == true)
        .filter_map(|m| {
            let api = m["api"].get("chat-completions")?;
            Some(ModelInfo {
                id: m["id"]
                    .as_str()
                    .filter(|id| {
                        id.len() <= 128
                            && id
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || "-._".contains(c))
                    })?
                    .to_owned(),
                efforts: m["capabilities"]["reasoning"]["effort"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|e| e.as_str().map(str::to_owned))
                    .collect(),
                reasoning_field: api["compatibility"]["reasoning_field"]
                    .as_str()
                    .map(str::to_owned),
            })
        })
        .collect()
}

/// Fetches the public catalog (no key) and keeps the usable models.
pub async fn catalog() -> Result<Vec<ModelInfo>, String> {
    let catalog: Value = reqwest::Client::builder()
        .user_agent(concat!("Astrid/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| "HTTP client unavailable")?
        .get(CATALOG)
        .send()
        .await
        .map_err(|_| "OpenCode model catalog unreachable")?
        .json()
        .await
        .map_err(|_| "OpenCode model catalog invalid")?;
    Ok(parse_catalog(&catalog))
}

/// Finds one usable model in the live catalog.
pub async fn lookup(id: &str) -> Result<ModelInfo, String> {
    let models = catalog().await?;
    models.iter().find(|m| m.id == id).cloned().ok_or_else(|| {
        format!(
            "OpenCode model {id:?} is not available through Chat Completions with tools; see `astrid opencode models`"
        )
    })
}

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
        if key.len() < 16
            || key.len() > 512
            || !key.chars().all(|c| c.is_ascii_graphic())
            || key.contains('"')
            || key.contains('\\')
        {
            return Err("that does not look like an OpenCode API key".into());
        }
        Ok(Self(key))
    }
    /// `OPENCODE_API_KEY`, else the Keychain item from `astrid opencode login`.
    pub fn load() -> Result<Self, String> {
        if let Ok(key) = std::env::var("OPENCODE_API_KEY") {
            if !key.trim().is_empty() {
                return Self::new(key);
            }
        }
        crate::keychain::read(SERVICE)?
            .map(Self::new)
            .transpose()?
            .ok_or_else(|| {
                "no OpenCode key: run `astrid opencode login` or set OPENCODE_API_KEY".into()
            })
    }
    pub fn store(&self) -> Result<(), String> {
        crate::keychain::store(SERVICE, "Astrid OpenCode Go API key", &self.0)
    }
}
pub const SERVICE: &str = "dev.astrid.opencode";

pub struct OpenCodeProvider {
    client: reqwest::Client,
    key: ApiKey,
    model: ModelInfo,
    endpoint: String,
}

fn effort_name(effort: ReasoningEffort) -> &'static str {
    use ReasoningEffort::*;
    match effort {
        Disabled => "none",
        Minimal => "minimal",
        Low => "low",
        Medium => "medium",
        High => "high",
        Xhigh => "xhigh",
        Max => "max",
    }
}

impl OpenCodeProvider {
    pub fn new(key: ApiKey, model: ModelInfo) -> Result<Self, ModelError> {
        Self::with_endpoint(key, model, format!("{BASE}/chat/completions"))
    }
    pub fn with_endpoint(
        key: ApiKey,
        model: ModelInfo,
        endpoint: impl Into<String>,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            client: reqwest::Client::builder()
                // OpenCode asks third-party clients to identify themselves.
                .user_agent(concat!("Astrid/", env!("CARGO_PKG_VERSION")))
                .retry(reqwest::retry::never())
                .connect_timeout(Duration::from_secs(30))
                .timeout(Duration::from_secs(900))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            key,
            model,
            endpoint: endpoint.into(),
        })
    }

    fn capability(&self) -> ModelCapability {
        use ReasoningEffort::*;
        ModelCapability {
            visible_history: true,
            reasoning_efforts: [Disabled, Minimal, Low, Medium, High, Xhigh, Max]
                .into_iter()
                .filter(|e| self.model.efforts.iter().any(|n| n == effort_name(*e)))
                .collect(),
        }
    }

    /// The model id as Astrid names it, e.g. `opencode/glm-5.3`.
    fn full_name(&self) -> String {
        format!("{PREFIX}{}", self.model.id)
    }

    pub fn body(&self, request: &ModelRequest<'_>) -> Result<Value, ModelError> {
        if request.model != self.full_name() {
            return Err(ModelError::Provider(format!(
                "this provider serves {}, not {}",
                self.full_name(),
                request.model
            )));
        }
        if request.options.computer_use || request.options.look {
            return Err(ModelError::Provider(
                "screen tools are not supported on OpenCode models".into(),
            ));
        }
        let field = self.model.reasoning_field.as_deref();
        let native = request.options.replay != ReplayMode::VisibleHistory;
        let mut messages = vec![json!({"role":"system","content":request.instructions})];
        for message in request.messages {
            match message {
                Message::User(text) | Message::Summary(text) => messages.push(json!({"role":"user","content":text})),
                Message::Assistant(response) => {
                    let raw = response.continuation.opaque(OPAQUE).map(|v| v["message"].clone()).filter(|_| native);
                    messages.push(raw.unwrap_or_else(|| {
                        let mut m = json!({"role":"assistant","content":if response.text.is_empty() { Value::Null } else { json!(response.text) }});
                        if !response.tool_calls.is_empty() {
                            m["tool_calls"] = response
                                .tool_calls
                                .iter()
                                .map(|c| json!({"id":c.call_id,"type":"function","function":{"name":c.name,"arguments":c.arguments}}))
                                .collect();
                        }
                        m
                    }));
                }
                Message::Tool(result) => messages.push(json!({"role":"tool","tool_call_id":result.call_id,
                    "content":serde_json::to_string(&result.outcome).expect("JSON tool outcome is serializable")})),
            }
        }
        if !native {
            // Reasoning from other providers or earlier sessions is never invented.
            if let Some(field) = field {
                for m in &mut messages {
                    if let Some(o) = m.as_object_mut() {
                        o.remove(field);
                    }
                }
            }
        }
        let mut definitions = tools::definitions();
        if request.options.instabox_exec {
            definitions.push(crate::remote::definition());
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
            .map(|d| json!({"type":"function","function":{"name":d["name"],"description":d["description"],"parameters":d["parameters"]}}))
            .collect();
        let mut body = json!({"model":self.model.id,"messages":messages,"tools":tools,"stream":true,"stream_options":{"include_usage":true}});
        if let Some(effort) = request.options.effort {
            if !self.capability().reasoning_efforts.contains(&effort) {
                return Err(ModelError::Provider(
                    "explicit reasoning effort is unsupported for this model".into(),
                ));
            }
            body["reasoning_effort"] = json!(effort_name(effort));
        }
        Ok(body)
    }

    /// Stable per conversation: derived from the instructions and first message.
    pub fn session_id(request: &ModelRequest<'_>) -> String {
        let mut hash = Sha256::new();
        hash.update(request.model.as_bytes());
        hash.update(request.instructions.as_bytes());
        if let Some(Message::User(first) | Message::Summary(first)) = request.messages.first() {
            hash.update(first.as_bytes());
        }
        let digest = hash.finalize();
        format!(
            "astrid-{}",
            digest
                .iter()
                .take(16)
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
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
        m[4].entries = 1;
        m[4].serialized_bytes = bytes(&body["tools"]);
        for message in body["messages"].as_array().into_iter().flatten() {
            let index = match message["role"].as_str() {
                Some("system") => 0,
                Some("user") => 1,
                Some("tool") => 3,
                _ => 2,
            };
            m[index].entries += 1;
            m[index].serialized_bytes += bytes(message);
        }
        let total = bytes(body);
        m[5].entries = 1;
        m[5].serialized_bytes = total
            - m.iter()
                .enumerate()
                .filter(|(i, _)| *i != 5)
                .map(|(_, x)| x.serialized_bytes)
                .sum::<usize>();
        ContextSnapshot {
            measurements: m.into(),
            serialized_request_bytes: total,
            non_opaque_json_size_token_heuristic: total.div_ceil(4),
            provider_input_tokens: None,
        }
    }
}

fn bytes(value: &Value) -> usize {
    serde_json::to_vec(value).map(|v| v.len()).unwrap_or(0)
}

#[async_trait]
impl ModelProvider for OpenCodeProvider {
    fn capabilities(&self, _model: &str) -> ModelCapability {
        self.capability()
    }
    fn provider_name(&self) -> Option<&'static str> {
        Some("opencode_go")
    }
    fn measure_request(
        &self,
        request: &ModelRequest<'_>,
    ) -> Result<Option<ContextSnapshot>, ModelError> {
        Ok(Some(Self::snapshot(&self.body(request)?)))
    }

    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        text: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        let body = self.body(request)?;
        text.request_prepared(Self::snapshot(&body)).await?;
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.key.0)
            .header("x-opencode-session", Self::session_id(request))
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let raw = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|v| {
                    v["error"]["message"]
                        .as_str()
                        .or(v["message"].as_str())
                        .map(str::to_owned)
                })
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
            while let Some(newline) = buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buffer.drain(..=newline).collect();
                let line = String::from_utf8_lossy(&line);
                let Some(data) = line.trim_end().strip_prefix("data:").map(str::trim) else {
                    continue;
                };
                if data == "[DONE]" {
                    return assembly.finish(self.model.reasoning_field.as_deref());
                }
                let value: Value = serde_json::from_str(data)
                    .map_err(|_| ModelError::Protocol("invalid stream JSON".into()))?;
                assembly
                    .chunk(&value, self.model.reasoning_field.as_deref(), text)
                    .await?;
            }
        }
        // Some servers close without [DONE] after a finish_reason.
        if assembly.finish_reason.is_some() {
            return assembly.finish(self.model.reasoning_field.as_deref());
        }
        Err(ModelError::Protocol(
            "stream ended before completion".into(),
        ))
    }
}

#[derive(Default)]
struct PartialCall {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Default)]
struct Assembly {
    text: String,
    reasoning: String,
    calls: Vec<PartialCall>,
    finish_reason: Option<String>,
}

impl Assembly {
    async fn chunk(
        &mut self,
        value: &Value,
        field: Option<&str>,
        text: &mut dyn TextSink,
    ) -> Result<(), ModelError> {
        if let Some(message) = value["error"]["message"].as_str() {
            return Err(ModelError::Provider(message.chars().take(2000).collect()));
        }
        let Some(choice) = value["choices"].get(0) else {
            return Ok(());
        };
        let delta = &choice["delta"];
        if let Some(piece) = delta["content"].as_str() {
            self.text.push_str(piece);
            text.delta(piece).await?;
        }
        if let Some(piece) = field.and_then(|f| delta[f].as_str()) {
            self.reasoning.push_str(piece);
        }
        for call in delta["tool_calls"].as_array().into_iter().flatten() {
            let index = call["index"]
                .as_u64()
                .map(|i| i as usize)
                .unwrap_or(self.calls.len().saturating_sub(1));
            if index > self.calls.len() || index > 63 {
                return Err(ModelError::Protocol("noncontiguous tool call index".into()));
            }
            if index == self.calls.len() {
                self.calls.push(PartialCall::default());
            }
            let partial = &mut self.calls[index];
            if let Some(id) = call["id"].as_str() {
                partial.id = id.to_owned();
            }
            if let Some(name) = call["function"]["name"].as_str() {
                partial.name.push_str(name);
            }
            if let Some(args) = call["function"]["arguments"].as_str() {
                partial.arguments.push_str(args);
            }
        }
        if let Some(reason) = choice["finish_reason"].as_str() {
            self.finish_reason = Some(reason.to_owned());
        }
        Ok(())
    }

    fn finish(&mut self, field: Option<&str>) -> Result<ModelResponse, ModelError> {
        match self.finish_reason.as_deref() {
            Some("stop" | "tool_calls" | "function_call") => {}
            Some("length") => {
                return Err(ModelError::Provider(
                    "response hit the output limit before completing".into(),
                ));
            }
            Some("content_filter") => {
                return Err(ModelError::Provider(
                    "the provider's content filter stopped this response".into(),
                ));
            }
            other => {
                return Err(ModelError::Protocol(format!(
                    "unsupported finish reason {other:?}"
                )));
            }
        }
        let mut calls = Vec::new();
        let mut ids = HashSet::new();
        for partial in &self.calls {
            let arguments = if partial.arguments.trim().is_empty() {
                "{}"
            } else {
                partial.arguments.as_str()
            };
            let parsed: Value = serde_json::from_str(arguments)
                .map_err(|_| ModelError::Protocol("tool arguments are not valid JSON".into()))?;
            if !parsed.is_object()
                || partial.id.is_empty()
                || partial.name.is_empty()
                || !ids.insert(partial.id.clone())
            {
                return Err(ModelError::Protocol(
                    "invalid or duplicated tool call".into(),
                ));
            }
            calls.push(ToolCall {
                call_id: partial.id.clone(),
                name: partial.name.clone(),
                arguments: parsed.to_string(),
            });
        }
        let mut message = json!({"role":"assistant","content":if self.text.is_empty() { Value::Null } else { json!(self.text) }});
        if !calls.is_empty() {
            message["tool_calls"] = calls
                .iter()
                .map(|c| json!({"id":c.call_id,"type":"function","function":{"name":c.name,"arguments":c.arguments}}))
                .collect();
        }
        if let (Some(field), false) = (field, self.reasoning.is_empty()) {
            message[field] = json!(self.reasoning);
        }
        Ok(ModelResponse {
            continuation: ResponseContinuation::from_parts(
                &self.text,
                &calls,
                Some(json!({"type": OPAQUE, "message": message})),
            ),
            text: std::mem::take(&mut self.text),
            tool_calls: calls,
        })
    }
}

#[cfg(test)]
#[path = "opencode_tests.rs"]
mod tests;
