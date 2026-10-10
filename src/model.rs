use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;

use crate::openai::ResponseContinuation;

/// Conversation entries remain typed; provider replay data belongs to its adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Message {
    User(String),
    /// Deterministic incomplete task data, never executable tool requests.
    Summary(String),
    Assistant(ModelResponse),
    Tool(ToolResult),
}

#[derive(Debug)]
pub struct ModelRequest<'a> {
    pub model: &'a str,
    pub instructions: &'a str,
    pub messages: &'a [Message],
}

/// Constructed by the adapter only after successful response completion.
#[derive(Debug, Clone, Serialize)]
pub struct ModelResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub(crate) continuation: ResponseContinuation,
}

// Deserialization must preserve the adapter's completed-response invariant.
impl<'de> Deserialize<'de> for ModelResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct StoredResponse {
            text: String,
            tool_calls: Vec<ToolCall>,
            continuation: ResponseContinuation,
        }
        let stored = StoredResponse::deserialize(deserializer)?;
        let response = Self {
            text: stored.text,
            tool_calls: stored.tool_calls,
            continuation: stored.continuation,
        };
        response
            .continuation
            .validate_response(&response)
            .map_err(serde::de::Error::custom)?;
        Ok(response)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    /// JSON remains unparsed until the typed tool boundary validates it.
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub name: String,
    pub outcome: ToolOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ToolOutcome {
    Success { data: Value },
    Error { code: String, message: String },
    TimedOut { data: Value },
}

impl ToolResult {
    pub fn is_error(&self) -> bool {
        matches!(
            self.outcome,
            ToolOutcome::Error { .. } | ToolOutcome::TimedOut { .. }
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("provider transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("provider rejected request (HTTP {status}): {message}")]
    Http { status: u16, message: String },
    #[error("provider response failed: {0}")]
    Provider(String),
    #[error("invalid provider stream: {0}")]
    Protocol(String),
    #[error("could not deliver streamed text: {0}")]
    Output(#[from] io::Error),
}

#[async_trait]
pub trait TextSink: Send {
    /// Optional instrumentation is explicitly selected by the runtime caller.
    fn telemetry_enabled(&self) -> bool {
        false
    }
    /// Only typed metadata; credentials and provider continuation stay private.
    async fn telemetry(&mut self, _telemetry: crate::observability::Telemetry) {}
    /// Optional adapter-owned metadata for the exact prepared request, before dispatch.
    async fn request_prepared(
        &mut self,
        _snapshot: crate::context::ContextSnapshot,
    ) -> io::Result<()> {
        Ok(())
    }

    async fn delta(&mut self, text: &str) -> io::Result<()>;
}

#[async_trait]
impl<F> TextSink for F
where
    F: FnMut(&str) -> io::Result<()> + Send,
{
    async fn delta(&mut self, text: &str) -> io::Result<()> {
        self(text)
    }
}

/// A narrow seam for deterministic tests, not a universal provider abstraction.
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Known backend identity for traces; unknown is represented as unavailable.
    fn provider_name(&self) -> Option<&'static str> {
        None
    }
    /// Pure preflight accounting; None means this provider cannot enforce a budget.
    fn measure_request(
        &self,
        _request: &ModelRequest<'_>,
    ) -> Result<Option<crate::context::ContextSnapshot>, ModelError> {
        Ok(None)
    }

    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        text: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError>;
}
