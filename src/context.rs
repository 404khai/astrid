//! Context provenance, request selection, and deterministic incomplete compaction.
//! Serialized size and local admission heuristics are not provider token usage.
use serde::{Deserialize, Serialize};

/// Stable within one session. Metadata points into history without copying it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContextItemId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContextOrigin {
    OperatingInstructions,
    RepositoryInstructions {
        path: String,
    },
    UserTask,
    Assistant {
        model_call_id: crate::events::ModelCallId,
    },
    ToolResult {
        tool_call_id: crate::events::ToolCallId,
        model_call_id: crate::events::ModelCallId,
        /// Requested file path, not proof of current file content or freshness.
        requested_path: Option<String>,
        path_truncated: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextItem {
    pub id: ContextItemId,
    pub origin: ContextOrigin,
    pub message_index: Option<usize>,
    pub added_reason: String,
}

impl ContextItem {
    pub fn inspection_line(&self) -> String {
        let source = match &self.origin {
            ContextOrigin::OperatingInstructions => "operating instructions".into(),
            ContextOrigin::RepositoryInstructions { path } => {
                format!("repository instructions: {path}")
            }
            ContextOrigin::UserTask => "user task".into(),
            ContextOrigin::Assistant { model_call_id } => {
                format!("assistant from model {model_call_id}")
            }
            ContextOrigin::ToolResult {
                tool_call_id,
                requested_path,
                path_truncated,
                ..
            } => format!(
                "tool {tool_call_id}{}{}",
                requested_path
                    .as_ref()
                    .map_or_else(String::new, |path| format!(", requested path: {path}")),
                if *path_truncated {
                    " (path label truncated)"
                } else {
                    ""
                },
            ),
        };
        format!(
            "context item {}: {source}; added: {}",
            self.id.0, self.added_reason
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextLedger {
    pub items: Vec<ContextItem>,
}

impl ContextLedger {
    pub fn add(&mut self, origin: ContextOrigin, message_index: Option<usize>) -> ContextItem {
        let item = ContextItem {
            id: ContextItemId(self.items.len()),
            added_reason: match &origin {
                ContextOrigin::OperatingInstructions => {
                    "operating guidance and effective permission policy"
                }
                ContextOrigin::RepositoryInstructions { .. } => {
                    "workspace-root repository guidance"
                }
                ContextOrigin::UserTask => "submitted task",
                ContextOrigin::Assistant { .. } => "validated completed model response",
                ContextOrigin::ToolResult { .. } => "terminal tool outcome returned to the model",
            }
            .into(),
            origin,
            message_index,
        };
        self.items.push(item.clone());
        item
    }
}

/// Bound copied path metadata independently of the original tool arguments.
pub fn requested_path(call: &crate::model::ToolCall) -> (Option<String>, bool) {
    if !matches!(call.name.as_str(), "read_file" | "write_file" | "edit_file") {
        return (None, false);
    }
    let Ok(arguments) = serde_json::from_str::<serde_json::Value>(&call.arguments) else {
        return (None, false);
    };
    let Some(path) = arguments["path"].as_str() else {
        return (None, false);
    };
    let mut end = path.len().min(1024);
    while !path.is_char_boundary(end) {
        end -= 1;
    }
    (Some(path[..end].into()), end != path.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    Instructions,
    UserMessages,
    AssistantContinuation,
    ToolResults,
    ToolDefinitions,
    RequestFraming,
    Summary,
}

/// Aggregate metadata only; never contains prompt or continuation contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextMeasurement {
    pub source: ContextSource,
    pub entries: usize,
    pub serialized_bytes: usize,
    pub contains_opaque_data: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub measurements: Vec<ContextMeasurement>,
    pub serialized_request_bytes: usize,
    /// Size heuristic for non-opaque JSON only: ceil(bytes / 4).
    /// Includes JSON syntax, not tokenizer counts. Not a total request estimate.
    pub non_opaque_json_size_token_heuristic: usize,
    /// No tokenizer or provider token count is available in this slice.
    pub provider_input_tokens: Option<usize>,
}

impl ContextSnapshot {
    pub fn inspection_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "context: {} serialized request bytes; provider input tokens unavailable",
            self.serialized_request_bytes
        )];
        for item in &self.measurements {
            lines.push(format!(
                "  {:?}: {} bytes, {} entries{}",
                item.source,
                item.serialized_bytes,
                item.entries,
                if item.contains_opaque_data {
                    "; opaque token cost unknown"
                } else {
                    ""
                }
            ));
        }
        lines.push(format!(
            "  non-opaque JSON bytes/4 heuristic: {}; not a tokenizer count or full context total",
            self.non_opaque_json_size_token_heuristic
        ));
        lines
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum SelectionPolicy {
    Recency,
    FileReferences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudget {
    pub estimated_context_tokens: usize,
    pub response_reserve_tokens: usize,
    pub max_request_bytes: usize,
    pub max_summary_bytes: usize,
    pub policy: SelectionPolicy,
}
impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            estimated_context_tokens: 65_536,
            response_reserve_tokens: 4_096,
            max_request_bytes: 524_288,
            max_summary_bytes: 4_096,
            policy: SelectionPolicy::FileReferences,
        }
    }
}
impl ContextBudget {
    pub fn validate(&self) -> bool {
        self.estimated_context_tokens > self.response_reserve_tokens
            && self.max_request_bytes > 0
            && self.max_summary_bytes > 0
    }
    pub fn admits(&self, snapshot: &ContextSnapshot) -> bool {
        self.validate()
            && snapshot.serialized_request_bytes <= self.max_request_bytes
            && snapshot.non_opaque_json_size_token_heuristic
                <= self.estimated_context_tokens - self.response_reserve_tokens
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextDecision {
    pub item_id: ContextItemId,
    pub retained: bool,
    pub reason: String,
}

/// Lineage describes inputs considered for condensation, not lossless recall.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSummary {
    pub source_items: Vec<ContextItemId>,
    pub text_bytes: usize,
    pub text: String,
    pub incomplete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSelection {
    pub budget: ContextBudget,
    pub decisions: Vec<ContextDecision>,
    pub summary: Option<ContextSummary>,
    pub snapshot: ContextSnapshot,
}
impl ContextSelection {
    pub fn inspection_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "context selection: {:?}; input heuristic allowance {}, response reserve {}, byte ceiling {} (provider fit unknown)",
            self.budget.policy,
            self.budget.estimated_context_tokens - self.budget.response_reserve_tokens,
            self.budget.response_reserve_tokens,
            self.budget.max_request_bytes,
        )];
        for decision in &self.decisions {
            lines.push(format!(
                "  item {} {}: {}",
                decision.item_id.0,
                if decision.retained {
                    "retained"
                } else {
                    "evicted from request"
                },
                decision.reason
            ));
        }
        if let Some(summary) = &self.summary {
            lines.push(format!(
                "  compacted {} source items into {} text bytes; incomplete; source IDs {:?}",
                summary.source_items.len(),
                summary.text_bytes,
                summary.source_items
            ));
            lines.push(format!("  summary task data: {}", summary.text));
        }
        lines
    }
}

pub(crate) struct PreparedSelection {
    pub metadata: ContextSelection,
    pub messages: Vec<crate::model::Message>,
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("context selection cancelled before commit")]
    Cancelled,
    #[error("invalid context budget")]
    InvalidBudget,
    #[error("provider does not expose request accounting required for context budgeting")]
    AccountingUnavailable,
    #[error(
        "protected instructions, task, and latest exchange exceed the configured context allowance: {details}"
    )]
    ProtectedOverflow { details: String },
    #[error("invalid context history: {0}")]
    InvalidHistory(String),
    #[error("context accounting failed: {0}")]
    Accounting(#[from] crate::model::ModelError),
}

/// Validate closure independently of policy; every multi-tool batch stays whole.
pub(crate) fn exchange_ranges(
    messages: &[crate::model::Message],
) -> Result<Vec<std::ops::Range<usize>>, ContextError> {
    use crate::model::Message;
    if !matches!(messages.first(), Some(Message::User(_))) {
        return Err(ContextError::InvalidHistory("missing original task".into()));
    }
    let mut ranges = Vec::new();
    let current_task = messages
        .iter()
        .rposition(|m| matches!(m, Message::User(_)))
        .unwrap_or(0);
    let mut start = 1;
    while start < messages.len() {
        if matches!(&messages[start], Message::User(_)) {
            ranges.push(start..start + 1);
            start += 1;
            continue;
        }
        let Message::Assistant(response) = &messages[start] else {
            return Err(ContextError::InvalidHistory(
                "exchange must start with an assistant".into(),
            ));
        };
        let end = start + 1 + response.tool_calls.len();
        let Some(outcomes) = messages.get(start + 1..end) else {
            return Err(ContextError::InvalidHistory("incomplete tool batch".into()));
        };
        for (call, outcome) in response.tool_calls.iter().zip(outcomes) {
            if !matches!(outcome, Message::Tool(result) if result.call_id == call.call_id && result.name == call.name)
            {
                return Err(ContextError::InvalidHistory(
                    "tool result does not match its request".into(),
                ));
            }
        }
        // Historical submissions stay together with their assistant/tool exchanges.
        if start < current_task {
            if let Some(previous) = ranges.last_mut() {
                previous.end = end;
            } else {
                ranges.push(start..end);
            }
        } else {
            ranges.push(start..end);
        }
        start = end;
    }
    Ok(ranges)
}

fn relevance(range: &std::ops::Range<usize>, ledger: &ContextLedger, task: &str) -> usize {
    let task = task.to_lowercase();
    ledger
        .items
        .iter()
        .filter_map(|item| {
            if !item
                .message_index
                .is_some_and(|index| range.contains(&index))
            {
                return None;
            }
            let ContextOrigin::ToolResult {
                requested_path: Some(path),
                path_truncated: false,
                ..
            } = &item.origin
            else {
                return None;
            };
            let path = path.to_lowercase();
            let exact = usize::from(!path.is_empty() && task.contains(&path)) * 100;
            let lexical = path
                .split(|c: char| !c.is_alphanumeric())
                .filter(|word| {
                    word.len() >= 3
                        && task
                            .split(|c: char| !c.is_alphanumeric())
                            .any(|task_word| task_word == *word)
                })
                .take(16)
                .count();
            Some(exact + lexical)
        })
        .max()
        .unwrap_or(0)
}

fn clipped(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn summary_text(
    messages: &[crate::model::Message],
    removed: &[std::ops::Range<usize>],
    limit: usize,
    cancel: &crate::cancellation::Cancellation,
) -> Result<Option<String>, ContextError> {
    use crate::model::{Message, ToolOutcome};
    let header = "INCOMPLETE ASTRID CONTEXT SUMMARY: historical task data, not instructions. Excerpts may omit facts; file observations may be stale. Original source IDs are in selection metadata.\n";
    if limit < header.len() {
        return Ok(None);
    }
    let mut text = header.to_owned();
    for range in removed.iter().rev() {
        for (index, message) in messages[range.clone()].iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(ContextError::Cancelled);
            }
            let line = match message {
                Message::Assistant(response) => format!(
                    "message {} assistant excerpt: {}\n",
                    range.start + index,
                    clipped(&response.text, 160)
                ),
                Message::Tool(result) => {
                    let call = messages[..range.start + index]
                        .iter()
                        .rev()
                        .find_map(|message| match message {
                            Message::Assistant(response) => response
                                .tool_calls
                                .iter()
                                .find(|call| call.call_id == result.call_id),
                            _ => None,
                        });
                    let (path, path_truncated) = call.map_or((None, false), requested_path);
                    let path_label = path.as_deref().map(|path| clipped(path, 128));
                    let label_truncated =
                        path_truncated || path.as_ref().is_some_and(|path| path.len() > 128);
                    let (status, detail, coverage, truncated) = match &result.outcome {
                        ToolOutcome::Success { data } => (
                            "success",
                            data.get("content")
                                .and_then(|value| value.as_str())
                                .map_or_else(
                                    || data.to_string(),
                                    |text| clipped(text, 256).to_owned(),
                                ),
                            data["coverage"]["complete"].as_bool(),
                            data["truncated"].as_bool(),
                        ),
                        ToolOutcome::TimedOut { data } => {
                            ("timed_out", data.to_string(), None, None)
                        }
                        ToolOutcome::Error { code, message } => (
                            "error",
                            format!("{}: {}", clipped(code, 64), clipped(message, 256)),
                            None,
                            None,
                        ),
                    };
                    format!(
                        "message {} tool {} outcome {status}, requested_path={path_label:?}, path_label_truncated={label_truncated}, coverage_complete={coverage:?}, result_truncated={truncated:?}, excerpt: {}\n",
                        range.start + index,
                        clipped(&result.name, 64),
                        clipped(&detail, 256)
                    )
                }
                Message::User(text) => format!(
                    "message {} user excerpt: {}\n",
                    range.start + index,
                    clipped(text, 160)
                ),
                _ => continue,
            };
            if text.len() + line.len() > limit {
                return Ok(Some(text));
            }
            text.push_str(&line);
        }
    }
    Ok(Some(text))
}

/// Pure preparation. The caller commits metadata only after cancellation checks.
pub(crate) async fn select(
    provider: &dyn crate::model::ModelProvider,
    model: &str,
    instructions: &str,
    history: &[crate::model::Message],
    ledger: &ContextLedger,
    budget: &ContextBudget,
    cancel: &crate::cancellation::Cancellation,
) -> Result<PreparedSelection, ContextError> {
    use crate::model::{Message, ModelRequest};
    if !budget.validate() {
        return Err(ContextError::InvalidBudget);
    }
    let groups = exchange_ranges(history)?;
    let measure = |messages: &[Message]| -> Result<ContextSnapshot, ContextError> {
        if cancel.is_cancelled() {
            return Err(ContextError::Cancelled);
        }
        let measured = provider.measure_request(&ModelRequest {
            model,
            instructions,
            messages,
        });
        if cancel.is_cancelled() {
            return Err(ContextError::Cancelled);
        }
        measured?.ok_or(ContextError::AccountingUnavailable)
    };
    let current_task = history
        .iter()
        .rposition(|m| matches!(m, Message::User(_)))
        .unwrap_or(0);
    let task_group = groups
        .iter()
        .position(|range| range.contains(&current_task));
    let mut selected = vec![false; groups.len()];
    if let Some(index) = task_group {
        selected[index] = true;
    }
    let latest_exchange = groups.iter().rposition(|range| {
        history[range.clone()]
            .iter()
            .any(|m| matches!(m, Message::Assistant(_)))
    });
    if let Some(index) = latest_exchange {
        selected[index] = true;
    }
    let assemble = |chosen: &[bool], summary: Option<&str>| {
        let mut messages = vec![history[0].clone()];
        if let Some(text) = summary {
            messages.push(Message::Summary(text.into()));
        }
        for (range, keep) in groups.iter().zip(chosen) {
            if *keep {
                messages.extend(history[range.clone()].iter().cloned());
            }
        }
        messages
    };
    let mandatory = assemble(&selected, None);
    let snapshot = measure(&mandatory)?;
    if !budget.admits(&snapshot) {
        let sizes = snapshot
            .measurements
            .iter()
            .map(|item| format!("{:?}={} bytes", item.source, item.serialized_bytes))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ContextError::ProtectedOverflow {
            details: format!(
                "non-opaque JSON bytes/4 heuristic {} / {} input allowance ({} response reserve); serialized request {} / {} bytes; {sizes}. Provider fit is unknown. Protected items cannot be pruned; reduce inspection output or explicitly increase the configured budget.",
                snapshot.non_opaque_json_size_token_heuristic,
                budget.estimated_context_tokens - budget.response_reserve_tokens,
                budget.response_reserve_tokens,
                snapshot.serialized_request_bytes,
                budget.max_request_bytes,
            ),
        });
    }
    let mut candidates = (0..groups.len().saturating_sub(1)).collect::<Vec<_>>();
    candidates.sort_by_key(|index| {
        std::cmp::Reverse((
            if budget.policy == SelectionPolicy::FileReferences {
                relevance(
                    &groups[*index],
                    ledger,
                    match &history[current_task] {
                        Message::User(text) => text,
                        _ => "",
                    },
                )
            } else {
                0
            },
            *index,
        ))
    });
    for index in candidates {
        tokio::task::yield_now().await;
        if Some(index) == task_group || Some(index) == latest_exchange {
            continue;
        }
        selected[index] = true;
        if !budget.admits(&measure(&assemble(&selected, None))?) {
            selected[index] = false;
        }
    }
    let removed = groups
        .iter()
        .zip(&selected)
        .filter(|(_, keep)| !**keep)
        .map(|(range, _)| range.clone())
        .collect::<Vec<_>>();
    let source_items = ledger
        .items
        .iter()
        .filter(|item| {
            item.message_index
                .is_some_and(|index| removed.iter().any(|range| range.contains(&index)))
        })
        .map(|item| item.id)
        .collect::<Vec<_>>();
    let mut summary = None;
    let mut summary_limit = budget.max_summary_bytes;
    while !removed.is_empty() {
        tokio::task::yield_now().await;
        let Some(text) = summary_text(history, &removed, summary_limit, cancel)? else {
            break;
        };
        if budget.admits(&measure(&assemble(&selected, Some(&text)))?) {
            summary = Some(text);
            break;
        }
        summary_limit /= 2;
    }
    let messages = assemble(&selected, summary.as_deref());
    let snapshot = measure(&messages)?;
    let summary_metadata = summary.as_ref().map(|text| ContextSummary {
        source_items,
        text_bytes: text.len(),
        text: text.clone(),
        incomplete: true,
    });
    let decisions = ledger
        .items
        .iter()
        .map(|item| {
            let group = item
                .message_index
                .and_then(|index| groups.iter().position(|range| range.contains(&index)));
            let protected = group.is_none() || group == task_group || group == latest_exchange;
            let retained = protected || group.is_some_and(|index| selected[index]);
            ContextDecision {
                item_id: item.id,
                retained,
                reason: if protected {
                    "protected instructions, task, or latest complete exchange"
                } else if retained && budget.policy == SelectionPolicy::FileReferences {
                    "file-reference/lexical priority, then recency; complete exchange fits"
                } else if retained {
                    "recency; complete exchange fits"
                } else if summary_metadata.is_some() {
                    "complete exchange omitted under budget; considered for incomplete condensation"
                } else {
                    "complete exchange omitted under budget; no summary fits"
                }
                .into(),
            }
        })
        .collect();
    if cancel.is_cancelled() {
        return Err(ContextError::Cancelled);
    }
    Ok(PreparedSelection {
        messages,
        metadata: ContextSelection {
            budget: budget.clone(),
            decisions,
            summary: summary_metadata,
            snapshot,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::{AuthError, Authentication, BearerToken},
        cancellation::Cancellation,
        model::{Message, ModelProvider, ModelRequest, ToolOutcome, ToolResult},
        openai::{OpenAiProvider, completed_response},
    };
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;

    struct NoAuth;
    #[async_trait]
    impl Authentication for NoAuth {
        async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
            panic!("preflight cannot authenticate")
        }
    }
    fn provider() -> OpenAiProvider {
        OpenAiProvider::new(Arc::new(NoAuth)).unwrap()
    }
    fn add_exchange(history: &mut Vec<Message>, path: &str, text: &str, opaque: bool) {
        let call_id = format!("call-{}", history.len());
        let mut output = vec![
            json!({"type":"function_call","id":call_id,"call_id":call_id,"name":"read_file","arguments":json!({"path":path}).to_string()}),
        ];
        if opaque {
            output.push(json!({"type":"reasoning","encrypted_content":"PRIVATE-REASONING-SENTINEL".repeat(800)}));
        }
        let response = completed_response(json!({"status":"completed","output":output})).unwrap();
        history.push(Message::Assistant(response));
        history.push(Message::Tool(ToolResult { call_id, name: "read_file".into(), outcome: ToolOutcome::Success { data: json!({"path":path,"content":text,"truncated":true,"coverage":{"complete":false}}) } }));
    }
    fn ledger(history: &[Message]) -> ContextLedger {
        let mut ledger = ContextLedger::default();
        ledger.add(ContextOrigin::OperatingInstructions, None);
        for (index, message) in history.iter().enumerate() {
            let origin = match message {
                Message::User(_) => ContextOrigin::UserTask,
                Message::Assistant(_) => ContextOrigin::Assistant {
                    model_call_id: Default::default(),
                },
                Message::Tool(result) => {
                    let response = history[..index]
                        .iter()
                        .rev()
                        .find_map(|message| {
                            if let Message::Assistant(response) = message {
                                Some(response)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    let (requested_path, path_truncated) = requested_path(
                        response
                            .tool_calls
                            .iter()
                            .find(|call| call.call_id == result.call_id)
                            .unwrap(),
                    );
                    ContextOrigin::ToolResult {
                        tool_call_id: Default::default(),
                        model_call_id: Default::default(),
                        requested_path,
                        path_truncated,
                    }
                }
                Message::Summary(_) => panic!("original history cannot contain summaries"),
            };
            ledger.add(origin, Some(index));
        }
        ledger
    }
    fn bytes(provider: &dyn ModelProvider, messages: &[Message]) -> usize {
        provider
            .measure_request(&ModelRequest {
                model: "test",
                instructions: "operating",
                messages,
            })
            .unwrap()
            .unwrap()
            .serialized_request_bytes
    }
    fn budget(bytes: usize) -> ContextBudget {
        ContextBudget {
            estimated_context_tokens: 1_000_000,
            response_reserve_tokens: 1,
            max_request_bytes: bytes,
            max_summary_bytes: 1024,
            policy: SelectionPolicy::Recency,
        }
    }

    #[tokio::test]
    async fn selection_compares_recency_and_file_relevance_without_reading_more_files() {
        let provider = provider();
        let mut history = vec![Message::User("repair target.rs".into())];
        add_exchange(
            &mut history,
            "target.rs",
            &"important evidence ".repeat(300),
            false,
        );
        add_exchange(
            &mut history,
            "noise.rs",
            &"unrelated evidence ".repeat(300),
            false,
        );
        add_exchange(&mut history, "latest.rs", "latest evidence", false);
        let ledger = ledger(&history);
        let one_old = vec![
            history[0].clone(),
            history[1].clone(),
            history[2].clone(),
            history[5].clone(),
            history[6].clone(),
        ];
        let mut budget = budget(bytes(&provider, &one_old) + 800);
        let recent = select(
            &provider,
            "test",
            "operating",
            &history,
            &ledger,
            &budget,
            &Cancellation::default(),
        )
        .await
        .unwrap();
        budget.policy = SelectionPolicy::FileReferences;
        let relevant = select(
            &provider,
            "test",
            "operating",
            &history,
            &ledger,
            &budget,
            &Cancellation::default(),
        )
        .await
        .unwrap();
        let selected_path = |selection: &PreparedSelection, path: &str| {
            selection.messages.iter().any(|message| matches!(message,Message::Tool(result) if matches!(&result.outcome,ToolOutcome::Success{data} if data["path"] == path)))
        };
        assert!(!selected_path(&recent, "target.rs"));
        assert!(selected_path(&recent, "noise.rs"));
        assert!(selected_path(&relevant, "target.rs"));
        assert!(!selected_path(&relevant, "noise.rs"));
        assert!(selected_path(&recent, "latest.rs") && selected_path(&relevant, "latest.rs"));
        assert!(budget.admits(&relevant.metadata.snapshot));
        assert_eq!(history.len(), 7);
    }

    #[tokio::test]
    async fn summaries_preserve_file_coverage_and_exclude_private_reasoning() {
        let provider = provider();
        let mut history = vec![Message::User("task".into())];
        add_exchange(
            &mut history,
            "old.rs",
            &"🦀 useful content ".repeat(2000),
            true,
        );
        add_exchange(&mut history, "new.rs", "new", false);
        let ledger = ledger(&history);
        let mandatory = vec![history[0].clone(), history[3].clone(), history[4].clone()];
        let budget = budget(bytes(&provider, &mandatory) + 1400);
        let prepared = select(
            &provider,
            "test",
            "operating",
            &history,
            &ledger,
            &budget,
            &Cancellation::default(),
        )
        .await
        .unwrap();
        let summary = prepared.metadata.summary.as_ref().unwrap();
        assert!(summary.text.contains("old.rs"));
        assert!(summary.text.contains("coverage_complete=Some(false)"));
        assert!(summary.text.contains("result_truncated=Some(true)"));
        assert!(!summary.text.contains("PRIVATE-REASONING-SENTINEL"));
        assert!(summary.text.len() <= budget.max_summary_bytes);
        assert_eq!(summary.source_items.len(), 2);
        assert!(summary.incomplete);
        assert_eq!(
            prepared
                .messages
                .iter()
                .filter(|message| matches!(message, Message::Summary(_)))
                .count(),
            1
        );
        assert_eq!(
            prepared
                .messages
                .iter()
                .filter(|message| matches!(message, Message::Tool(_)))
                .count(),
            1
        );
        // Summary fit failure leaves the valid uncondensed candidate, without
        // overwriting history or creating an over-budget request.
        let exact = self::budget(bytes(&provider, &mandatory));
        let no_summary = select(
            &provider,
            "test",
            "operating",
            &history,
            &ledger,
            &exact,
            &Cancellation::default(),
        )
        .await
        .unwrap();
        assert!(no_summary.metadata.summary.is_none());
        assert_eq!(no_summary.messages.len(), mandatory.len());
    }

    #[tokio::test]
    async fn protected_overflow_and_incomplete_batches_fail_before_inference() {
        let provider = provider();
        let mut history = vec![Message::User("task".into())];
        add_exchange(&mut history, "file", "result", false);
        let ledger = ledger(&history);
        let tiny = budget(bytes(&provider, &history) - 1);
        assert!(matches!(
            select(
                &provider,
                "test",
                "operating",
                &history,
                &ledger,
                &tiny,
                &Cancellation::default()
            )
            .await,
            Err(ContextError::ProtectedOverflow { .. })
        ));
        let error = select(
            &provider,
            "test",
            "operating",
            &history,
            &ledger,
            &tiny,
            &Cancellation::default(),
        )
        .await
        .err()
        .unwrap()
        .to_string();
        assert!(error.contains(&format!(
            "serialized request {} / {} bytes",
            bytes(&provider, &history),
            tiny.max_request_bytes
        )));
        assert!(error.contains("ToolResults="));
        assert!(error.contains("Provider fit is unknown"));
        history.pop();
        assert!(matches!(
            select(
                &provider,
                "test",
                "operating",
                &history,
                &ledger,
                &budget(100_000),
                &Cancellation::default()
            )
            .await,
            Err(ContextError::InvalidHistory(_))
        ));
    }

    #[test]
    fn response_reserve_and_wire_ceiling_are_separate_local_limits() {
        let snapshot = provider()
            .measure_request(&ModelRequest {
                model: "test",
                instructions: "operating",
                messages: &[Message::User("task".into())],
            })
            .unwrap()
            .unwrap();
        let mut limits = budget(snapshot.serialized_request_bytes);
        assert!(limits.admits(&snapshot));
        limits.max_request_bytes -= 1;
        assert!(!limits.admits(&snapshot));
        limits.max_request_bytes += 1;
        limits.estimated_context_tokens = snapshot.non_opaque_json_size_token_heuristic;
        assert!(!limits.admits(&snapshot));
        limits.response_reserve_tokens = 0;
        assert!(limits.admits(&snapshot));
        limits.response_reserve_tokens = limits.estimated_context_tokens;
        assert!(!limits.validate());
    }

    #[tokio::test]
    async fn latest_multi_tool_exchange_keeps_denied_and_success_outcomes_together() {
        let provider = provider();
        let mut history = vec![Message::User("task".into())];
        add_exchange(&mut history, "old.rs", &"old".repeat(4000), false);
        let output = vec![
            json!({"type":"function_call","id":"read","call_id":"read","name":"read_file","arguments":"{\"path\":\"new.rs\"}"}),
            json!({"type":"function_call","id":"shell","call_id":"shell","name":"shell","arguments":"{\"command\":\"echo test\"}"}),
        ];
        history.push(Message::Assistant(
            completed_response(json!({"status":"completed","output":output})).unwrap(),
        ));
        history.push(Message::Tool(ToolResult {
            call_id: "read".into(),
            name: "read_file".into(),
            outcome: ToolOutcome::Success {
                data: json!({"content":"new"}),
            },
        }));
        history.push(Message::Tool(ToolResult {
            call_id: "shell".into(),
            name: "shell".into(),
            outcome: ToolOutcome::Error {
                code: "permission_denied".into(),
                message: "denied".into(),
            },
        }));
        let ledger = ledger(&history);
        let mandatory = vec![
            history[0].clone(),
            history[3].clone(),
            history[4].clone(),
            history[5].clone(),
        ];
        let limits = budget(bytes(&provider, &mandatory) + 1200);
        let selected = select(
            &provider,
            "test",
            "operating",
            &history,
            &ledger,
            &limits,
            &Cancellation::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            selected
                .messages
                .iter()
                .filter(|message| matches!(message, Message::Tool(_)))
                .count(),
            2
        );
        assert!(selected.messages.iter().any(|message| matches!(message,Message::Tool(result) if result.call_id=="shell" && result.is_error())));
        let too_small = budget(bytes(&provider, &mandatory) - 1);
        assert!(matches!(
            select(
                &provider,
                "test",
                "operating",
                &history,
                &ledger,
                &too_small,
                &Cancellation::default()
            )
            .await,
            Err(ContextError::ProtectedOverflow { .. })
        ));
    }
    #[tokio::test]
    async fn followup_selection_keeps_current_task_and_historical_questions_with_answers() {
        let mut history = vec![Message::User("original task".into())];
        add_exchange(&mut history, "old.rs", &"old data ".repeat(4000), false);
        history.push(Message::User("obsolete question ".repeat(4000)));
        add_exchange(
            &mut history,
            "middle.rs",
            &"middle data ".repeat(4000),
            false,
        );
        history.push(Message::User("recent question".into()));
        add_exchange(&mut history, "recent.rs", "small latest result", false);
        history.push(Message::User("current question".into()));
        let budget = ContextBudget {
            max_request_bytes: 14_000,
            ..Default::default()
        };
        let selected = select(
            &provider(),
            "test",
            "instructions",
            &history,
            &ledger(&history),
            &budget,
            &Cancellation::default(),
        )
        .await
        .unwrap();
        let user_messages = selected
            .messages
            .iter()
            .filter_map(|m| match m {
                Message::User(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            user_messages,
            vec!["original task", "recent question", "current question"]
        );
        let decision = |index| {
            selected
                .metadata
                .decisions
                .iter()
                .zip(ledger(&history).items)
                .find_map(|(decision, item)| {
                    (item.message_index == Some(index)).then_some(decision.retained)
                })
                .unwrap()
        };
        assert!(!decision(3)); // obsolete question
        assert!(!decision(4)); // its assistant/tool exchange
        assert!(!decision(5));
        assert!(decision(6)); // recent question and latest complete exchange
        assert!(decision(7));
        assert!(decision(8));
        assert!(decision(9)); // current submission
        assert!(
            selected
                .metadata
                .summary
                .as_ref()
                .unwrap()
                .text
                .contains("user excerpt")
        );
        let messages = selected
            .messages
            .into_iter()
            .filter(|m| !matches!(m, Message::Summary(_)))
            .collect::<Vec<_>>();
        assert!(exchange_ranges(&messages).is_ok());
    }
}

#[cfg(test)]
mod default_budget_regression {
    use super::{ContextBudget, ContextSnapshot};
    #[test]
    fn default_admits_reported_architecture_inspection_without_relaxing_byte_ceiling() {
        let snapshot = ContextSnapshot {
            measurements: Vec::new(),
            serialized_request_bytes: 133_328,
            non_opaque_json_size_token_heuristic: 32_251,
            provider_input_tokens: None,
        };
        let default = ContextBudget::default();
        assert!(default.admits(&snapshot));
        let previous = ContextBudget {
            estimated_context_tokens: 32_768,
            ..default.clone()
        };
        assert!(!previous.admits(&snapshot));
        let oversized = ContextSnapshot {
            serialized_request_bytes: default.max_request_bytes + 1,
            ..snapshot
        };
        assert!(!default.admits(&oversized));
    }
}
