//! Thin noninteractive CLI adapter. Rendering starts after all owned execution
//! joins, so blocked stdout cannot hold active workers or cancellation open.
use astrid::{
    model::ModelProvider,
    scheduler::{self, AdmissionError, BatchControl, BatchOutcome, BatchResult, BatchSpec},
    tools::Tools,
};
use serde_json::{Value, json};
use std::{
    future::Future,
    io::{self, Write},
};

pub(crate) async fn drive(
    provider: &dyn ModelProvider,
    tools: &Tools,
    spec: BatchSpec,
    interrupt: impl Future<Output = ()>,
) -> Result<BatchResult, AdmissionError> {
    let control = BatchControl::new(&spec);
    let execution = scheduler::run(provider, tools, spec, control.clone(), None);
    tokio::pin!(execution, interrupt);
    tokio::select! {
        result = &mut execution => result,
        () = &mut interrupt => { control.cancellation.cancel(); execution.await }
    }
}
pub(crate) fn report(result: &BatchResult) -> Value {
    let count_tasks = result
        .tasks
        .iter()
        .filter(|t| t.model_calls.is_some())
        .count();
    let known_calls: Option<usize> = result
        .tasks
        .iter()
        .filter_map(|t| t.model_calls)
        .try_fold(0usize, |sum, n| sum.checked_add(n));
    json!({
        "schema":1, "batch":result.id, "outcome":result.outcome, "interpretation":"unverified",
        "source_order":"manifest order; dependency inputs retain declared edge order",
        "tasks":result.tasks.iter().map(|t|json!({"key":t.key,"id":t.id,"status":t.status,
            "inputs":t.inputs,"result":t.result,"child_outcome":t.child_outcome,
            "model_calls":t.model_calls,"tool_calls":t.tool_calls,
            "recording":t.recording.as_ref().map(|r|json!({"status":format!("{:?}",r.status).to_lowercase(),"diagnostic":r.diagnostic}))})).collect::<Vec<_>>(),
        "lifecycle":result.lifecycle, "delivery":format!("{:?}",result.delivery).to_lowercase(),
        "recording":format!("{:?}",result.recording.status).to_lowercase(),
        "recording_diagnostic":result.recording.diagnostic,
        "measurements":{"wall_micros":astrid::observability::micros(result.elapsed),
            "cancellation_cleanup_micros":astrid::observability::micros(result.cleanup_elapsed),
            "recording_finalization_micros":result.recording_elapsed.map(astrid::observability::micros),
            "model_calls_known_sum":known_calls,"model_call_reporting_tasks":count_tasks,
            "task_count":result.tasks.len(),"usage":null,"cost":null,"model_window_occupancy":null}
    })
}
pub(crate) fn render(
    result: &BatchResult,
    json_output: bool,
    output: &mut dyn Write,
) -> io::Result<()> {
    if json_output {
        serde_json::to_writer_pretty(&mut *output, &report(result))?;
        writeln!(output)?;
    } else {
        writeln!(
            output,
            "Batch {}: {:?} · results are unverified",
            result.id, result.outcome
        )?;
        for task in &result.tasks {
            writeln!(
                output,
                "\n[{}] {:?} · task={} · run={}",
                crate::console::printable(&task.key).replace(['\n', '\t'], " "),
                task.status.state,
                task.id,
                task.status
                    .run
                    .map_or_else(|| "not dispatched".into(), |id| id.to_string())
            )?;
            if let Some(recording) = &task.recording
                && let Some(diagnostic) = &recording.diagnostic
            {
                writeln!(
                    output,
                    "Child metadata {:?}: {}",
                    recording.status,
                    crate::console::printable(diagnostic)
                )?;
            }
            if let Some(result) = &task.result {
                writeln!(output, "{}", crate::console::printable(&result.text))?;
            } else {
                writeln!(
                    output,
                    "No accepted result; reason={:?}; dependency causes={:?}",
                    task.status.reason, task.status.causes
                )?;
            }
        }
        writeln!(
            output,
            "\nOriginal results retained in manifest order; dependency/synthesis outputs do not replace them."
        )?;
        writeln!(
            output,
            "Batch metadata: {:?}; child coverage is separate. Usage/cost/model-window occupancy unavailable in this report.",
            result.recording.status
        )?;
    }
    Ok(())
}
pub(crate) fn check_outcome(outcome: BatchOutcome) -> Result<(), &'static str> {
    match outcome {
        BatchOutcome::Completed => Ok(()),
        BatchOutcome::Cancelled => Err("batch cancelled; original source results retained above"),
        BatchOutcome::Failed => Err("batch failed; original source results retained above"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astrid::{
        batch_manifest::Manifest,
        model::{Message, ModelError, ModelRequest, ModelResponse, TextSink, ToolOutcome},
        permissions::PermissionPolicy,
        tools::Tools,
        workspace::Workspace,
    };
    use async_trait::async_trait;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tokio::sync::{Barrier, Notify};
    struct Script {
        barrier: Barrier,
        entered: Notify,
        active: Arc<AtomicUsize>,
        mode: &'static str,
        calls: AtomicUsize,
    }
    struct Guard(Arc<AtomicUsize>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    fn response(text: &str, calls: Vec<astrid::model::ToolCall>) -> ModelResponse {
        let mut output: Vec<_> = calls.iter().map(crate::test_support::item).collect();
        output.push(crate::test_support::message(text));
        output.push(json!({"type":"reasoning","id":"private","encrypted_content":"PRIVATE-CLI-SENTINEL","summary":[]}));
        astrid::openai::completed_response(json!({"status":"completed","output":output})).unwrap()
    }
    #[async_trait]
    impl ModelProvider for Script {
        fn measure_request(
            &self,
            request: &ModelRequest<'_>,
        ) -> Result<Option<astrid::context::ContextSnapshot>, ModelError> {
            astrid::openai::OpenAiProvider::new(Arc::new(crate::test_support::TestAuth))
                .unwrap()
                .measure_request(request)
        }
        async fn generate(
            &self,
            request: &ModelRequest<'_>,
            sink: &mut dyn TextSink,
        ) -> Result<ModelResponse, ModelError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.active.fetch_add(1, Ordering::SeqCst);
            let _guard = Guard(self.active.clone());
            let Message::User(input) = &request.messages[0] else {
                panic!("fresh task")
            };
            let key = input.lines().next().unwrap();
            if key == "review" {
                assert!(input.contains("src/lib.rs"));
                assert!(input.contains("greeting_trims_whitespace"));
                assert!(!input.contains("PRIVATE-CLI-SENTINEL"));
                return Ok(response(
                    "SCRIPTED unverified review: src/lib.rs trims names; declared tests include greeting_trims_whitespace and required_greeting_file_matches. Tests were not executed.",
                    vec![],
                ));
            }
            if key == "checklist" {
                assert!(input.contains("house.txt"));
                assert!(input.contains("garden.md"));
                assert!(!input.contains("PRIVATE-CLI-SENTINEL"));
                assert!(!input.contains("PROVISIONAL-CLI"));
                return Ok(response(
                    "Unverified checklist: house.txt bulbs; garden.md basil Saturday and boiler date unknown.",
                    vec![],
                ));
            }
            if request.messages.len() == 1 {
                self.barrier.wait().await;
                self.entered.notify_one();
                if self.mode == "wait" {
                    std::future::pending::<()>().await;
                }
                if self.mode == "fail" && key == "house" {
                    return Err(ModelError::Protocol("synthetic house failure".into()));
                }
                sink.delta("PROVISIONAL-CLI").await?;
                return Ok(response(
                    "inspect",
                    vec![crate::test_support::call(
                        "read",
                        "read_file",
                        match key {
                            "source" => json!({"path":"src/lib.rs","start_line":1,"end_line":4}),
                            "tests" => json!({"path":"src/lib.rs","start_line":6,"end_line":25}),
                            "house" => json!({"path":"house.txt"}),
                            _ => json!({"path":"garden.md"}),
                        },
                    )],
                ));
            }
            let content = request
                .messages
                .iter()
                .find_map(|m| match m {
                    Message::Tool(t) => match &t.outcome {
                        ToolOutcome::Success { data } => data["content"].as_str(),
                        _ => None,
                    },
                    _ => None,
                })
                .unwrap();
            Ok(response(
                &format!(
                    "{}: {content}",
                    match key {
                        "house" => "house.txt",
                        "garden" => "garden.md",
                        _ => "src/lib.rs",
                    }
                ),
                vec![],
            ))
        }
    }
    fn setup(mode: &'static str) -> (tempfile::TempDir, Tools, BatchSpec, Script) {
        let root = tempfile::tempdir().unwrap();
        for file in ["house.txt", "garden.md"] {
            std::fs::copy(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/personal-tasks/outstanding")
                    .join(file),
                root.path().join(file),
            )
            .unwrap();
        }
        let manifest=Manifest::parse(br#"{"schema":1,"max_model_calls":12,"tasks":[
            {"key":"house","instruction":"house","model":"fixture","max_model_calls":4},
            {"key":"garden","instruction":"garden","model":"fixture","max_model_calls":4},
            {"key":"checklist","instruction":"checklist","model":"fixture","max_model_calls":4,"depends_on":["house","garden"]}
        ]}"#).unwrap();
        let spec = manifest.into_spec(2, PermissionPolicy::read_only(), None);
        let tools =
            Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1)).unwrap();
        let script = Script {
            barrier: Barrier::new(2),
            entered: Notify::new(),
            active: Arc::new(AtomicUsize::new(0)),
            mode,
            calls: AtomicUsize::new(0),
        };
        (root, tools, spec, script)
    }
    #[tokio::test]
    async fn cli_driver_overlaps_real_notes_reads_and_renders_original_bundle_after_join() {
        let (root, tools, spec, script) = setup("normal");
        let before = std::fs::read(root.path().join("house.txt")).unwrap();
        let result = tokio::time::timeout(
            Duration::from_secs(8),
            drive(&script, &tools, spec, std::future::pending()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(result.outcome, BatchOutcome::Completed);
        assert_eq!(script.calls.load(Ordering::SeqCst), 5);
        assert_eq!(script.active.load(Ordering::SeqCst), 0);
        assert_eq!(result.tasks[0].tool_calls, Some(1));
        assert_eq!(result.tasks[1].tool_calls, Some(1));
        assert_eq!(result.tasks[2].inputs.len(), 2);
        assert!(
            result.tasks[0]
                .result
                .as_ref()
                .unwrap()
                .text
                .contains("Completed: Replace the kitchen tap washer")
        );
        let mut output = vec![];
        render(&result, true, &mut output).unwrap();
        let report: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(report["outcome"], "completed");
        assert!(report["measurements"]["cost"].is_null());
        assert_eq!(
            std::fs::read(root.path().join("house.txt")).unwrap(),
            before
        );
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        assert!(render(&result, false, &mut Broken).is_err());
        assert_eq!(script.active.load(Ordering::SeqCst), 0);
    }
    #[tokio::test]
    async fn cli_driver_retains_failure_and_source_results_when_dependency_skips() {
        let (_root, tools, spec, script) = setup("fail");
        let result = tokio::time::timeout(
            Duration::from_secs(8),
            drive(&script, &tools, spec, std::future::pending()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(result.outcome, BatchOutcome::Failed);
        assert!(check_outcome(result.outcome).is_err());
        assert_eq!(result.tasks[2].status.state, scheduler::TaskState::Skipped);
        assert!(
            result.tasks[1]
                .result
                .as_ref()
                .unwrap()
                .text
                .contains("garden.md")
        );
        let mut output = vec![];
        render(&result, true, &mut output).unwrap();
        let report: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(report["tasks"][2]["status"]["state"], "skipped");
    }
    #[tokio::test]
    async fn cli_interrupt_joins_active_workers_before_reporting() {
        let (_root, tools, spec, script) = setup("wait");
        let result = tokio::time::timeout(
            Duration::from_secs(8),
            drive(&script, &tools, spec, script.entered.notified()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(result.outcome, BatchOutcome::Cancelled);
        assert_eq!(script.active.load(Ordering::SeqCst), 0);
        assert!(check_outcome(result.outcome).is_err());
        assert!(result.tasks.iter().all(|t| t.result.is_none()));
    }
    #[tokio::test]
    async fn cli_driver_reconciles_read_only_code_and_declared_test_inspections() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/greeting");
        for file in ["src/lib.rs", "GREETING.txt", "AGENTS.md", "Cargo.toml"] {
            std::fs::copy(fixture.join(file), root.path().join(file)).unwrap();
        }
        let before = std::fs::read(root.path().join("src/lib.rs")).unwrap();
        let manifest=Manifest::parse(br#"{"schema":1,"tasks":[
            {"key":"source","instruction":"source","model":"fixture","max_model_calls":2},
            {"key":"tests","instruction":"tests","model":"fixture","max_model_calls":2},
            {"key":"review","instruction":"review","model":"fixture","max_model_calls":2,"depends_on":["source","tests"]}]}"#).unwrap();
        let spec = manifest.into_spec(2, PermissionPolicy::read_only(), None);
        let tools =
            Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(1)).unwrap();
        let script = Script {
            barrier: Barrier::new(2),
            entered: Notify::new(),
            active: Arc::new(AtomicUsize::new(0)),
            mode: "normal",
            calls: AtomicUsize::new(0),
        };
        let result = tokio::time::timeout(
            Duration::from_secs(8),
            drive(&script, &tools, spec, std::future::pending()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(result.outcome, BatchOutcome::Completed);
        assert_eq!(script.calls.load(Ordering::SeqCst), 5);
        assert_eq!(result.tasks[0].tool_calls, Some(1));
        assert_eq!(result.tasks[1].tool_calls, Some(1));
        assert!(
            result.tasks[0]
                .result
                .as_ref()
                .unwrap()
                .text
                .contains("name.trim()")
        );
        assert!(
            result.tasks[1]
                .result
                .as_ref()
                .unwrap()
                .text
                .contains("greeting_trims_whitespace")
        );
        assert!(
            result.tasks[2]
                .result
                .as_ref()
                .unwrap()
                .text
                .contains("not executed")
        );
        assert_eq!(result.tasks[2].inputs.len(), 2);
        assert_eq!(
            std::fs::read(root.path().join("src/lib.rs")).unwrap(),
            before
        );
    }
}
