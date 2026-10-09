//! Derived presentation only: consumes events, never dispatches models or tools.
use super::{
    format::{printable, single_line},
    theme::Ink,
};
use astrid::{
    events::{EventKind, ExecutionEvent, ToolCallId},
    model::{ToolCall, ToolOutcome},
    output::{OutputStream, TextDecoder},
};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug)]
pub(super) struct Piece {
    pub text: String,
    pub ink: Ink,
    pub model: bool,
}
pub(super) struct Presentation {
    model: String,
    pub mode: String,
    pub show_context: bool,
    pub expanded_tool_calls: bool,
    pieces: Vec<Piece>,
    calls: BTreeMap<ToolCallId, String>,
    run: String,
    turn: usize,
    model_calls: usize,
    state: &'static str,
    pub(super) waiting: bool,
    pub(super) approval_authority: &'static str,
    pub(super) done: bool,
    text_open: bool,
    tool_output_open: bool,
    active_stream: Option<(ToolCallId, OutputStream)>,
    decoders: BTreeMap<(ToolCallId, OutputStream), TextDecoder>,
}
impl Presentation {
    pub(super) fn new(model: String) -> Self {
        Self {
            model,
            mode: "auto".into(),
            show_context: false,
            expanded_tool_calls: false,
            pieces: Vec::new(),
            calls: BTreeMap::new(),
            run: String::new(),
            turn: 0,
            model_calls: 0,
            state: "starting",
            approval_authority: "Workspace operation subject to validated paths.",
            waiting: false,
            done: false,
            text_open: false,
            tool_output_open: false,
            active_stream: None,
            decoders: BTreeMap::new(),
        }
    }
    fn emit(&mut self, text: &str, ink: Ink, model: bool) {
        self.pieces.push(Piece {
            text: printable(text),
            ink: super::theme::mode_ink(ink, self.mode == "unbound"),
            model,
        });
    }
    pub(super) fn take_output(&mut self) -> Vec<Piece> {
        std::mem::take(&mut self.pieces)
    }
    pub(super) fn finish(&mut self, model_calls: usize, tool_calls: usize) {
        self.line(
            &format!("  {model_calls} model calls · {tool_calls} tool outcomes"),
            Ink::Dim,
        );
    }
    pub(super) fn status(&self, width: usize) -> String {
        if self.run.is_empty() {
            return format!("{} · {}", self.mode, self.state);
        }
        if width < 30 {
            format!(
                "{} RUN {} T{} {}",
                self.mode, self.run, self.turn, self.state
            )
        } else if width < 76 {
            format!(
                "{} RUN {}   TURN {}   {}   MODEL {}",
                self.mode, self.run, self.turn, self.state, self.model
            )
        } else {
            format!(
                "{} RUN {}   TURN {}   MODEL {}   CALLS {}   {}",
                self.mode, self.run, self.turn, self.model, self.model_calls, self.state
            )
        }
    }
    fn line(&mut self, text: &str, ink: Ink) {
        if self.tool_output_open {
            self.emit("\n", Ink::Normal, false);
            self.tool_output_open = false;
            self.active_stream = None;
        }
        if self.text_open {
            self.emit("\n", Ink::Normal, true);
            self.text_open = false;
        }
        self.emit(&format!("{}\n", single_line(text)), ink, false)
    }
    pub(super) fn apply(&mut self, event: &ExecutionEvent) {
        match &event.kind {
            EventKind::RunStarted { task, model, .. } => {
                self.model = single_line(model);
                self.run = event.run_id.to_string()[..5].into();
                self.line(&format!("› {}", single_line(task)), Ink::Accent);
                self.line("", Ink::Normal);
                self.state = "running";
            }
            EventKind::TurnStarted { number } => self.turn = *number,
            EventKind::ModelCallStarted { number } => {
                self.model_calls = *number;
                self.state = "model";
            }
            EventKind::ContextPrepared { snapshot } if self.show_context => {
                for line in snapshot.inspection_lines() {
                    self.line(&line, Ink::Normal);
                }
            }
            EventKind::ContextItemAdded { item } if self.show_context => {
                self.line(&item.inspection_line(), Ink::Normal);
            }
            EventKind::ContextSelected { selection } if self.show_context => {
                for line in selection.inspection_lines() {
                    self.line(&line, Ink::Normal);
                }
            }
            EventKind::ModelTextDelta { text } => {
                self.emit(text, Ink::Reply, true);
                self.text_open = true;
            }
            EventKind::ModelCallCompleted { .. } => {
                if self.text_open {
                    self.emit("\n\n", Ink::Normal, true);
                    self.text_open = false;
                }
            }
            EventKind::ModelCallFailed { message } => self.line(
                &format!("× model        response interrupted: {message}"),
                Ink::Normal,
            ),
            EventKind::ModelCallCancelled => {
                self.line("! model        response interrupted", Ink::Normal)
            }
            EventKind::ToolCallRequested { call } => {
                if let Some(id) = event.tool_call_id {
                    self.calls.insert(id, single_line(&call.name));
                }
                self.line(
                    &format!("● {:13} {}", single_line(&call.name), target(call)),
                    Ink::Normal,
                );
            }
            EventKind::PermissionsConfigured { policy } => {
                self.mode = astrid::permissions::PermissionMode::from_policy(*policy)
                    .map_or_else(|| "custom".into(), |mode| mode.to_string());
                self.line(
                    &format!(
                        "  policy       read={:?} write={:?} shell={:?}",
                        policy.read, policy.write, policy.execute
                    ),
                    Ink::Dim,
                );
            }
            EventKind::WorkspaceBaseline { git, complete, .. } => {
                self.line(
                    &format!(
                        "  workspace    branch={} · {} initial dirty paths · evidence {}",
                        git.branch.as_deref().unwrap_or(if git.detached {
                            "detached HEAD"
                        } else {
                            "unavailable"
                        }),
                        git.dirty.len(),
                        if *complete {
                            "complete within scope"
                        } else {
                            "incomplete"
                        }
                    ),
                    Ink::Dim,
                );
                for path in git.dirty.iter().take(20) {
                    self.line(
                        &format!(
                            "    {} · staged={} unstaged={} untracked={}",
                            path.path, path.staged, path.unstaged, path.untracked
                        ),
                        Ink::Dim,
                    );
                }
                if git.dirty.len() > 20 {
                    self.line(
                        "    additional dirty paths retained in runtime evidence",
                        Ink::Dim,
                    );
                }
                for error in &git.errors {
                    self.line(&format!("    Git: {error}"), Ink::Dim);
                }
            }
            EventKind::WorkspaceChanges { report } => {
                self.line(
                    &format!(
                        "  changes      {} observed paths · evidence {}",
                        report.changes.len(),
                        if report.complete {
                            "complete within scope"
                        } else {
                            "incomplete"
                        }
                    ),
                    Ink::Dim,
                );
                self.line(&report.attribution, Ink::Dim);
                for change in &report.changes {
                    self.line(
                        &format!("    {} · {}", change.path, change.kind),
                        Ink::Normal,
                    );
                    if let Some(patch) = &change.patch {
                        self.emit(patch, Ink::Normal, false);
                    }
                    if let Some(reason) = &change.unavailable {
                        self.line(&format!("    evidence unavailable: {reason}"), Ink::Dim);
                    }
                }
                for error in &report.errors {
                    self.line(&format!("    evidence: {error}"), Ink::Dim);
                }
            }
            EventKind::NativeMutationRecorded { evidence } => {
                self.line(
                    &format!(
                        "  mutation     {} · {}",
                        evidence.path,
                        if evidence.complete {
                            "before/after recorded"
                        } else {
                            "evidence incomplete"
                        }
                    ),
                    Ink::Dim,
                );
                if let Some(change) = &evidence.change {
                    if let Some(patch) = &change.patch {
                        self.emit(patch, Ink::Normal, false);
                    }
                    if let Some(reason) = &change.unavailable {
                        self.line(&format!("    evidence unavailable: {reason}"), Ink::Dim);
                    }
                }
                for error in &evidence.errors {
                    self.line(&format!("    evidence: {error}"), Ink::Dim);
                }
            }
            EventKind::PermissionPolicyEvaluated {
                capability,
                action,
                reason,
            } => {
                self.line(
                    &format!("  permission   {capability:?} {action:?} · {reason}"),
                    Ink::Dim,
                );
            }
            EventKind::ToolOutput { output } => {
                if let Some(id) = event.tool_call_id {
                    let key = (id, output.stream);
                    let text = self.decoders.entry(key).or_default().push(&output.bytes);
                    if !text.is_empty() {
                        if self.active_stream != Some(key) {
                            self.line(
                                &format!(
                                    "  {}",
                                    if output.stream == OutputStream::Stdout {
                                        "stdout"
                                    } else {
                                        "stderr"
                                    }
                                ),
                                Ink::Dim,
                            );
                        }
                        self.emit(&text, Ink::Normal, false);
                        self.tool_output_open = true;
                        self.active_stream = Some(key);
                    }
                }
            }
            EventKind::ToolCallStarted => self.state = "tool",
            EventKind::PermissionRequested { command, workspace } => {
                let shell = event
                    .tool_call_id
                    .and_then(|id| self.calls.get(&id))
                    .is_some_and(|name| name == "shell");
                let authority = if shell {
                    "Runs with your account's permissions."
                } else {
                    "Workspace operation subject to validated paths."
                };
                self.approval_authority = authority;
                self.line("? permission   operation approval required", Ink::Warning);
                // Show the complete command, not a truncated permission target.
                self.emit(
                    &format!(
                        "  cwd: {}\n  command: {}\n  {authority}\n",
                        single_line(workspace),
                        printable(command)
                    ),
                    Ink::Normal,
                    false,
                );
                self.waiting = true;
                self.state = "permission";
            }
            EventKind::PermissionGranted
            | EventKind::PermissionDenied
            | EventKind::PermissionCancelled
            | EventKind::PermissionFailed { .. } => {
                self.waiting = false;
                self.state = "running";
                match &event.kind {
                    EventKind::PermissionGranted => {
                        self.line("✓ permission   granted", Ink::Success)
                    }
                    EventKind::PermissionDenied => self.line("! permission   denied", Ink::Warning),
                    EventKind::PermissionFailed { message } => {
                        self.line(&format!("× permission   {message}"), Ink::Normal)
                    }
                    _ => self.line("! permission   cancelled", Ink::Normal),
                }
            }
            EventKind::ToolCallCompleted { outcome }
            | EventKind::ToolCallFailed { outcome }
            | EventKind::ToolCallDenied { outcome }
            | EventKind::ToolCallTimedOut { outcome } => {
                let name = event
                    .tool_call_id
                    .and_then(|id| self.calls.remove(&id))
                    .unwrap_or_else(|| "tool".into());
                let (symbol, state) = match &event.kind {
                    EventKind::ToolCallCompleted { .. } => ("✓", "completed"),
                    EventKind::ToolCallDenied { .. } => ("!", "denied"),
                    EventKind::ToolCallTimedOut { .. } => ("×", "timed out"),
                    _ => ("×", "failed"),
                };
                self.line(
                    &format!("{symbol} {state:13} {name} · {}", summary(outcome)),
                    if symbol == "✓" {
                        Ink::Success
                    } else {
                        Ink::Error
                    },
                );
                if name == "shell" {
                    self.command_output(outcome);
                } else if self.expanded_tool_calls {
                    self.result_preview(outcome);
                }
            }
            EventKind::ToolCallCancelled { .. } | EventKind::ToolCallSkipped { .. } => {
                let name = event
                    .tool_call_id
                    .and_then(|id| self.calls.remove(&id))
                    .unwrap_or_else(|| "tool".into());
                let detail = match &event.kind {
                    EventKind::ToolCallSkipped { reason } => format!("skipped · {reason}"),
                    _ => "cancelled".into(),
                };
                self.line(&format!("! {name:13} {detail}"), Ink::Error);
                if let EventKind::ToolCallCancelled { output: Some(data) } = &event.kind {
                    self.command_output(&ToolOutcome::Success { data: data.clone() });
                }
            }
            EventKind::CancellationRequested => {
                self.state = "cancelling";
                self.line("! run          cancellation requested", Ink::Normal);
            }
            EventKind::RunCompleted { .. } => {
                self.state = "completed";
                self.done = true;
                self.line("✓ run          completed", Ink::Success);
            }
            EventKind::RunCancelled => {
                self.state = "cancelled";
                self.done = true;
                self.waiting = false;
                self.line("! run          cancelled", Ink::Warning);
            }
            EventKind::RunFailed { code, message } => {
                self.state = "failed";
                self.done = true;
                self.line(&format!("× run          {code}: {message}"), Ink::Error);
            }
            EventKind::ModelCallLimitReached { limit, .. } => {
                self.state = "limit reached";
                self.done = true;
                self.line(
                    &format!(
                        "! run          model-call limit {limit}; final tool results uninspected"
                    ),
                    Ink::Normal,
                );
            }
            _ => {}
        }
    }
    fn result_preview(&mut self, outcome: &ToolOutcome) {
        if let ToolOutcome::Success { data } = outcome {
            let text = if let Some(lines) = data["lines"].as_array() {
                lines
                    .iter()
                    .map(|line| {
                        format!(
                            "{}: {}\n",
                            line["line"],
                            line["text"].as_str().unwrap_or("")
                        )
                    })
                    .collect::<String>()
            } else {
                data["content"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| serde_json::to_string_pretty(data).unwrap_or_default())
            };
            let text = printable(&text);
            for line in text.lines().take(12) {
                self.line(
                    &format!("    {}", super::format::truncate(line, 160)),
                    Ink::Dim,
                );
            }
            if text.lines().count() > 12 {
                self.line("    … additional output omitted from preview", Ink::Dim);
            }
        }
    }
    fn command_output(&mut self, outcome: &ToolOutcome) {
        if let ToolOutcome::Success { data } | ToolOutcome::TimedOut { data } = outcome {
            for stream in ["stdout", "stderr"] {
                let metadata = &data["output"][stream];
                let queued = metadata["live_queued_bytes"].as_u64().unwrap_or(0);
                if queued == 0
                    && let Some(text) = data[stream].as_str().filter(|text| !text.is_empty())
                {
                    self.line(&format!("  {stream}"), Ink::Dim);
                    self.emit(
                        &format!("{}\n", printable(text).trim_end()),
                        Ink::Normal,
                        false,
                    );
                }
                if metadata["truncated"] == true
                    || metadata["text_unavailable_suffix_bytes"]
                        .as_u64()
                        .unwrap_or(0)
                        > 0
                    || metadata["live_omitted_bytes"].as_u64().unwrap_or(0) > 0
                {
                    self.line(&format!("  {stream}: observed={} captured={} capture omitted={} live omitted={} text suffix unavailable={} complete={}",
                        metadata["observed_bytes"], metadata["captured_bytes"], metadata["capture_omitted_bytes"], metadata["live_omitted_bytes"], metadata["text_unavailable_suffix_bytes"], metadata["complete"]), Ink::Dim);
                }
            }
        }
        self.decoders.clear();
    }
}
fn target(call: &ToolCall) -> String {
    let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
    let parts: Vec<_> = ["command", "path", "pattern"]
        .iter()
        .filter_map(|key| args[key].as_str())
        .collect();
    if parts.is_empty() {
        "invalid or empty arguments".into()
    } else {
        parts.join(" · ")
    }
}
pub(super) fn summary(outcome: &ToolOutcome) -> String {
    match outcome {
        ToolOutcome::Error { code, message } => format!("{code}: {message}"),
        ToolOutcome::Success { data } | ToolOutcome::TimedOut { data } => {
            if data["timed_out"] == true {
                return if data["stdout"].is_null() && data["stderr"].is_null() {
                    "output unavailable after timeout".into()
                } else {
                    "partial output retained; unread output unavailable".into()
                };
            }
            if let Some(text) = data["content"].as_str() {
                let mut result = format!("{} lines", text.lines().count());
                if data["truncated"] == true {
                    result.push_str(&format!(
                        " · truncated ({})",
                        data["coverage"]["truncation_reason"]
                            .as_str()
                            .unwrap_or("limit")
                    ));
                }
                return result;
            }
            if let Some(bytes) = data["bytes_written"].as_u64() {
                return format!("{bytes} bytes written");
            }
            for (key, label) in [
                ("matches", "matches"),
                ("files", "files"),
                ("entries", "entries"),
            ] {
                if let Some(items) = data[key].as_array() {
                    let mut result = format!("{} {label}", items.len());
                    if let Some(offset) = data["page"]["next_offset"].as_u64() {
                        result.push_str(&format!(" · more results, next offset {offset}"));
                    } else if data["truncated"] == true {
                        result.push_str(" · incomplete coverage");
                    }
                    if items.iter().any(|item| item["context_truncated"] == true) {
                        result.push_str(" · context truncated");
                    }
                    return result;
                }
            }
            if let Some(code) = data["exit_code"].as_i64() {
                return format!("exit {code}");
            }
            if data.get("exit_code").is_some() {
                return "no exit code (signal)".into();
            }
            "result available".into()
        }
    }
}
