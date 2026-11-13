//! `astrid presence-ask`: a JSON-lines client of the runtime for the presence
//! app's ask surface (ADR 0033). One fresh read-only run per ask.
//!
//! stdin:  {"type":"ask","text":...} | {"type":"approve","id":n,"approve":bool} | {"type":"cancel"}
//! stdout: {"type":"ready"} | {"type":"state","state":..,"label":..} | {"type":"delta","text":..}
//!         | {"type":"approval","id":n,"title":..,"detail":..} | {"type":"answer","text":..,"brief":{..}|null}
//!         | {"type":"error","message":..} | {"type":"done","outcome":..}
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

/// Asks the model to finish with a machine-readable brief the UI renders.
pub const BRIEF_INSTRUCTIONS: &str = "You are answering inside Astrid's on-screen ask panel. Use the mail tools when the question is about email. Keep it short and concrete. End your reply with exactly one fenced code block tagged `brief` containing JSON: {\"title\": short headline, \"summary\": one sentence, \"todos\": [{\"text\": action the user should take, \"source\": n}], \"updates\": [{\"text\": what happened, \"source\": n}], \"sources\": [{\"id\": n, \"kind\": \"gmail\" or \"apple\", \"from\": sender name, \"subject\": subject, \"date\": short date, \"message_id\": id from mail_search}]}. Number sources from 1; omit empty lists; every todo and update cites a source when one exists. Do not invent emails.";

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
    let (asks_tx, mut asks_rx) = tokio::sync::mpsc::channel::<String>(4);
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
                            let _ = asks_tx.blocking_send(text);
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
    while let Some(ask) = asks_rx.recv().await {
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
            task: format!("{ask}\n\n{BRIEF_INSTRUCTIONS}"),
            max_model_calls: 8,
            permissions: PermissionPolicy::read_only(),
        };
        let forward = async {
            while let Some(event) = events_rx.recv().await {
                if let Some((state, label)) = state_for(&event.kind) {
                    emit(json!({"type":"state","state":state,"label":label}));
                }
                if let EventKind::ModelTextDelta { text } = &event.kind {
                    emit(json!({"type":"delta","text":text}));
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
                        let (text, brief) = split_brief(&result.final_text);
                        emit(json!({"type":"answer","text":text,"brief":brief}));
                        "completed".to_string()
                    }
                    RunOutcome::Cancelled => "cancelled".into(),
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
