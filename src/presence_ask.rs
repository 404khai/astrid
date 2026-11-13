//! `astrid presence-ask`: a JSON-lines client of the runtime for the presence
//! app's ask surface (ADR 0033). One fresh read-only run per ask.
//!
//! stdin:  {"type":"ask","text":...,"history"?:[{"role":"user"|"astrid","text":..}]}
//!         | {"type":"approve","id":n,"approve":bool} | {"type":"cancel"}
//! stdout: {"type":"ready"} | {"type":"state","state":..,"label":..} | {"type":"delta","text":..}
//!         | {"type":"approval","id":n,"title":..,"detail":..} | {"type":"answer","text":..,"brief":{..}|null}
//!         | {"type":"canvas","event":"begin"|"shape"|"end",..} (ADR 0036)
//!         | {"type":"error","message":..} | {"type":"done","outcome":..}
use crate::presence_canvas::{CANVAS_INSTRUCTIONS, CanvasStream};
use astrid::{
    cancellation::Cancellation,
    events::EventKind,
    model::ModelProvider,
    permissions::PermissionPolicy,
    runtime::{self, PermissionHandler, RunConfig, RunOutcome},
    tools::{PermissionRequest, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{self, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

const MAX_ASK: usize = 4000;
/// Bounds on the earlier turns a desktop chat sends with a follow-up (ADR 0037).
const MAX_HISTORY_TURNS: usize = 12;
const MAX_HISTORY_TURN: usize = 2000;
const MAX_HISTORY: usize = 12_000;

/// One earlier turn of a chat: who said it, and what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub user: bool,
    pub text: String,
}

/// Reads an ask's optional `history`, keeping the most recent turns that fit
/// the bounds. Malformed entries are skipped; long turns are cut.
pub fn history_from(value: &Value) -> Vec<Turn> {
    let Some(list) = value.as_array() else {
        return Vec::new();
    };
    let mut turns = Vec::new();
    let mut total = 0;
    for entry in list.iter().rev() {
        let user = match entry["role"].as_str() {
            Some("user") => true,
            Some("astrid") => false,
            _ => continue,
        };
        let text: String = entry["text"]
            .as_str()
            .unwrap_or("")
            .trim()
            .chars()
            .take(MAX_HISTORY_TURN)
            .collect();
        if text.is_empty() {
            continue;
        }
        let size = text.chars().count();
        if turns.len() == MAX_HISTORY_TURNS || total + size > MAX_HISTORY {
            break;
        }
        total += size;
        turns.push(Turn { user, text });
    }
    turns.reverse();
    turns
}

/// The run's task: earlier turns as context (never as instructions to act on),
/// then the ask and the surface's instructions. With no history this is the
/// same task the presence ask panel has always sent.
pub fn compose_task(ask: &str, history: &[Turn]) -> String {
    let mut task = String::new();
    if !history.is_empty() {
        task.push_str("Earlier in this conversation (context only):\n");
        for turn in history {
            let who = if turn.user { "User" } else { "Astrid" };
            task.push_str(&format!("{who}: {}\n", turn.text));
        }
        task.push_str("\nThe user now asks:\n");
    }
    task.push_str(&format!(
        "{ask}\n\n{BRIEF_INSTRUCTIONS}\n\n{CANVAS_INSTRUCTIONS}"
    ));
    task
}

/// Asks the model to finish with a machine-readable brief the UI renders.
pub const BRIEF_INSTRUCTIONS: &str = "You are answering inside Astrid's on-screen ask panel. Use the mail tools when the question is about email. Keep it short and concrete. Unless you are drawing on the canvas, end your reply with exactly one fenced code block tagged `brief` containing JSON: {\"title\": short headline, \"summary\": one sentence, \"todos\": [{\"text\": action the user should take, \"source\": n}], \"updates\": [{\"text\": what happened, \"source\": n}], \"sources\": [{\"id\": n, \"kind\": \"gmail\" or \"apple\", \"from\": sender name, \"subject\": subject, \"date\": short date, \"message_id\": id from mail_search}]}. Number sources from 1; omit empty lists; every todo and update cites a source when one exists. Do not invent emails.";

fn emit(value: Value) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}

type Pending = Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<bool>>>>;

/// Sends each approval to the presence UI and waits for its answer.
struct UiApprovals {
    pending: Pending,
    next: u64,
}
#[async_trait]
impl PermissionHandler for UiApprovals {
    async fn decide(&mut self, request: &PermissionRequest) -> io::Result<bool> {
        self.next += 1;
        let id = self.next;
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, tx);
        let detail: Value =
            serde_json::from_str(&request.command).unwrap_or(json!({"text": request.command}));
        let title = match detail["capability"].as_str() {
            Some("mail.read_only") if detail["operation"] == "search" => format!(
                "Search {} for “{}”?",
                if detail["source"] == "gmail" {
                    "Gmail"
                } else {
                    "Apple Mail"
                },
                detail["arguments"]["query"].as_str().unwrap_or("")
            ),
            Some("mail.read_only") => format!(
                "Read one {} message?",
                if detail["source"] == "gmail" {
                    "Gmail"
                } else {
                    "Apple Mail"
                }
            ),
            _ => "Allow this step?".into(),
        };
        emit(json!({"type":"approval","id":id,"title":title,"detail":detail}));
        match tokio::time::timeout(Duration::from_secs(300), rx).await {
            Ok(Ok(answer)) => Ok(answer),
            _ => Ok(false),
        }
    }
}

/// Extracts the trailing ```brief JSON block, if the model produced one.
pub fn split_brief(text: &str) -> (String, Option<Value>) {
    let Some(start) = text.rfind("```brief") else {
        return (text.trim().to_owned(), None);
    };
    let rest = &text[start + "```brief".len()..];
    let Some(end) = rest.find("```") else {
        return (text.trim().to_owned(), None);
    };
    match serde_json::from_str::<Value>(rest[..end].trim()) {
        Ok(brief) if brief.is_object() => (text[..start].trim().to_owned(), Some(brief)),
        _ => (text.trim().to_owned(), None),
    }
}

/// Maps runtime events to agent states the UI shows with ThinkingOrb.
fn state_for(kind: &EventKind) -> Option<(&'static str, String)> {
    match kind {
        EventKind::ModelCallStarted { .. } => Some(("working", "Thinking".into())),
        EventKind::ModelFirstTextDelta => Some(("composing", "Writing".into())),
        EventKind::ToolCallRequested { call } => Some(match call.name.as_str() {
            "mail_search" => ("searching", "Searching mail".into()),
            "mail_read" => ("weaving", "Reading an email".into()),
            _ => ("searching", "Looking".into()),
        }),
        EventKind::PermissionRequested { .. } => Some(("listening", "Waiting for you".into())),
        _ => None,
    }
}

pub async fn serve(
    provider: Box<dyn ModelProvider>,
    tools: Tools,
    model: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let pending: Pending = Arc::default();
    let (asks_tx, mut asks_rx) = tokio::sync::mpsc::channel::<(String, Vec<Turn>)>(4);
    let cancel_slot: Arc<Mutex<Option<Cancellation>>> = Arc::default();
    {
        let pending = pending.clone();
        let cancel_slot = cancel_slot.clone();
        // Plain thread: blocking stdin reads never stall the runtime.
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in io::stdin().lock().lines() {
                let Ok(line) = line else { break };
                if line.len() > 64 * 1024 {
                    continue;
                }
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                match message["type"].as_str() {
                    Some("ask") => {
                        let text = message["text"]
                            .as_str()
                            .unwrap_or("")
                            .trim()
                            .chars()
                            .take(MAX_ASK)
                            .collect::<String>();
                        if !text.is_empty() {
                            let history = history_from(&message["history"]);
                            let _ = asks_tx.blocking_send((text, history));
                        }
                    }
                    Some("approve") => {
                        let id = message["id"].as_u64().unwrap_or(0);
                        if let Some(tx) = pending
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .remove(&id)
                        {
                            let _ = tx.send(message["approve"] == true);
                        }
                    }
                    Some("cancel") => {
                        if let Some(c) = cancel_slot
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .as_ref()
                        {
                            c.cancel();
                        }
                    }
                    _ => {}
                }
            }
            // stdin closed: cancel any running ask so the process can exit.
            if let Some(c) = cancel_slot
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
                c.cancel();
            }
        });
    }
    emit(json!({"type":"ready","model":model}));
    while let Some((ask, history)) = asks_rx.recv().await {
        let cancel = Cancellation::default();
        *cancel_slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(cancel.clone());
        let (events_tx, mut events_rx) =
            tokio::sync::mpsc::channel::<astrid::events::ExecutionEvent>(256);
        let mut approvals = UiApprovals {
            pending: pending.clone(),
            next: 0,
        };
        let config = RunConfig {
            routing: None,
            observability: None,
            context_budget: Some(Default::default()),
            model: model.clone(),
            task: compose_task(&ask, &history),
            max_model_calls: 8,
            permissions: PermissionPolicy::read_only(),
        };
        let mut canvas = CanvasStream::default();
        let forward = async {
            while let Some(event) = events_rx.recv().await {
                if let Some((state, label)) = state_for(&event.kind) {
                    emit(json!({"type":"state","state":state,"label":label}));
                }
                if let EventKind::ModelTextDelta { text } = &event.kind {
                    emit(json!({"type":"delta","text":text}));
                    canvas.feed(text).into_iter().for_each(emit);
                }
            }
        };
        let run = runtime::run(
            provider.as_ref(),
            &tools,
            &mut approvals,
            config,
            cancel.clone(),
            Some(events_tx),
        );
        let (result, ()) = tokio::join!(run, forward);
        match result {
            Ok(result) => {
                let outcome = match &result.outcome {
                    RunOutcome::Completed => {
                        // Providers that don't stream text still get their drawing.
                        if !canvas.started() {
                            canvas.feed(&result.final_text).into_iter().for_each(emit);
                        }
                        canvas.finish().into_iter().for_each(emit);
                        let (text, brief) =
                            split_brief(&crate::presence_canvas::strip(&result.final_text));
                        emit(
                            json!({"type":"answer","text":text,"brief":brief,"canvas":canvas.started()}),
                        );
                        "completed".to_string()
                    }
                    RunOutcome::Cancelled => {
                        emit(json!({"type":"canvas","event":"cancel"}));
                        "cancelled".into()
                    }
                    RunOutcome::Failed { code, message } => {
                        emit(json!({"type":"error","message":format!("{code}: {message}")}));
                        "failed".into()
                    }
                    RunOutcome::ModelCallLimitReached { .. } => {
                        emit(
                            json!({"type":"error","message":"Astrid hit its step limit before finishing."}),
                        );
                        "limit".into()
                    }
                };
                emit(json!({"type":"done","outcome":outcome}));
            }
            Err(error) => emit(json!({"type":"error","message":error.to_string()})),
        }
        emit(json!({"type":"state","state":"idle","label":"Ready"}));
        *cancel_slot.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
    Ok(())
}

/// The empty, Astrid-owned workspace the ask surface runs in.
pub fn workspace() -> Result<Workspace, Box<dyn std::error::Error>> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    let dir = std::path::PathBuf::from(home)
        .join("Library/Application Support/Astrid/presence-workspace");
    std::fs::create_dir_all(&dir)?;
    Ok(Workspace::new(dir)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn briefs_are_split_from_the_reply_and_bad_json_falls_back_to_text() {
        let reply = "Here's your inbox.\n```brief\n{\"title\":\"Two things\",\"sources\":[{\"id\":1,\"kind\":\"gmail\"}]}\n```";
        let (text, brief) = split_brief(reply);
        assert_eq!(text, "Here's your inbox.");
        assert_eq!(brief.unwrap()["title"], "Two things");
        let (text, brief) = split_brief("Plain answer.\n```brief\n{not json}\n```");
        assert!(brief.is_none() && text.contains("Plain answer."));
        assert_eq!(split_brief("No block").0, "No block");
    }

    #[test]
    fn history_keeps_recent_bounded_turns_and_skips_bad_entries() {
        let history = history_from(&json!([
            {"role": "user", "text": "  draw a pump  "},
            {"role": "system", "text": "ignored"},
            {"role": "astrid", "text": ""},
            {"role": "astrid", "text": "I drew a pump."},
            {"text": "no role"},
            7
        ]));
        assert_eq!(
            history,
            vec![
                Turn {
                    user: true,
                    text: "draw a pump".into()
                },
                Turn {
                    user: false,
                    text: "I drew a pump.".into()
                },
            ]
        );
        assert!(history_from(&Value::Null).is_empty());

        let many: Vec<Value> = (0..40)
            .map(|n| json!({"role": "user", "text": format!("turn {n}")}))
            .collect();
        let kept = history_from(&Value::Array(many));
        assert_eq!(kept.len(), MAX_HISTORY_TURNS);
        assert_eq!(kept.last().unwrap().text, "turn 39");

        let long: Vec<Value> = (0..12)
            .map(|_| json!({"role": "astrid", "text": "x".repeat(5000)}))
            .collect();
        let kept = history_from(&Value::Array(long));
        assert!(
            kept.iter()
                .all(|t| t.text.chars().count() == MAX_HISTORY_TURN)
        );
        assert!(kept.iter().map(|t| t.text.len()).sum::<usize>() <= MAX_HISTORY);
    }

    #[test]
    fn tasks_without_history_are_unchanged_and_history_comes_first() {
        let plain = compose_task("hi", &[]);
        assert_eq!(
            plain,
            format!("hi\n\n{BRIEF_INSTRUCTIONS}\n\n{CANVAS_INSTRUCTIONS}")
        );
        let task = compose_task(
            "make it bigger",
            &[
                Turn {
                    user: true,
                    text: "draw a pump".into(),
                },
                Turn {
                    user: false,
                    text: "Here is a pump.".into(),
                },
            ],
        );
        assert!(task.starts_with(
            "Earlier in this conversation (context only):\nUser: draw a pump\nAstrid: Here is a pump.\n\nThe user now asks:\nmake it bigger\n\n"
        ));
    }

    #[test]
    fn mail_events_map_to_agent_states() {
        let call = |name: &str| EventKind::ToolCallRequested {
            call: astrid::model::ToolCall {
                call_id: "c".into(),
                name: name.into(),
                arguments: "{}".into(),
            },
        };
        assert_eq!(state_for(&call("mail_search")).unwrap().0, "searching");
        assert_eq!(state_for(&call("mail_read")).unwrap().0, "weaving");
        assert_eq!(
            state_for(&EventKind::ModelFirstTextDelta).unwrap().0,
            "composing"
        );
    }
}
