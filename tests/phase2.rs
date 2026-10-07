mod support;
use astrid::{
    cancellation::Cancellation,
    events::{EventKind, ExecutionState},
    model::{
        Message, ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink, ToolOutcome,
    },
    openai::completed_response,
    permissions::PermissionPolicy,
    runtime::{self, PermissionHandler, RunConfig, RunOutcome},
    tools::{PermissionRequest, ToolExecution, ToolExecutor, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::json;
use std::{collections::VecDeque, fs, io, sync::Mutex, time::Duration};
use support::{call, item};

struct Script(Mutex<VecDeque<ModelResponse>>);
#[async_trait]
impl ModelProvider for Script {
    async fn generate(
        &self,
        _: &ModelRequest<'_>,
        _: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected model invocation"))
    }
}
fn response(calls: Vec<astrid::model::ToolCall>) -> ModelResponse {
    completed_response(
        json!({"status":"completed","output":calls.iter().map(item).collect::<Vec<_>>() }),
    )
    .unwrap()
}
fn data(result: ToolExecution) -> serde_json::Value {
    match result {
        ToolExecution::Finished(result) => match result.outcome {
            ToolOutcome::Success { data } | ToolOutcome::TimedOut { data } => data,
            other => panic!("unexpected tool outcome {other:?}"),
        },
        other => panic!("unexpected execution {other:?}"),
    }
}

#[tokio::test]
async fn finite_dual_pipe_flood_caps_capture_and_counts_observed_bytes() {
    let root = tempfile::tempdir().unwrap();
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(5)).unwrap();
    let result = data(tools.execute(&call("flood", "shell", json!({"command":"dd if=/dev/zero bs=1000 count=200 2>/dev/null; dd if=/dev/zero bs=1000 count=200 >&2 2>/dev/null"})), &Cancellation::default()).await);
    for stream in ["stdout", "stderr"] {
        assert_eq!(result["output"][stream]["observed_bytes"], 200_000);
        assert_eq!(result["output"][stream]["captured_bytes"], 65_536);
        assert_eq!(result["output"][stream]["capture_omitted_bytes"], 134_464);
        assert_eq!(result["output"][stream]["complete"], true);
        assert_eq!(result["output"][stream]["truncated"], true);
        assert_eq!(result[stream].as_str().unwrap().len(), 65_536);
    }
    assert_eq!(result["exit_code"], 0);
}

#[tokio::test]
async fn timeout_retains_partial_output_without_inventing_completion_or_exit_code() {
    let root = tempfile::tempdir().unwrap();
    let tools = Tools::new(
        Workspace::new(root.path()).unwrap(),
        Duration::from_millis(150),
    )
    .unwrap();
    let result = data(
        tools
            .execute(
                &call(
                    "timeout",
                    "shell",
                    json!({"command":"printf before; printf error >&2; sleep 30"}),
                ),
                &Cancellation::default(),
            )
            .await,
    );
    assert_eq!(result["stdout"], "before");
    assert_eq!(result["stderr"], "error");
    assert_eq!(result["timed_out"], true);
    assert!(result["exit_code"].is_null());
    assert_eq!(result["output"]["stdout"]["complete"], false);
    assert_eq!(result["output"]["stdout"]["observed_bytes"], 6);
}

#[tokio::test]
async fn incomplete_utf8_suffix_is_explicitly_unavailable_in_retained_text() {
    let root = tempfile::tempdir().unwrap();
    let tools = Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(2)).unwrap();
    let result = data(
        tools
            .execute(
                &call("utf8", "shell", json!({"command":"printf '\\342'"})),
                &Cancellation::default(),
            )
            .await,
    );
    assert_eq!(result["stdout"], "");
    assert_eq!(result["output"]["stdout"]["captured_bytes"], 1);
    assert_eq!(
        result["output"]["stdout"]["text_unavailable_suffix_bytes"],
        1
    );
}

struct SelectiveApproval;
#[async_trait]
impl PermissionHandler for SelectiveApproval {
    async fn decide(&mut self, request: &PermissionRequest) -> io::Result<bool> {
        Ok(!request.command.contains("FORBIDDEN"))
    }
}

#[tokio::test]
async fn repository_acceptance_preserves_user_edits_streams_tests_denies_and_records_patches() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    for path in ["Cargo.toml", "AGENTS.md", "src/lib.rs"] {
        fs::copy(
            format!("tests/fixtures/greeting/{path}"),
            root.path().join(path),
        )
        .unwrap();
    }
    fs::write(root.path().join("USER.txt"), "pre-existing user work\n").unwrap();
    let old = fs::read_to_string(root.path().join("src/lib.rs")).unwrap();
    let provider = Script(Mutex::new(vec![
        response(vec![call("inspect", "read_file", json!({"path":"src/lib.rs"})), call("test-before", "shell", json!({"command":"cargo test --offline"}))]),
        response(vec![call("denied", "shell", json!({"command":"printf forbidden > FORBIDDEN"}))]),
        response(vec![call("fix", "edit_file", json!({"path":"src/lib.rs","old_text":"format!(\"Hello, {}!\", name)","new_text":"format!(\"Hello, {}!\", name.trim())"})), call("create", "write_file", json!({"path":"GREETING.txt","content":"Hello, Astrid!\n","overwrite":false}))]),
        response(vec![call("test-after", "shell", json!({"command":"cargo test --offline"}))]),
        response(vec![]),
    ].into()));
    assert!(old.contains("format!(\"Hello, {}!\", name)"));
    let tools = Tools::new(
        Workspace::new(root.path()).unwrap(),
        Duration::from_secs(15),
    )
    .unwrap();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let consumer = async {
        let mut events = vec![];
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        events
    };
    let mut approval = SelectiveApproval;
    let (result, events) = tokio::join!(
        runtime::run(
            &provider,
            &tools,
            &mut approval,
            RunConfig {
                model: "test".into(),
                task: "repair greeting".into(),
                max_model_calls: 10,
                permissions: PermissionPolicy::default()
            },
            Cancellation::default(),
            Some(sender)
        ),
        consumer
    );
    let result = result.unwrap();
    assert_eq!(result.outcome, RunOutcome::Completed);
    assert_eq!(
        fs::read_to_string(root.path().join("USER.txt")).unwrap(),
        "pre-existing user work\n"
    );
    assert!(!root.path().join("FORBIDDEN").exists());
    assert_eq!(
        fs::read_to_string(root.path().join("GREETING.txt")).unwrap(),
        "Hello, Astrid!\n"
    );
    let shell_results: Vec<_> = result
        .session
        .messages
        .iter()
        .filter_map(|message| {
            if let Message::Tool(tool) = message
                && tool.name == "shell"
            {
                Some(&tool.outcome)
            } else {
                None
            }
        })
        .collect();
    assert!(matches!(shell_results[0], ToolOutcome::Success { data } if data["exit_code"] == 101));
    assert!(
        matches!(shell_results[1], ToolOutcome::Error { code, .. } if code == "permission_denied")
    );
    assert!(matches!(shell_results[2], ToolOutcome::Success { data } if data["exit_code"] == 0));
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, EventKind::ToolOutput { .. }))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, EventKind::ToolCallDenied { .. }))
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event.kind, EventKind::NativeMutationRecorded { .. }))
            .count(),
        2
    );
    assert!(result.changes.changes.iter().any(|change| {
        change.path == "utf8:src/lib.rs"
            && change
                .patch
                .as_ref()
                .is_some_and(|patch| patch.contains("name.trim()"))
    }));
    let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
    for event in &events {
        replay.transition(event).unwrap();
    }
    assert_eq!(replay, result.state);
}
