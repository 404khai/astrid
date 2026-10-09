mod support;
use astrid::{
    cancellation::Cancellation,
    model::{ModelProvider, ModelRequest, TextSink},
    observability::{self, Options, Phase, RecordingStatus, Settings, Switch, Telemetry, Usage},
    permissions::{PermissionAction, PermissionPolicy},
    runtime::{self, RunConfig, RunOutcome},
    tools::Tools,
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::json;
use std::{
    fs, io,
    os::unix::fs::{PermissionsExt, symlink},
    time::Duration,
};
use support::{Confirmation, Server, call, reply};
fn config(options: Option<Options>) -> RunConfig {
    RunConfig {
        observability: options,
        model: "fixture".into(),
        task: "TASK-SENTINEL".into(),
        max_model_calls: 5,
        context_budget: Some(Default::default()),
        permissions: Default::default(),
    }
}
fn tools(root: &std::path::Path) -> Tools {
    Tools::new(Workspace::new(root).unwrap(), Duration::from_secs(1)).unwrap()
}
fn usage_reply(text: &str) -> Vec<serde_json::Value> {
    let mut events = reply(&[], text);
    events.last_mut().unwrap()["response"]["usage"] = json!({"input_tokens":12,"output_tokens":3,"input_tokens_details":{"cached_tokens":0},"total_tokens":15});
    events
}
#[test]
fn settings_are_private_atomic_and_resolve_without_enabling_on_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("config/settings.json");
    let settings = Settings::load(&path).unwrap();
    assert_eq!(
        settings.resolve(None, false),
        (Switch::Off, "built-in default")
    );
    Settings {
        observability: Switch::On,
        ..Settings::default()
    }
    .save(&path)
    .unwrap();
    let settings = Settings::load(&path).unwrap();
    assert_eq!(settings.resolve(None, true), (Switch::On, "user setting"));
    assert_eq!(
        settings.resolve(Some(Switch::Off), true),
        (Switch::Off, "run flag")
    );
    Settings {
        observability: Switch::Off,
        ..Settings::default()
    }
    .save(&path)
    .unwrap();
    assert_eq!(settings.observability, Switch::On); // resolved snapshot remains immutable
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    for invalid in [
        "{",
        r#"{"observability":"maybe"}"#,
        r#"{"observability":"on","extra":1}"#,
        "{}",
    ] {
        fs::write(&path, invalid).unwrap();
        assert!(Settings::load(&path).is_err());
    }
    let other = temp.path().join("outside");
    fs::write(&other, "{}").unwrap();
    fs::remove_file(&path).unwrap();
    symlink(&other, &path).unwrap();
    assert!(Settings::load(&path).is_err());
}
#[test]
fn usage_keeps_zero_partial_and_missing_distinct_and_rejects_inconsistency() {
    assert_eq!(Usage::from_response(&json!({})), Usage::default());
    let zero = Usage::from_response(
        &json!({"usage":{"input_tokens":0,"output_tokens":0,"total_tokens":0,"input_tokens_details":{"cached_tokens":0}}}),
    );
    assert_eq!(zero.input_tokens, Some(0));
    assert_eq!(zero.cached_tokens, Some(0));
    assert!(!zero.invalid);
    let partial = Usage::from_response(&json!({"usage":{"input_tokens":5}}));
    assert_eq!(partial.input_tokens, Some(5));
    assert_eq!(partial.output_tokens, None);
    for usage in [
        json!({"input_tokens":-1}),
        json!({"input_tokens":1.5}),
        json!({"input_tokens":1,"input_tokens_details":{"cached_tokens":2}}),
        json!({"input_tokens":1,"output_tokens":2,"total_tokens":4}),
        json!("bad"),
        json!({"input_tokens_details":4}),
    ] {
        let u = Usage::from_response(&json!({"usage":usage}));
        assert!(u.invalid);
        assert_eq!(u.input_tokens, None);
    }
}
#[tokio::test]
async fn headless_multicall_trace_has_usage_timing_context_and_no_content() {
    let root = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    let store = storage.path().join("traces");
    fs::write(root.path().join("AGENTS.md"), "INSTRUCTION-SENTINEL").unwrap();
    let write = call(
        "a",
        "write_file",
        json!({"path":"PATH-SENTINEL","content":"RESULT-SENTINEL","overwrite":false}),
    );
    let mut first = reply(&[write], "");
    first.last_mut().unwrap()["response"]["output"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"reasoning","encrypted_content":"PRIVATE-SENTINEL"}));
    let server = Server::start(vec![first, usage_reply("TEXT-SENTINEL")]).await;
    let t = tools(root.path());
    let mut permission = Confirmation::new(true);
    let (sender, receiver) = tokio::sync::mpsc::channel(2);
    drop(receiver); // disconnected client
    let result = runtime::run(
        &server.provider,
        &t,
        &mut permission,
        config(Some(Options::new(store.clone()))),
        Cancellation::default(),
        Some(sender),
    )
    .await
    .unwrap();
    assert_eq!(result.outcome, RunOutcome::Completed);
    assert_eq!(
        result.recording.status,
        RecordingStatus::Complete,
        "{:?}",
        result.recording.diagnostic
    );
    assert_eq!(
        fs::read_to_string(root.path().join("PATH-SENTINEL")).unwrap(),
        "RESULT-SENTINEL"
    );
    server.finish().await;
    let trace = observability::read_trace(&store, &result.recording.run_id.to_string()).unwrap();
    assert!(trace.complete);
    let summary = trace.summary();
    assert_eq!(summary.models.len(), 2);
    assert_eq!(summary.tools.len(), 1);
    assert!(!summary.contexts.is_empty());
    assert!(summary.runtime_timings_us.contains_key("context_selection"));
    let first = summary
        .models
        .values()
        .find(|c| c.usage.as_ref().is_some_and(|u| u.input_tokens.is_none()))
        .unwrap();
    assert!(!first.timings_us.contains_key("first_visible_text"));
    let final_call = summary
        .models
        .values()
        .find(|c| c.usage.as_ref().is_some_and(|u| u.input_tokens == Some(12)))
        .unwrap();
    assert!(final_call.timings_us.contains_key("first_visible_text"));
    assert!(final_call.timings_us.contains_key("provider_attempt"));
    assert!(
        summary
            .tools
            .values()
            .next()
            .unwrap()
            .timings_us
            .contains_key("tool_execution")
    );
    assert_eq!(summary.cost, None);
    assert_eq!(summary.true_ttft_us, None);
    let mut stats = observability::Stats::default();
    stats.add(&summary);
    assert_eq!(stats.observed_model_calls, 2);
    assert_eq!(stats.input_reporting_calls, 1);
    assert_eq!(stats.known_input_tokens, 12);
    assert_eq!(stats.known_cached_tokens, 0);
    assert_eq!(stats.unrecorded_runs, None);
    let path = store.join(format!("{}.jsonl", result.recording.run_id));
    let stored = fs::read_to_string(&path).unwrap();
    for secret in [
        "TASK-SENTINEL",
        "INSTRUCTION-SENTINEL",
        "PATH-SENTINEL",
        "RESULT-SENTINEL",
        "PRIVATE-SENTINEL",
        "TEXT-SENTINEL",
        "test-only-credential",
    ] {
        assert!(!stored.contains(secret), "leaked {secret}");
    }
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
#[tokio::test]
async fn off_never_writes_traces_and_keeps_permission_and_context_contracts() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::start(vec![
        reply(
            &[call(
                "a",
                "write_file",
                json!({"path":"denied","content":"x"}),
            )],
            "",
        ),
        reply(&[], "done"),
    ])
    .await;
    let t = tools(root.path());
    let mut permission = Confirmation::new(true);
    let mut conf = config(None);
    conf.permissions = PermissionPolicy {
        write: PermissionAction::Deny,
        ..Default::default()
    };
    let result = runtime::run(
        &server.provider,
        &t,
        &mut permission,
        conf,
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    server.finish().await;
    assert_eq!(result.recording.status, RecordingStatus::Off);
    assert_eq!(result.outcome, RunOutcome::Completed);
    assert!(!root.path().join("denied").exists());
    assert!(result.session.selection.is_some());
}
#[tokio::test]
async fn limits_or_unavailable_store_do_not_relabel_committed_edits() {
    for unavailable in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let storage = tempfile::tempdir().unwrap();
        let store = storage.path().join("traces");
        if unavailable {
            fs::write(&store, "not a directory").unwrap();
        }
        let server = Server::start(vec![
            reply(
                &[call(
                    "a",
                    "write_file",
                    json!({"path":"edited","content":"committed","overwrite":false}),
                )],
                "",
            ),
            reply(&[], "done"),
        ])
        .await;
        let mut options = Options::new(store.clone());
        if !unavailable {
            options.max_run_bytes = 250;
        }
        let result = runtime::run(
            &server.provider,
            &tools(root.path()),
            &mut Confirmation::new(true),
            config(Some(options)),
            Cancellation::default(),
            None,
        )
        .await
        .unwrap();
        server.finish().await;
        assert_eq!(result.outcome, RunOutcome::Completed);
        assert_eq!(
            fs::read_to_string(root.path().join("edited")).unwrap(),
            "committed"
        );
        assert!(matches!(
            result.recording.status,
            RecordingStatus::Incomplete | RecordingStatus::Unavailable
        ));
        if let Ok(trace) = observability::read_trace(&store, &result.recording.run_id.to_string()) {
            assert!(!trace.complete);
        }
    }
}
#[tokio::test]
async fn cancelled_run_and_failed_provider_keep_honest_terminal_trace() {
    let root = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    let store = storage.path().join("traces");
    let server = Server::http(429, vec![vec![]]).await;
    let result = runtime::run(
        &server.provider,
        &tools(root.path()),
        &mut Confirmation::new(true),
        config(Some(Options::new(store.clone()))),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    server.finish().await;
    assert!(matches!(result.outcome, RunOutcome::Failed { .. }));
    let s = observability::read_trace(&store, &result.recording.run_id.to_string())
        .unwrap()
        .summary();
    assert!(s.complete);
    assert!(s.models.values().all(|c| c.usage.is_none()));
    let cancel = Cancellation::default();
    cancel.cancel();
    let server = Server::start(vec![]).await;
    let result = runtime::run(
        &server.provider,
        &tools(root.path()),
        &mut Confirmation::new(true),
        config(Some(Options::new(store.clone()))),
        cancel,
        None,
    )
    .await
    .unwrap();
    server.finish().await;
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    let s = observability::read_trace(&store, &result.recording.run_id.to_string())
        .unwrap()
        .summary();
    assert!(s.complete);
    assert!(s.models.is_empty());
    assert_eq!(s.outcome.as_deref(), Some("run_cancelled"));
}
#[derive(Default)]
struct Sink {
    enabled: bool,
    telemetry: Vec<Telemetry>,
}
#[async_trait]
impl TextSink for Sink {
    fn telemetry_enabled(&self) -> bool {
        self.enabled
    }
    async fn telemetry(&mut self, v: Telemetry) {
        self.telemetry.push(v);
    }
    async fn delta(&mut self, _: &str) -> io::Result<()> {
        Ok(())
    }
}
#[tokio::test]
async fn optional_usage_does_not_weaken_completion_barrier_and_off_has_no_metrics() {
    for enabled in [false, true] {
        let mut events = usage_reply("done");
        events.last_mut().unwrap()["response"]["usage"]["input_tokens"] = json!("malformed");
        let server = Server::start(vec![events]).await;
        let mut sink = Sink {
            enabled,
            ..Default::default()
        };
        server
            .provider
            .generate(
                &ModelRequest {
                    model: "fixture",
                    instructions: "",
                    messages: &[],
                },
                &mut sink,
            )
            .await
            .unwrap();
        server.finish().await;
        if enabled {
            assert!(
                sink.telemetry
                    .iter()
                    .any(|t| t.usage.as_ref().is_some_and(|u| u.invalid))
            );
        } else {
            assert!(sink.telemetry.is_empty());
        }
    }
    let mut failed = usage_reply("provisional");
    failed.last_mut().unwrap()["response"]["status"] = json!("failed");
    let server = Server::start(vec![failed]).await;
    let mut sink = Sink {
        enabled: true,
        ..Default::default()
    };
    assert!(
        server
            .provider
            .generate(
                &ModelRequest {
                    model: "fixture",
                    instructions: "",
                    messages: &[]
                },
                &mut sink
            )
            .await
            .is_err()
    );
    server.finish().await;
    assert!(sink.telemetry.iter().all(|t| t.usage.is_none()));
    assert!(
        sink.telemetry
            .iter()
            .any(|t| t.phase == Some(Phase::ProviderAttempt))
    );
}

struct CancellingProvider {
    cancel: Cancellation,
}
#[async_trait]
impl ModelProvider for CancellingProvider {
    async fn generate(
        &self,
        _: &ModelRequest<'_>,
        sink: &mut dyn TextSink,
    ) -> Result<astrid::model::ModelResponse, astrid::model::ModelError> {
        sink.delta("PRIVATE-PROVISIONAL").await.unwrap();
        self.cancel.cancel();
        std::future::pending().await
    }
}
#[tokio::test]
async fn cancellation_during_stream_preserves_attempt_without_final_usage() {
    let root = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    let store = storage.path().join("traces");
    let cancel = Cancellation::default();
    let provider = CancellingProvider {
        cancel: cancel.clone(),
    };
    let mut conf = config(Some(Options::new(store.clone())));
    conf.context_budget = None;
    let result = runtime::run(
        &provider,
        &tools(root.path()),
        &mut Confirmation::new(true),
        conf,
        cancel,
        None,
    )
    .await
    .unwrap();
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert_eq!(result.recording.status, RecordingStatus::Complete);
    let summary = observability::read_trace(&store, &result.recording.run_id.to_string())
        .unwrap()
        .summary();
    assert_eq!(summary.provider, None);
    assert_eq!(summary.models.len(), 1);
    let call = summary.models.values().next().unwrap();
    assert_eq!(call.usage, None);
    assert_eq!(call.outcome.as_deref(), Some("model_call_cancelled"));
    assert!(call.inclusive_runtime_us.is_some());
}
#[tokio::test]
async fn consumer_delay_is_measured_as_delivery_wait() {
    let root = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    let store = storage.path().join("traces");
    let server = Server::start(vec![usage_reply("done")]).await;
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let mut confirmation = Confirmation::new(true);
    let t = tools(root.path());
    let execution = runtime::run(
        &server.provider,
        &t,
        &mut confirmation,
        config(Some(Options::new(store.clone()))),
        Cancellation::default(),
        Some(sender),
    );
    let consume = async {
        let mut delayed = false;
        while let Some(event) = receiver.recv().await {
            if !delayed && matches!(event.kind, astrid::events::EventKind::TurnStarted { .. }) {
                delayed = true;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
    };
    let (result, ()) = tokio::join!(execution, consume);
    let result = result.unwrap();
    server.finish().await;
    assert_eq!(result.recording.status, RecordingStatus::Complete);
    let summary = observability::read_trace(&store, &result.recording.run_id.to_string())
        .unwrap()
        .summary();
    assert!(summary.runtime_timings_us["delivery_wait"] >= 20_000);
}
#[test]
fn settings_and_inspection_cli_work_without_authentication() {
    let temp = tempfile::tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_astrid");
    let run = |args: &[&str]| {
        std::process::Command::new(bin)
            .args(args)
            .env("HOME", temp.path())
            .env_remove("ASTRID_MODEL")
            .output()
            .unwrap()
    };
    let initial = run(&["settings"]);
    assert!(initial.status.success());
    assert!(String::from_utf8_lossy(&initial.stdout).contains("off (built-in default)"));
    assert!(
        run(&["settings", "set", "observability", "on"])
            .status
            .success()
    );
    assert!(String::from_utf8_lossy(&run(&["settings"]).stdout).contains("on (user setting)"));
    assert!(
        run(&["settings", "set", "expanded-tool-calls", "on"])
            .status
            .success()
    );
    let output = run(&["settings"]);
    let output = String::from_utf8_lossy(&output.stdout);
    assert!(output.contains("expanded-tool-calls: on"));
    assert!(output.contains("observability: on"));
    assert!(
        run(&["settings", "set", "expanded-tool-calls", "off"])
            .status
            .success()
    );
    assert!(
        String::from_utf8_lossy(&run(&["settings"]).stdout).contains("expanded-tool-calls: off")
    );
    assert!(run(&["stats"]).status.success());
    assert!(run(&["trace"]).status.success());
    assert!(
        !run(&["settings", "set", "observability", "maybe"])
            .status
            .success()
    );
    assert!(!run(&["trace", "../auth"]).status.success());
    fs::write(
        temp.path().join(".config/astrid/settings.json"),
        "malformed",
    )
    .unwrap();
    let invalid = run(&[
        "run",
        "task",
        "--model",
        "fixture",
        "--observability",
        "off",
    ]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("invalid observability settings"));
}

// An explicit local recorder-overhead experiment, never a live-model benchmark.
#[tokio::test]
#[ignore = "run explicitly to record environment-specific overhead"]
async fn recording_overhead_experiment() {
    struct Final;
    #[async_trait]
    impl ModelProvider for Final {
        async fn generate(
            &self,
            _: &ModelRequest<'_>,
            _: &mut dyn TextSink,
        ) -> Result<astrid::model::ModelResponse, astrid::model::ModelError> {
            astrid::openai::completed_response(
                json!({"status":"completed","output":[support::message("done")]}),
            )
        }
    }
    let root = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    let t = tools(root.path());
    let mut off = Vec::new();
    let mut on = Vec::new();
    for _ in 0..21 {
        for enabled in [false, true] {
            let mut c = config(enabled.then(|| Options::new(storage.path().join("traces"))));
            c.context_budget = None;
            let started = std::time::Instant::now();
            let r = runtime::run(
                &Final,
                &t,
                &mut Confirmation::new(true),
                c,
                Cancellation::default(),
                None,
            )
            .await
            .unwrap();
            assert_eq!(r.outcome, RunOutcome::Completed);
            if enabled {
                assert_eq!(r.recording.status, RecordingStatus::Complete);
                on.push(started.elapsed().as_micros());
            } else {
                off.push(started.elapsed().as_micros());
            }
        }
    }
    off.sort();
    on.sort();
    println!(
        "alternating 21 samples each; non-Git temporary workspace, no network, final-response mock; off median={}us; on median={}us; on min={}us max={}us",
        off[10], on[10], on[0], on[20]
    );
}
