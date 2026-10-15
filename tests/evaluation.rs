//! Phase 6 execution baseline. Scripted inference is never a model-quality score.
mod support;

use astrid::{
    cancellation::Cancellation,
    context::{ContextBudget, SelectionPolicy},
    events::{EventKind, ExecutionEvent, ExecutionState},
    model::ToolCall,
    observability::{self, Options},
    permissions::{PermissionMode, PermissionPolicy},
    runtime::{self, RunConfig, RunOutcome},
    tools::{PermissionRequest, ToolError, ToolExecution, ToolExecutor, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};
use support::{Confirmation, Server, call, reply};

struct ObservedTools {
    real: Tools,
    dispatched: Mutex<Vec<String>>,
    cancel_after: Option<&'static str>,
}
#[async_trait]
impl ToolExecutor for ObservedTools {
    fn workspace(&self) -> &Workspace {
        self.real.workspace()
    }
    fn permission(&self, call: &ToolCall) -> Result<Option<PermissionRequest>, ToolError> {
        self.real.permission(call)
    }
    async fn execute(&self, call: &ToolCall, cancel: &Cancellation) -> ToolExecution {
        self.dispatched.lock().unwrap().push(call.call_id.clone());
        let result = self.real.execute(call, cancel).await;
        if self.cancel_after == Some(call.call_id.as_str()) {
            cancel.cancel();
        }
        result
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn outputs(request: &Value) -> Vec<Value> {
    request["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["type"] == "function_call_output")
        .map(|item| serde_json::from_str(item["output"].as_str().unwrap()).unwrap())
        .collect()
}
fn copy(root: &Path, fixture: &Path, name: &str) {
    let target = root.join(name);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::copy(fixture.join(name), target).unwrap();
}

#[tokio::test]
async fn reproducible_runtime_baseline() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rubric_bytes = include_bytes!("fixtures/evaluation/rubric.json");
    let rubric: Value = serde_json::from_slice(rubric_bytes).unwrap();
    let mut records = Vec::new();
    for selection_policy in [SelectionPolicy::FileReferences, SelectionPolicy::Recency] {
        for id in [
            "coding-repair",
            "notes-retrieval",
            "permission-denial",
            "cancellation",
            "context-compaction",
        ] {
            let root = tempfile::tempdir().unwrap();
            let trace_store = tempfile::tempdir().unwrap();
            let mut policy = PermissionPolicy::read_only();
            let mut budget = ContextBudget {
                policy: selection_policy,
                ..Default::default()
            };
            let mut cancel_after = None;
            let mut fixture_hashes = serde_json::Map::new();
            let replies = match id {
                "coding-repair" => {
                    let fixture = project.join("tests/fixtures/greeting");
                    for name in ["Cargo.toml", "AGENTS.md", "src/lib.rs"] {
                        copy(root.path(), &fixture, name);
                    }
                    fs::write(root.path().join("GREETING.txt"), "wrong\n").unwrap();
                    policy = PermissionMode::Unbound.policy();
                    vec![
                        reply(
                            &[call("read", "read_file", json!({"path":"GREETING.txt"}))],
                            "Inspect",
                        ),
                        reply(
                            &[
                                call(
                                    "edit",
                                    "edit_file",
                                    json!({"path":"GREETING.txt","old_text":"wrong","new_text":"Hello, Astrid!"}),
                                ),
                                call("verify", "shell", json!({"command":"cargo test --offline"})),
                            ],
                            "Repair",
                        ),
                        reply(&[], "SCRIPTED completion"),
                    ]
                }
                "notes-retrieval" => {
                    let fixture = project.join("tests/fixtures/personal-tasks/outstanding");
                    for name in ["house.txt", "garden.md"] {
                        copy(root.path(), &fixture, name);
                    }
                    vec![
                        reply(
                            &[
                                call("house", "read_file", json!({"path":"house.txt"})),
                                call("garden", "read_file", json!({"path":"garden.md"})),
                            ],
                            "Read sources",
                        ),
                        reply(&[], "SCRIPTED unscored checklist"),
                    ]
                }
                "permission-denial" => vec![
                    reply(
                        &[
                            call(
                                "write",
                                "write_file",
                                json!({"path":"forbidden","content":"bad","overwrite":false}),
                            ),
                            call("shell", "shell", json!({"command":"touch forbidden"})),
                        ],
                        "Attempt denied operations",
                    ),
                    reply(&[], "SCRIPTED denied"),
                ],
                "cancellation" => {
                    fs::write(root.path().join("note.txt"), "Buy lamp.\n").unwrap();
                    cancel_after = Some("first");
                    vec![reply(
                        &[
                            call("first", "read_file", json!({"path":"note.txt"})),
                            call(
                                "second",
                                "write_file",
                                json!({"path":"forbidden","content":"bad","overwrite":false}),
                            ),
                        ],
                        "Read then stop",
                    )]
                }
                "context-compaction" => {
                    fs::write(
                        root.path().join("target.rs"),
                        "target evidence ".repeat(600),
                    )
                    .unwrap();
                    fs::write(root.path().join("noise.rs"), "noise evidence ".repeat(600)).unwrap();
                    fs::write(root.path().join("latest.rs"), "latest").unwrap();
                    budget.estimated_context_tokens = 100_000;
                    budget.response_reserve_tokens = 1000;
                    budget.max_request_bytes = 28_000;
                    budget.max_summary_bytes = 1200;
                    vec![
                        reply(
                            &[call("old", "read_file", json!({"path":"target.rs"}))],
                            "Read target",
                        ),
                        reply(
                            &[call("noise", "read_file", json!({"path":"noise.rs"}))],
                            "Read noise",
                        ),
                        reply(
                            &[call("latest", "read_file", json!({"path":"latest.rs"}))],
                            "Read latest",
                        ),
                        reply(&[], "SCRIPTED done"),
                    ]
                }
                _ => unreachable!(),
            };
            // Hash the complete initial synthetic input, including nested coding files.
            for entry in walkdir::WalkDir::new(root.path())
                .sort_by_file_name()
                .into_iter()
                .map(Result::unwrap)
                .filter(|e| e.file_type().is_file())
            {
                fixture_hashes.insert(
                    entry
                        .path()
                        .strip_prefix(root.path())
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    json!(digest(&fs::read(entry.path()).unwrap())),
                );
            }
            let server = Server::start(replies).await;
            let tools = ObservedTools {
                real: Tools::new(
                    Workspace::new(root.path()).unwrap(),
                    Duration::from_secs(30),
                )
                .unwrap(),
                dispatched: Mutex::default(),
                cancel_after,
            };
            let config = RunConfig {
                routing: None,
                observability: Some(Options::new(trace_store.path().join("traces"))),
                context_budget: Some(budget.clone()),
                model: "scripted-phase-6".into(),
                task: format!("EVALUATION {id}: {}", rubric["tasks"][id].as_str().unwrap()),
                max_model_calls: 6,
                permissions: policy,
            };
            let (tx, mut rx) = tokio::sync::mpsc::channel(4);
            let mut confirmation = Confirmation::new(true);
            let started = Instant::now();
            let (result, events) = tokio::join!(
                runtime::run(
                    &server.provider,
                    &tools,
                    &mut confirmation,
                    config.clone(),
                    Cancellation::default(),
                    Some(tx)
                ),
                async {
                    let mut events = Vec::<ExecutionEvent>::new();
                    while let Some(event) = rx.recv().await {
                        events.push(event);
                    }
                    events
                }
            );
            let elapsed = started.elapsed();
            let result = result.unwrap();
            let expected_calls = match id {
                "coding-repair" => 3,
                "context-compaction" => 4,
                "cancellation" => 1,
                _ => 2,
            };
            // Assert before waiting for unused server responses on a runtime failure.
            assert_eq!(
                result.model_calls, expected_calls,
                "{id}: {:?}",
                result.outcome
            );
            let requests = server.finish().await;
            let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
            for event in &events {
                replay.transition(event).unwrap();
            }
            assert_eq!(replay, result.state);
            let selections: Vec<_> = events
                .iter()
                .filter_map(|e| match &e.kind {
                    EventKind::ContextSelected { selection } => Some(selection),
                    _ => None,
                })
                .collect();
            let compactions = selections.iter().filter(|s| s.summary.is_some()).count();
            let completed = result.outcome == RunOutcome::Completed;
            let passed = match id {
                "coding-repair" => {
                    completed
                        && fs::read_to_string(root.path().join("GREETING.txt")).unwrap()
                            == "Hello, Astrid!\n"
                        && outputs(requests.last().unwrap()).iter().any(|o| {
                            o["data"]["exit_code"] == 0
                                && o["data"]["stdout"]
                                    .as_str()
                                    .is_some_and(|s| s.contains("2 passed"))
                        })
                }
                "notes-retrieval" => {
                    completed
                        && outputs(&requests[1]).len() == 2
                        && ["house.txt", "garden.md"].iter().all(|name| {
                            let bytes = fs::read(root.path().join(name)).unwrap();
                            fixture_hashes[*name] == digest(&bytes)
                                && outputs(&requests[1]).iter().any(|o| {
                                    o["data"]["path"] == *name
                                        && o["data"]["content"]
                                            == String::from_utf8(bytes.clone()).unwrap()
                                        && o["data"]["coverage"]["complete"] == true
                                })
                        })
                }
                "permission-denial" => {
                    completed
                        && tools.dispatched.lock().unwrap().is_empty()
                        && confirmation.commands.is_empty()
                        && !root.path().join("forbidden").exists()
                        && events
                            .iter()
                            .filter(|e| matches!(e.kind, EventKind::ToolCallDenied { .. }))
                            .count()
                            == 2
                }
                "cancellation" => {
                    result.outcome == RunOutcome::Cancelled
                        && result.final_text.is_empty()
                        && *tools.dispatched.lock().unwrap() == ["first"]
                        && !root.path().join("forbidden").exists()
                        && events
                            .iter()
                            .any(|e| matches!(e.kind, EventKind::ToolCallSkipped { .. }))
                }
                "context-compaction" => {
                    completed
                        && compactions > 0
                        && selections.iter().all(|s| budget.admits(&s.snapshot))
                        && selections
                            .iter()
                            .any(|s| s.decisions.iter().any(|d| !d.retained))
                        && selections
                            .iter()
                            .filter_map(|s| s.summary.as_ref())
                            .all(|s| s.incomplete)
                        && requests.last().unwrap()["input"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|i| i["call_id"] == "latest")
                        && requests.last().unwrap()["input"]
                            .to_string()
                            .contains("EVALUATION context-compaction")
                }
                _ => false,
            };
            let summary = observability::read_trace(
                &trace_store.path().join("traces"),
                &result.recording.run_id.to_string(),
            )
            .unwrap()
            .summary();
            let record = json!({"task":id,"criterion":rubric["tasks"][id],"task_success":passed,"run_outcome":format!("{:?}", result.outcome),"evidence":"scripted inference / real tools","config":{"model":config.model,"permissions":config.permissions,"context_budget":budget,"max_model_calls":config.max_model_calls},"fixture_sha256":fixture_hashes,"wall_us":observability::micros(elapsed),"model_calls":result.model_calls,"tool_outcomes":result.tool_calls,"tool_requests":events.iter().filter(|e| matches!(e.kind, EventKind::ToolCallRequested { .. })).count(),"tool_dispatches":tools.dispatched.lock().unwrap().len(),"context_compactions":compactions,"trace_summary":summary});
            println!("{id}: {}", if passed { "PASS" } else { "FAIL" });
            records.push(record);
        }
    }
    let report = json!({"schema":1,"suite":"phase-6-v2-policy-comparison","rubric_sha256":digest(rubric_bytes),"runner_sha256":digest(include_bytes!("evaluation.rs")),"evidence":"deterministic execution baseline; model quality unscored","records":records});
    if let Some(path) = std::env::var_os("ASTRID_EVAL_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert!(
        records.iter().all(|r| r["task_success"] == true),
        "{report}"
    );
}
