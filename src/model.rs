use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;

use crate::openai::ResponseContinuation;

/// Conversation entries remain typed; provider replay data belongs to its adapter.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub(crate) continuation: ResponseContinuation,
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
