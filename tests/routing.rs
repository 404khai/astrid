//! Scripted provider/real-tool acceptance, never model-quality evidence.
mod support;
use astrid::{
    cancellation::Cancellation,
    context::ContextBudget,
    events::{EventKind, ExecutionState},
    model::{ModelProvider, ModelRequest},
    observability::{self, Options},
    routing::{ModelClass, ModelRoute, ReasoningEffort, ReplayMode, RequestOptions, RoutingPolicy},
    runtime::{self, RunConfig, RunOutcome},
    tools::Tools,
    workspace::Workspace,
};
use serde_json::json;
use std::{fs, time::Duration};
use support::{Confirmation, Server, call, reply};

fn policy() -> RoutingPolicy {
    RoutingPolicy {
        initial_effort: Some(ReasoningEffort::Low),
        stages: vec![ModelRoute {
            from_call: 2,
            model: "gpt-5.6-terra".into(),
            class: ModelClass::Coding,
            effort: Some(ReasoningEffort::High),
        }],
    }
}
fn config(store: &std::path::Path) -> RunConfig {
    RunConfig {
        routing: Some(policy()),
        observability: Some(Options::new(store.join("traces"))),
        model: "gpt-5.6-sol".into(),
        task: "Inspect note.txt, report evidence; never mutate.".into(),
        max_model_calls: 4,
        permissions: astrid::permissions::PermissionPolicy::read_only(),
        context_budget: Some(ContextBudget {
            max_request_bytes: 20_000,
            ..Default::default()
        }),
    }
}
fn with_private_reasoning(mut events: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    events.last_mut().unwrap()["response"]["output"].as_array_mut().unwrap().insert(0,
        json!({"type":"reasoning","id":"private-item","encrypted_content":"PRIVATE-ROUTE-SENTINEL".repeat(2000),"summary":[]}));
    events
}

#[tokio::test]
async fn routed_run_preserves_private_session_denies_mutations_and_accounts_exact_wire_view() {
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "Buy lamp.\n").unwrap();
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(5)).unwrap();
    let server = Server::start(vec![
        with_private_reasoning(reply(
            &[call("read", "read_file", json!({"path":"note.txt"}))],
            "Visible inspection",
        )),
        reply(
            &[
                call(
                    "deny-write",
                    "write_file",
                    json!({"path":"forbidden","content":"bad","overwrite":false}),
                ),
                call("deny-shell", "shell", json!({"command":"touch forbidden"})),
            ],
            "Visible diagnosis",
        ),
        reply(&[], "SCRIPTED source: note.txt; denied mutations."),
    ])
    .await;
    let (tx, mut rx) = tokio::sync::mpsc::channel(2);
    let mut handler = Confirmation::new(true);
    let (result, events) = tokio::join!(
        runtime::run(
            &server.provider,
            &tools,
            &mut handler,
            config(store.path()),
            Cancellation::default(),
            Some(tx)
        ),
        async {
            let mut events = Vec::new();
            while let Some(event) = rx.recv().await {
                events.push(event);
            }
            events
        }
    );
    let result = result.unwrap();
    assert_eq!(result.outcome, RunOutcome::Completed);
    assert_eq!(result.model_calls, 3);
    assert!(handler.commands.is_empty());
    assert!(!root.path().join("forbidden").exists());
    assert_eq!(
        fs::read_to_string(root.path().join("note.txt")).unwrap(),
        "Buy lamp.\n"
    );
    let lengths = server.request_bytes.lock().unwrap().clone();
    let requests = server.finish().await;
    assert_eq!(
        requests
            .iter()
            .map(|r| r["model"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-terra"]
    );
    assert_eq!(requests[0]["reasoning"]["effort"], "low");
    assert_eq!(requests[1]["reasoning"]["effort"], "high");
    assert!(
        requests[1]["input"]
            .to_string()
            .contains("Visible inspection")
    );
    assert!(requests[1]["input"].to_string().contains("Buy lamp."));
    assert!(
        requests[1]["input"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["type"] == "function_call" && i["call_id"] == "read")
    );
    assert!(
        requests
            .iter()
            .all(|r| !r.to_string().contains("PRIVATE-ROUTE-SENTINEL")
                && !r.to_string().contains("private-item"))
    );
    let snapshots: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let EventKind::ContextPrepared { snapshot } = &e.kind {
                Some(snapshot.serialized_request_bytes)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(snapshots, lengths);
    assert!(snapshots.iter().all(|bytes| *bytes <= 20_000));
    let routes: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let EventKind::ModelRouteSelected { decision } = &e.kind {
                Some(decision)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(routes.len(), 3);
    assert_eq!(routes[1].class, ModelClass::Coding);
    assert!(
        routes
            .iter()
            .all(|r| r.replay == ReplayMode::VisibleHistory)
    );
    let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
    for event in &events {
        replay.transition(event).unwrap();
    }
    assert_eq!(replay, result.state);
    // A duplicate route is rejected atomically, without changing replay state.
    let index = events
        .iter()
        .position(|e| matches!(e.kind, EventKind::ModelRouteSelected { .. }))
        .unwrap();
    let mut partial = ExecutionState::new(result.session.id, result.state.run_id);
    for event in &events[..=index] {
        partial.transition(event).unwrap();
    }
    let saved = partial.clone();
    let mut duplicate = events[index].clone();
    duplicate.sequence += 1;
    assert!(partial.transition(&duplicate).is_err());
    assert_eq!(partial, saved);
    let summary = observability::read_trace(
        &store.path().join("traces"),
        &result.recording.run_id.to_string(),
    )
    .unwrap()
    .summary();
    assert!(summary.complete);
    assert!(summary.models.values().all(|m| m.route.is_some()
        && m.inclusive_runtime_us.is_some()
        && m.usage.as_ref().is_none_or(|u| u.input_tokens.is_none()
            && u.output_tokens.is_none()
            && u.cached_tokens.is_none())));
    assert!(summary.cost.is_none());
    assert!(
        !serde_json::to_string(&summary)
            .unwrap()
            .contains("PRIVATE-ROUTE-SENTINEL")
    );
    if let Some(path) = std::env::var_os("ASTRID_ROUTING_REPORT") {
        use sha2::{Digest, Sha256};
        let report = json!({
            "evidence":"scripted inference / real tools; model quality unscored",
            "baseline":"2deea5f plus preserved headless work and Phase 7 edits",
            "runner_sha256":format!("{:x}",Sha256::digest(include_bytes!("routing.rs"))),
            "configuration":policy(),
            "initial_model":"gpt-5.6-sol",
            "request_bytes":lengths,
            "checks":{"whole_exchange_visible_handoff":true,"private_reasoning_excluded":true,"private_original_history_retained":true,"permission_denials":2,"mutations":0,"event_replay_matches":true,"duplicate_route_rejected":true},
            "task_quality":null,
            "trace_summary":summary
        });
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert_eq!(result.session.replay_mode(), ReplayMode::VisibleHistory);
    assert_eq!(result.session.dispatch_origins().len(), 3);
    let original = serde_json::to_string(&result.session.messages).unwrap();
    assert!(original.contains("PRIVATE-ROUTE-SENTINEL"));
    let serialized = serde_json::to_string(&result.session).unwrap();
    assert!(serialized.contains("PRIVATE-ROUTE-SENTINEL"));
    use astrid::session_store::{SessionSnapshot, SessionStore, StoredSession};
    let idle_store = SessionStore::open(store.path(), tools.workspace()).unwrap();
    let snapshot = SessionSnapshot::new(
        tools.workspace(),
        0,
        vec![StoredSession {
            session: result.session,
            runs: 1,
            outcome: Some(result.outcome),
        }],
    );
    idle_store.save(&snapshot).unwrap();
    let restored = idle_store
        .load()
        .unwrap()
        .unwrap()
        .entries
        .remove(0)
        .session;
    assert_eq!(serde_json::to_string(&restored.messages).unwrap(), original);
    let followup = Server::start(vec![reply(&[], "SCRIPTED followup")]).await;
    let mut next = config(store.path());
    next.routing = None;
    next.task = "Follow up without tools".into();
    let result = runtime::run_in_session(
        &followup.provider,
        &tools,
        &mut handler,
        next,
        Cancellation::default(),
        None,
        restored,
    )
    .await
    .unwrap();
    assert_eq!(result.outcome, RunOutcome::Completed);
    assert_eq!(result.tool_calls, 0);
    assert_eq!(result.session.replay_mode(), ReplayMode::VisibleHistory);
    let requests = followup.finish().await;
    assert!(!requests[0].to_string().contains("PRIVATE-ROUTE-SENTINEL"));
    assert!(requests[0].to_string().contains("permission_denied"));
}

#[tokio::test]
async fn unsupported_effort_and_invalid_stage_order_fail_before_any_request_or_mutation() {
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(5)).unwrap();
    let server = Server::start(vec![]).await;
    let mut unsupported_initial = config(store.path());
    unsupported_initial.routing.as_mut().unwrap().initial_effort = Some(ReasoningEffort::Minimal);
    let mut unsupported_stage = config(store.path());
    unsupported_stage.routing.as_mut().unwrap().stages[0].effort = Some(ReasoningEffort::Minimal);
    let mut unknown_model = config(store.path());
    unknown_model.model = "unknown-model".into();
    for cfg in [unsupported_initial, unsupported_stage, unknown_model] {
        assert!(
            runtime::run(
                &server.provider,
                &tools,
                &mut Confirmation::new(true),
                cfg,
                Cancellation::default(),
                None
            )
            .await
            .is_err()
        );
    }
    let mut bad = policy();
    bad.stages[0].from_call = 1;
    assert!(!bad.valid("gpt-5.6-sol", 4, &server.provider));
    bad.stages[0].from_call = 5;
    assert!(!bad.valid("gpt-5.6-sol", 4, &server.provider));
    bad.stages[0].from_call = 2;
    bad.stages.push(bad.stages[0].clone());
    assert!(!bad.valid("gpt-5.6-sol", 4, &server.provider));
    assert!(!policy().valid("unknown-model", 4, &server.provider));
    let request = ModelRequest {
        options: RequestOptions {
            instabox_exec: false,
            computer_use: false,
            effort: Some(ReasoningEffort::High),
            replay: ReplayMode::ProviderContinuation,
        },
        model: "unknown-model",
        instructions: "test",
        messages: &[],
    };
    assert!(server.provider.measure_request(&request).is_err());
    let mut sink = |_: &str| Ok(());
    assert!(server.provider.generate(&request, &mut sink).await.is_err());
    assert!(server.finish().await.is_empty());
}

#[tokio::test]
async fn final_response_does_not_spawn_a_configured_later_stage_and_precancel_does_no_inference() {
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(5)).unwrap();
    let server = Server::start(vec![reply(&[], "SCRIPTED final")]).await;
    let result = runtime::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config(store.path()),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(result.model_calls, 1);
    assert_eq!(result.outcome, RunOutcome::Completed);
    assert_eq!(server.finish().await.len(), 1);
    let server = Server::start(vec![]).await;
    let cancel = Cancellation::default();
    cancel.cancel();
    let result = runtime::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config(store.path()),
        cancel,
        None,
    )
    .await
    .unwrap();
    assert_eq!(result.model_calls, 0);
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert!(server.finish().await.is_empty());
}

#[tokio::test]
async fn effort_only_preserves_native_continuation_and_omission_sends_no_effort() {
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "note").unwrap();
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(5)).unwrap();
    for effort in [
        None,
        Some(ReasoningEffort::Low),
        Some(ReasoningEffort::Disabled),
    ] {
        let server = Server::start(vec![
            with_private_reasoning(reply(
                &[call("read", "read_file", json!({"path":"note.txt"}))],
                "visible",
            )),
            reply(&[], "done"),
        ])
        .await;
        let mut cfg = config(store.path());
        cfg.routing = Some(RoutingPolicy {
            initial_effort: effort,
            stages: vec![],
        });
        cfg.context_budget = Some(ContextBudget::default());
        let result = runtime::run(
            &server.provider,
            &tools,
            &mut Confirmation::new(false),
            cfg,
            Cancellation::default(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.outcome, RunOutcome::Completed);
        let requests = server.finish().await;
        assert!(requests[1].to_string().contains("PRIVATE-ROUTE-SENTINEL"));
        assert_eq!(requests[1].get("reasoning").is_some(), effort.is_some());
    }
}

#[tokio::test]
async fn cancellation_in_a_routed_batch_skips_side_effects_and_incomplete_session_cannot_resume() {
    use astrid::{
        model::ToolCall,
        tools::{PermissionRequest, ToolError, ToolExecution, ToolExecutor},
    };
    struct CancelAfterRead(Tools);
    #[async_trait::async_trait]
    impl ToolExecutor for CancelAfterRead {
        fn workspace(&self) -> &Workspace {
            self.0.workspace()
        }
        fn permission(&self, call: &ToolCall) -> Result<Option<PermissionRequest>, ToolError> {
            self.0.permission(call)
        }
        async fn execute(&self, call: &ToolCall, cancel: &Cancellation) -> ToolExecution {
            let result = self.0.execute(call, cancel).await;
            cancel.cancel();
            result
        }
    }
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "note").unwrap();
    let tools = CancelAfterRead(
        Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(5)).unwrap(),
    );
    let server = Server::start(vec![reply(
        &[
            call("read", "read_file", json!({"path":"note.txt"})),
            call(
                "write",
                "write_file",
                json!({"path":"forbidden","content":"bad","overwrite":false}),
            ),
        ],
        "read",
    )])
    .await;
    let result = runtime::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(true),
        config(store.path()),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert_eq!(result.model_calls, 1);
    assert!(!root.path().join("forbidden").exists());
    let rejected = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut Confirmation::new(true),
        config(store.path()),
        Cancellation::default(),
        None,
        result.session,
    )
    .await
    .unwrap_err();
    assert!(rejected.message.contains("incomplete"));
    assert_eq!(server.finish().await.len(), 1);
}

#[tokio::test]
async fn selected_stage_failure_is_terminal_observable_and_never_falls_back() {
    use astrid::model::{ModelError, ModelResponse, TextSink};
    struct RejectStage<'a>(&'a dyn ModelProvider);
    #[async_trait::async_trait]
    impl ModelProvider for RejectStage<'_> {
        fn provider_name(&self) -> Option<&'static str> {
            self.0.provider_name()
        }
        fn capabilities(&self, model: &str) -> astrid::routing::ModelCapability {
            self.0.capabilities(model)
        }
        fn measure_request(
            &self,
            request: &ModelRequest<'_>,
        ) -> Result<Option<astrid::context::ContextSnapshot>, ModelError> {
            self.0.measure_request(request)
        }
        async fn generate(
            &self,
            request: &ModelRequest<'_>,
            sink: &mut dyn TextSink,
        ) -> Result<ModelResponse, ModelError> {
            if request.model == "gpt-5.6-terra" {
                Err(ModelError::Provider("scripted stage unavailable".into()))
            } else {
                self.0.generate(request, sink).await
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "note").unwrap();
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(5)).unwrap();
    let server = Server::start(vec![reply(
        &[call("read", "read_file", json!({"path":"note.txt"}))],
        "inspect",
    )])
    .await;
    let result = runtime::run(
        &RejectStage(&server.provider),
        &tools,
        &mut Confirmation::new(false),
        config(store.path()),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    assert!(matches!(result.outcome, RunOutcome::Failed { .. }));
    assert_eq!(result.model_calls, 2);
    assert_eq!(server.finish().await.len(), 1);
    let summary = observability::read_trace(
        &store.path().join("traces"),
        &result.recording.run_id.to_string(),
    )
    .unwrap()
    .summary();
    let failed = summary
        .models
        .values()
        .find(|m| m.outcome.as_deref() == Some("model_call_failed"))
        .unwrap();
    assert_eq!(
        failed.route.as_ref().unwrap()["effective_model"],
        "gpt-5.6-terra"
    );
    assert!(summary.complete);
    assert!(summary.cost.is_none());
}
