//! Deterministic desktop-host evidence. Scripted inference is not live task quality.
mod support;
use astrid::{
    events::{RunId, ToolCallId},
    host::{
        Host, Submit,
        view::{PAGE_BYTES, Workflow},
    },
    model::{Message, ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink},
    openai::{self, OpenAiProvider},
    session_store::SessionStore,
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use uuid::Uuid;
struct Script {
    accounting: OpenAiProvider,
    responses: Mutex<VecDeque<Value>>,
    calls: AtomicUsize,
    requests: Mutex<Vec<Vec<Message>>>,
    hold: bool,
    flood: usize,
}
impl Script {
    fn new(responses: Vec<Value>) -> Arc<Self> {
        Self::options(responses, false, 0)
    }
    fn options(responses: Vec<Value>, hold: bool, flood: usize) -> Arc<Self> {
        Arc::new(Self {
            accounting: OpenAiProvider::new(Arc::new(support::TestAuth)).unwrap(),
            responses: Mutex::new(responses.into()),
            calls: AtomicUsize::new(0),
            requests: Mutex::new(Vec::new()),
            hold,
            flood,
        })
    }
}
#[async_trait]
impl ModelProvider for Script {
    fn measure_request(
        &self,
        r: &ModelRequest<'_>,
    ) -> Result<Option<astrid::context::ContextSnapshot>, ModelError> {
        self.accounting.measure_request(r)
    }
    async fn generate(
        &self,
        r: &ModelRequest<'_>,
        sink: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.requests.lock().unwrap().push(r.messages.to_vec());
        if self.hold {
            std::future::pending::<()>().await;
        }
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| ModelError::Provider("script exhausted".into()))?;
        let response = openai::completed_response(response)?;
        if self.flood > 0 {
            for chunk in response.text.as_bytes().chunks(self.flood) {
                sink.delta(std::str::from_utf8(chunk).unwrap()).await?;
            }
        } else {
            sink.delta(&response.text).await?;
        }
        Ok(response)
    }
}
fn response(calls: Vec<astrid::model::ToolCall>, text: &str) -> Value {
    let mut output = vec![
        support::message(text),
        json!({"type":"reasoning","encrypted_content":"PRIVATE_CONTINUATION_SENTINEL","summary":[]}),
    ];
    output.extend(calls.iter().map(support::item));
    json!({"status":"completed","output":output})
}
fn host(provider: Arc<Script>, root: &std::path::Path, user: &std::path::Path) -> Arc<Host> {
    let host = Host::new(provider, user.into(), "mock-desktop".into()).unwrap();
    host.open_folder("main", root).unwrap();
    host
}
async fn until(host: &Host, predicate: impl Fn(&astrid::host::view::Snapshot) -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if predicate(&host.snapshot("main").unwrap()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
fn task(id: Uuid) -> Submit {
    Submit {
        request_id: id,
        task: "Checklist from note.txt; cite sources and omissions".into(),
    }
}
#[tokio::test]
async fn notes_real_tools_one_host_deduplicates_and_preserves_cli_store_lock() {
    let root = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("note.txt"), "Buy a lamp").unwrap();
    let provider = Script::new(vec![
        response(
            vec![
                support::call("read", "read_file", json!({"path":"note.txt"})),
                support::call(
                    "write",
                    "write_file",
                    json!({"path":"note.txt","content":"wrong","overwrite":true}),
                ),
                support::call("shell", "shell", json!({"command":"touch unwanted"})),
            ],
            "Inspect",
        ),
        response(
            vec![],
            "Buy a lamp (note.txt). Coverage limited to read sources.",
        ),
    ]);
    let h = host(provider.clone(), root.path(), user.path());
    assert!(SessionStore::open(user.path(), &Workspace::new(root.path()).unwrap()).is_err());
    assert!(h.snapshot("mascot").is_err());
    assert!(h.workflow("remote", Workflow::CodingUnbound).is_err());
    let id = Uuid::new_v4();
    assert!(h.submit("remote", task(id)).is_err());
    let first = h.submit("main", task(id)).unwrap();
    assert_eq!(
        h.submit("main", task(id)).unwrap().session_id,
        first.session_id
    );
    assert!(
        h.submit(
            "main",
            Submit {
                request_id: id,
                task: "different side effect".into()
            }
        )
        .is_err()
    );
    until(&h, |s| {
        s.run.as_ref().is_some_and(|r| r.status == "completed")
    })
    .await;
    h.submit("main", task(id)).unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        std::fs::read_to_string(root.path().join("note.txt")).unwrap(),
        "Buy a lamp"
    );
    assert!(!root.path().join("unwanted").exists());
    let requests = provider.requests.lock().unwrap();
    let last = requests.last().unwrap();
    assert_eq!(
        last.iter()
            .filter(|m| matches!(m, Message::Tool(_)))
            .count(),
        3
    );
    let visible = serde_json::to_string(&h.history("main", 0).unwrap()).unwrap();
    assert!(visible.contains("permission_denied"));
    assert!(!visible.contains("PRIVATE_CONTINUATION_SENTINEL"));
    assert!(serde_json::from_value::<Submit>(json!({"request_id":Uuid::new_v4(),"task":"x","policy":{"execute":"allow"},"principal":"root"})).is_err());
}
#[tokio::test]
async fn approval_is_bound_to_run_tool_view_and_single_use() {
    let root = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![
        response(
            vec![support::call(
                "shell",
                "shell",
                json!({"command":"printf approved"}),
            )],
            "Verify",
        ),
        response(vec![], "Command inspected"),
    ]);
    let h = host(provider, root.path(), user.path());
    h.workflow("main", Workflow::CodingAsk).unwrap();
    let id = Uuid::new_v4();
    h.submit("main", task(id)).unwrap();
    until(&h, |s| {
        s.run.as_ref().is_some_and(|r| r.permission.is_some())
    })
    .await;
    let permission = h.snapshot("main").unwrap().run.unwrap().permission.unwrap();
    assert!(permission.can_approve);
    assert!(permission.command.value.contains("printf approved"));
    assert!(
        h.permission(
            "remote",
            permission.id,
            permission.run_id,
            permission.tool_call_id,
            true
        )
        .is_err()
    );
    assert!(
        h.permission(
            "main",
            permission.id,
            RunId::default(),
            permission.tool_call_id,
            true
        )
        .is_err()
    );
    assert!(
        h.permission(
            "main",
            permission.id,
            permission.run_id,
            ToolCallId::default(),
            true
        )
        .is_err()
    );
    let old = h.attach("main", 0).unwrap();
    let new = h.attach("main", 0).unwrap();
    assert!(h.batch("main", old, None).is_err());
    h.batch("main", new, None).unwrap();
    assert!(
        h.snapshot("main")
            .unwrap()
            .run
            .unwrap()
            .permission
            .is_some()
    ); // renderer reattachment cannot grant.
    h.permission(
        "main",
        permission.id,
        permission.run_id,
        permission.tool_call_id,
        true,
    )
    .unwrap();
    assert!(
        h.permission(
            "main",
            permission.id,
            permission.run_id,
            permission.tool_call_id,
            true
        )
        .is_err()
    );
    until(&h, |s| {
        s.run.as_ref().is_some_and(|r| r.status == "completed")
    })
    .await;
    assert!(h.cancel("main", id).is_err());
}
#[tokio::test]
async fn quitting_blocked_approval_cancels_cleanup_and_restores_only_idle_history() {
    let root = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![response(
        vec![support::call(
            "write",
            "write_file",
            json!({"path":"unwanted","content":"x","overwrite":false}),
        )],
        "Pending",
    )]);
    let h = host(provider, root.path(), user.path());
    h.workflow("main", Workflow::CodingAsk).unwrap();
    h.submit("main", task(Uuid::new_v4())).unwrap();
    until(&h, |s| {
        s.run.as_ref().is_some_and(|r| r.permission.is_some())
    })
    .await;
    tokio::time::timeout(Duration::from_secs(5), h.shutdown())
        .await
        .unwrap()
        .unwrap();
    assert!(!root.path().join("unwanted").exists());
    assert_eq!(h.snapshot("main").unwrap().run.unwrap().status, "cancelled");
    let previous = h.snapshot("main").unwrap().selected_session;
    drop(h);
    let restored = host(Script::new(vec![]), root.path(), user.path());
    assert_eq!(
        restored.snapshot("main").unwrap().selected_session,
        previous
    );
    assert_eq!(restored.snapshot("main").unwrap().workflow, Workflow::Notes);
    restored.submit("main", task(Uuid::new_v4())).unwrap();
    until(&restored, |s| {
        s.run
            .as_ref()
            .is_some_and(|r| r.status.starts_with("rejected"))
    })
    .await;
}
#[tokio::test]
async fn stalled_projection_is_bounded_with_gaps_and_runtime_keeps_polling() {
    let root = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let text = "x".repeat(600 * 2048);
    let provider = Script::options(vec![response(vec![], &text)], false, 2048);
    let h = host(provider, root.path(), user.path());
    let sub = h.attach("main", 0).unwrap();
    let first = h.batch("main", sub, None).unwrap();
    let id = Uuid::new_v4();
    h.submit("main", task(id)).unwrap();
    until(&h, |s| {
        s.run.as_ref().is_some_and(|r| r.status == "completed")
    })
    .await;
    assert!(h.batch("main", sub, None).is_err());
    let batch = h.batch("main", sub, Some(first.batch_id)).unwrap();
    assert!(batch.coverage_gap);
    assert!(serde_json::to_vec(&batch).unwrap().len() <= PAGE_BYTES);
    assert!(!batch.events.is_empty());
    for pair in batch.events.windows(2) {
        assert!(pair[0].ordinal < pair[1].ordinal);
        assert!(pair[0].sequence < pair[1].sequence);
    }
    assert!(
        !serde_json::to_string(&batch)
            .unwrap()
            .contains("PRIVATE_CONTINUATION_SENTINEL")
    );
    let history = h.history("main", 0).unwrap();
    assert!(serde_json::to_vec(&history).unwrap().len() <= PAGE_BYTES);
    assert!(history.messages.iter().any(|m| m.text.omitted_bytes > 0));
}
#[tokio::test]
async fn view_loss_does_not_cancel_provider_but_explicit_cancel_does() {
    let root = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let provider = Script::options(vec![], true, 0);
    let h = host(provider.clone(), root.path(), user.path());
    let id = Uuid::new_v4();
    h.submit("main", task(id)).unwrap();
    until(&h, |_| provider.calls.load(Ordering::SeqCst) > 0).await;
    let view = h.attach("main", 0).unwrap();
    h.batch("main", view, None).unwrap();
    let _replacement = h.attach("main", 0).unwrap();
    assert_eq!(h.snapshot("main").unwrap().run.unwrap().status, "running");
    h.cancel("main", id).unwrap();
    until(&h, |s| {
        s.run.as_ref().is_some_and(|r| r.status == "cancelled")
    })
    .await;
}
#[tokio::test]
async fn session_selection_is_idle_only_and_failed_save_remains_visible() {
    let root = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let h = host(
        Script::new(vec![response(vec![], "hello")]),
        root.path(),
        user.path(),
    );
    let original = h.snapshot("main").unwrap().selected_session.unwrap();
    let second = h.new_session("main").unwrap();
    h.select_session("main", original).unwrap();
    assert_eq!(h.snapshot("main").unwrap().selected_session, Some(original));
    // Deterministic store corruption: do not silently overwrite it during idle save.
    let sessions = user.path().join("sessions");
    let partition = std::fs::read_dir(sessions)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::write(partition.join("state.json"), "corrupt").unwrap();
    assert!(h.select_session("main", second).is_err());
    assert!(h.snapshot("main").unwrap().save_error.is_some());
    assert!(h.shutdown().await.is_err());
}
#[tokio::test]
async fn clipped_permission_cannot_grant_and_cancellation_wins_before_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let provider = Script::new(vec![response(
        vec![support::call(
            "shell",
            "shell",
            json!({"command":format!("printf {}", "x".repeat(4096))}),
        )],
        "Inspect",
    )]);
    let h = host(provider, root.path(), user.path());
    h.workflow("main", Workflow::CodingAsk).unwrap();
    let id = Uuid::new_v4();
    h.submit("main", task(id)).unwrap();
    until(&h, |s| {
        s.run.as_ref().is_some_and(|r| r.permission.is_some())
    })
    .await;
    let p = h.snapshot("main").unwrap().run.unwrap().permission.unwrap();
    assert!(!p.can_approve);
    assert!(p.command.omitted_bytes > 0);
    assert!(
        h.permission("main", p.id, p.run_id, p.tool_call_id, true)
            .is_err()
    );
    h.cancel("main", id).unwrap();
    assert!(
        h.permission("main", p.id, p.run_id, p.tool_call_id, true)
            .is_err()
    );
    until(&h, |s| {
        s.run.as_ref().is_some_and(|r| r.status == "cancelled")
    })
    .await;
}
#[tokio::test]
async fn acknowledgement_is_single_use_and_foreign_or_stale_subscription_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let h = host(Script::new(vec![]), root.path(), user.path());
    let id = h.attach("main", 0).unwrap();
    let batch = h.batch("main", id, None).unwrap();
    assert!(h.acknowledge("remote", id, batch.batch_id).is_err());
    assert!(h.acknowledge("main", id, batch.batch_id + 1).is_err());
    h.acknowledge("main", id, batch.batch_id).unwrap();
    assert!(h.acknowledge("main", id, batch.batch_id).is_err());
    h.detach("main", id).unwrap();
    assert!(!h.attached("main", id).unwrap());
}
#[tokio::test]
async fn saved_history_keeps_private_replay_while_visible_errors_and_projections_omit_it() {
    let root = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let h = host(
        Script::new(vec![response(vec![], "visible checklist")]),
        root.path(),
        user.path(),
    );
    h.submit("main", task(Uuid::new_v4())).unwrap();
    until(&h, |s| {
        s.run.as_ref().is_some_and(|r| r.status == "completed")
    })
    .await;
    let partition = std::fs::read_dir(user.path().join("sessions"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let file = partition.join("state.json");
    let private = std::fs::read_to_string(&file).unwrap();
    assert!(private.contains("PRIVATE_CONTINUATION_SENTINEL"));
    for projection in [
        serde_json::to_string(&h.snapshot("main").unwrap()).unwrap(),
        serde_json::to_string(&h.history("main", 0).unwrap()).unwrap(),
    ] {
        assert!(!projection.contains("PRIVATE_CONTINUATION_SENTINEL"));
        assert!(!projection.contains("test-only-credential"));
    }
    std::fs::write(&file, "\"PRIVATE_CONTINUATION_SENTINEL\"").unwrap();
    let error = h.shutdown().await.unwrap_err().to_string();
    assert!(!error.contains("PRIVATE_CONTINUATION_SENTINEL"));
}
#[test]
fn visible_text_preserves_utf8_boundaries_and_serialized_single_message_cap() {
    let text = astrid::host::view::Text::new(&"🦀".repeat(1000));
    assert_eq!(text.value.len(), 2048);
    assert_eq!(text.omitted_bytes, 1952);
    let message = astrid::host::view::MessageView::project(0, &Message::User("\0".repeat(65536)));
    assert!(serde_json::to_vec(&message).unwrap().len() < PAGE_BYTES);
}
