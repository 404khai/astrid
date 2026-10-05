use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;

use crate::openai::ResponseContinuation;

/// Conversation entries remain typed; provider replay data belongs to its adapter.
#[derive(Debug, Clone)]
pub enum Message {
    User(String),
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
}

impl ToolResult {
    pub fn is_error(&self) -> bool {
        matches!(self.outcome, ToolOutcome::Error { .. })
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
    #[error("could not display streamed output: {0}")]
    Output(#[from] io::Error),
}

pub type TextSink<'a> = dyn FnMut(&str) -> io::Result<()> + Send + 'a;

/// A narrow seam for deterministic tests, not a universal provider abstraction.
#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        text: &mut TextSink<'_>,
    ) -> Result<ModelResponse, ModelError>;
}
