//! OpenAI wire formats and continuation data stay inside this adapter.
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use crate::{
    auth::Authentication,
    context::{ContextMeasurement, ContextSnapshot, ContextSource},
    model::{Message, ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink, ToolCall},
    tools,
};

#[derive(Debug, Clone)]
pub(crate) struct ResponseContinuation(Vec<Value>);

pub struct OpenAiProvider {
    client: reqwest::Client,
    auth: Arc<dyn Authentication>,
    endpoint: String,
}

impl OpenAiProvider {
    pub fn new(auth: Arc<dyn Authentication>) -> Result<Self, ModelError> {
        Self::with_endpoint(auth, "https://api.openai.com/v1/responses")
    }

    /// A transport seam for local protocol tests. The CLI always uses OpenAI.
    pub fn with_endpoint(
        auth: Arc<dyn Authentication>,
        endpoint: impl Into<String>,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent(concat!("Astrid/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(Duration::from_secs(30))
                .timeout(Duration::from_secs(300))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            auth,
            endpoint: endpoint.into(),
        })
    }

    fn body(request: &ModelRequest<'_>) -> Value {
        let mut input = Vec::new();
        for message in request.messages {
            match message {
                Message::User(text) => input.push(json!({"role":"user","content":text})),
                Message::Summary(text) => input.push(json!({"role":"user","content":text})),
                Message::Assistant(response) => input.extend(response.continuation.0.iter().cloned()),
                Message::Tool(result) => input.push(json!({"type":"function_call_output","call_id":result.call_id,"output":serde_json::to_string(&result.outcome).expect("JSON tool outcome is serializable")})),
            }
        }
        // Subscription-backed Responses accepts tools grouped in namespaces.
        json!({"model":request.model,"instructions":request.instructions,"input":input,
            "store":false,"stream":true,"include":["reasoning.encrypted_content"],
            "tools":[{"type":"namespace","name":"astrid","description":"Astrid workspace tools", "tools":tools::definitions()}]})
    }

    fn snapshot(body: &Value, request: &ModelRequest<'_>) -> ContextSnapshot {
        use ContextSource::*;
        let mut measurements = [
            Instructions,
            UserMessages,
            AssistantContinuation,
            ToolResults,
            ToolDefinitions,
            RequestFraming,
            Summary,
        ]
        .map(|source| ContextMeasurement {
            source,
            entries: 0,
            serialized_bytes: 0,
            contains_opaque_data: false,
        });
        measurements[0].entries = 1;
        measurements[0].serialized_bytes = serialized_bytes(&body["instructions"]);
        measurements[4].entries = 1;
        measurements[4].serialized_bytes = serialized_bytes(&body["tools"]);
        let mut summary_positions = HashSet::new();
        let mut position = 0;
        for message in request.messages {
            if matches!(message, Message::Summary(_)) {
                summary_positions.insert(position);
            }
            position += match message {
                Message::Assistant(response) => response.continuation.0.len(),
                _ => 1,
            };
        }
        for (position, item) in body["input"]
            .as_array()
            .expect("adapter constructs input array")
            .iter()
            .enumerate()
        {
            let index = if summary_positions.contains(&position) {
                6
            } else if item["role"] == "user" {
                1
            } else if item["type"] == "function_call_output" {
                3
            } else {
                2
            };
            measurements[index].entries += 1;
            measurements[index].serialized_bytes += serialized_bytes(item);
            // Reasoning is provider-private. Its wire size cannot reveal context cost.
            measurements[index].contains_opaque_data |= item["type"] == "reasoning";
        }
        let serialized_request_bytes = serialized_bytes(body);
        measurements[5].entries = 1;
        measurements[5].serialized_bytes = serialized_request_bytes
            - measurements
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != 5)
                .map(|(_, m)| m)
                .map(|m| m.serialized_bytes)
                .sum::<usize>();
        let non_opaque_bytes = measurements
            .iter()
            .filter(|m| !m.contains_opaque_data)
            .map(|m| m.serialized_bytes)
            .sum::<usize>();
        ContextSnapshot {
            measurements: measurements.into(),
            serialized_request_bytes,
            non_opaque_json_size_token_heuristic: non_opaque_bytes.div_ceil(4),
            provider_input_tokens: None,
        }
    }
}

/// Count UTF-8 JSON output without allocating another serialized prompt copy.
fn serialized_bytes(value: &Value) -> usize {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value).expect("JSON value serialization is infallible");
    counter.0
}

#[async_trait]
impl ModelProvider for OpenAiProvider {
    fn measure_request(
        &self,
        request: &ModelRequest<'_>,
    ) -> Result<Option<ContextSnapshot>, ModelError> {
        Ok(Some(Self::snapshot(&Self::body(request), request)))
    }
    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        text: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        let body = Self::body(request);
        text.request_prepared(Self::snapshot(&body, request))
            .await?;
        let token = self
            .auth
            .bearer_token()
            .await
            .map_err(|error| ModelError::Provider(format!("authentication failed: {error}")))?;
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(token.as_str())
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await?;
            let parsed: Option<Value> = serde_json::from_str(&body).ok();
            let message = parsed
                .as_ref()
                .and_then(|v| v["error"]["message"].as_str())
                .unwrap_or("no structured provider error");
            return Err(ModelError::Http {
                status,
                message: message.chars().take(2000).collect(),
            });
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.is_empty()
            && !content_type
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("text/event-stream")
        {
            let content_type = content_type.to_owned();
            let body = response.text().await?;
            if let Ok(value) = serde_json::from_str::<Value>(&body)
                && let Some(message) = value["error"]["message"].as_str()
            {
                return Err(ModelError::Provider(message.chars().take(2000).collect()));
            }
            return Err(ModelError::Protocol(format!(
                "expected text/event-stream, received {content_type:?} (body omitted)"
            )));
        }
        let mut stream = response.bytes_stream();
        let mut decoder = SseDecoder::default();
        let mut assembly = ResponseAssembly::default();
        while let Some(chunk) = stream.next().await {
            for event in decoder.feed(&chunk?)? {
                if let Some(completed) = assembly.event(event, text).await? {
                    return Ok(completed);
                }
            }
        }
        Err(ModelError::Protocol(
            "stream ended before response.completed; no tools executed".into(),
        ))
    }
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, ModelError> {
    value[key]
        .as_str()
        .ok_or_else(|| ModelError::Protocol(format!("missing or invalid {key}")))
}

/// Validate the entire final envelope before exposing any tool call to the loop.
pub fn completed_response(response: Value) -> Result<ModelResponse, ModelError> {
    if string(&response, "status")? != "completed" {
        return Err(ModelError::Protocol(
            "response.completed has unsuccessful status".into(),
        ));
    }
    let output = response["output"]
        .as_array()
        .ok_or_else(|| ModelError::Protocol("missing output array".into()))?;
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    let mut ids = HashSet::new();
    for item in output {
        match string(item, "type")? {
            "function_call" => {
                if item.get("status").is_some_and(|v| v != "completed") {
                    return Err(ModelError::Protocol("incomplete function call".into()));
                }
                let id = string(item, "call_id")?;
                if id.is_empty() || !ids.insert(id.to_owned()) {
                    return Err(ModelError::Protocol("empty or duplicated call_id".into()));
                }
                let name = string(item, "name")?;
                let name = name.strip_prefix("astrid.").unwrap_or(name);
                if item
                    .get("namespace")
                    .is_some_and(|v| !v.is_null() && v != "astrid")
                {
                    return Err(ModelError::Protocol("unexpected function namespace".into()));
                }
                if name.is_empty() {
                    return Err(ModelError::Protocol("empty function name".into()));
                }
                tool_calls.push(ToolCall {
                    call_id: id.into(),
                    name: name.into(),
                    arguments: string(item, "arguments")?.into(),
                });
            }
            "message" => {
                if string(item, "role")? != "assistant"
                    || item.get("status").is_some_and(|v| v != "completed")
                {
                    return Err(ModelError::Protocol("invalid assistant message".into()));
                }
                let content = item["content"]
                    .as_array()
                    .ok_or_else(|| ModelError::Protocol("missing assistant content".into()))?;
                for block in content {
                    match string(block, "type")? {
                        "output_text" => text.push_str(string(block, "text")?),
                        "refusal" => text.push_str(string(block, "refusal")?),
                        kind => {
                            return Err(ModelError::Protocol(format!(
                                "unsupported assistant content {kind}"
                            )));
                        }
                    }
                }
            }
            "reasoning" => {} // Preserve opaque reasoning items without interpreting them.
            kind => {
                return Err(ModelError::Protocol(format!(
                    "unsupported output item {kind}"
                )));
            }
        }
    }
    Ok(ModelResponse {
        text,
        tool_calls,
        continuation: ResponseContinuation(output.clone()),
    })
}

#[derive(Default)]
struct ResponseAssembly {
    completed_items: BTreeMap<usize, Value>,
    arguments: HashMap<String, String>,
    streamed_text: String,
}

impl ResponseAssembly {
    async fn event(
        &mut self,
        event: Value,
        text: &mut dyn TextSink,
    ) -> Result<Option<ModelResponse>, ModelError> {
        match string(&event, "type")? {
            "response.output_text.delta" | "response.refusal.delta" => {
                let delta = string(&event, "delta")?;
                self.streamed_text.push_str(delta);
                text.delta(delta).await?;
            }
            "response.output_item.added" if event["item"]["type"] == "function_call" => {
                let item = &event["item"];
                let id = string(item, "id")?.to_owned();
                if self
                    .arguments
                    .insert(id, string(item, "arguments")?.to_owned())
                    .is_some()
                {
                    return Err(ModelError::Protocol("duplicated streamed tool item".into()));
                }
            }
            "response.function_call_arguments.delta" => {
                let id = string(&event, "item_id")?;
                let arguments = self.arguments.get_mut(id).ok_or_else(|| {
                    ModelError::Protocol("arguments delta has no tool item".into())
                })?;
                arguments.push_str(string(&event, "delta")?);
            }
            "response.function_call_arguments.done" => {
                let id = string(&event, "item_id")?;
                if self
                    .arguments
                    .get(id)
                    .is_some_and(|args| args != &event["arguments"])
                {
                    return Err(ModelError::Protocol(
                        "tool argument fragments disagree with completed arguments".into(),
                    ));
                }
            }
            "response.output_item.done" => {
                let index = event["output_index"]
                    .as_u64()
                    .and_then(|v| usize::try_from(v).ok())
                    .ok_or_else(|| ModelError::Protocol("missing output index".into()))?;
                if self
                    .completed_items
                    .insert(index, event["item"].clone())
                    .is_some()
                {
                    return Err(ModelError::Protocol(
                        "duplicated completed output item".into(),
                    ));
                }
            }
            "response.completed" => {
                let mut envelope = event["response"].clone();
                // Some subscription streams omit items from the terminal envelope.
                // Only completed item events can supply them, after terminal success.
                if envelope["output"].as_array().is_some_and(Vec::is_empty)
                    && !self.completed_items.is_empty()
                {
                    for (expected, actual) in self.completed_items.keys().enumerate() {
                        if expected != *actual {
                            return Err(ModelError::Protocol(
                                "noncontiguous completed output items".into(),
                            ));
                        }
                    }
                    envelope["output"] =
                        Value::Array(self.completed_items.values().cloned().collect());
                }
                let response = completed_response(envelope)?;
                for item in &response.continuation.0 {
                    if item["type"] == "function_call" {
                        let id = string(item, "id")?;
                        if self
                            .arguments
                            .get(id)
                            .is_some_and(|args| args != &item["arguments"])
                        {
                            return Err(ModelError::Protocol(
                                "tool argument fragments disagree with final output".into(),
                            ));
                        }
                    }
                }
                if self.streamed_text.is_empty() && !response.text.is_empty() {
                    text.delta(&response.text).await?;
                } else if self.streamed_text != response.text {
                    return Err(ModelError::Protocol(
                        "streamed text disagrees with final output".into(),
                    ));
                }
                return Ok(Some(response));
            }
            "response.failed" | "response.incomplete" | "error" => {
                let detail = event["response"]["error"]["message"]
                    .as_str()
                    .or_else(|| event["message"].as_str())
                    .unwrap_or("response failed or incomplete");
                return Err(ModelError::Provider(detail.into()));
            }
            _ => {} // Usage, reasoning, annotations, and lifecycle notifications.
        }
        Ok(None)
    }
}

/// Byte-oriented SSE framing tolerates fragmented UTF-8 and CR/LF boundaries.
#[derive(Default)]
struct SseDecoder {
    line: Vec<u8>,
    data: Vec<String>,
    skip_lf: bool,
}

impl SseDecoder {
    fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Value>, ModelError> {
        let mut events = Vec::new();
        for &byte in bytes {
            if self.skip_lf {
                self.skip_lf = false;
                if byte == b'\n' {
                    continue;
                }
            }
            if byte == b'\r' || byte == b'\n' {
                self.skip_lf = byte == b'\r';
                let line = String::from_utf8(std::mem::take(&mut self.line))
                    .map_err(|_| ModelError::Protocol("invalid UTF-8 in SSE line".into()))?;
                if line.is_empty() {
                    if !self.data.is_empty() {
                        let data = std::mem::take(&mut self.data).join("\n");
                        if data != "[DONE]" {
                            events.push(serde_json::from_str(&data).map_err(|e| {
                                ModelError::Protocol(format!("invalid SSE JSON: {e}"))
                            })?);
                        }
                    }
                } else if let Some(value) = line.strip_prefix("data:") {
                    self.data
                        .push(value.strip_prefix(' ').unwrap_or(value).into());
                }
            } else {
                self.line.push(byte);
            }
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sse_handles_every_byte_boundary_and_multiline_data() {
        let input = ": heartbeat\r\nevent: example\r\ndata: {\r\ndata: \"type\":\"response.output_text.delta\",\"delta\":\"héllo\"}\r\n\r\n";
        let mut decoder = SseDecoder::default();
        let events = input
            .as_bytes()
            .iter()
            .flat_map(|byte| decoder.feed(&[*byte]).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            events,
            vec![json!({"type":"response.output_text.delta","delta":"héllo"})]
        );
    }
}
