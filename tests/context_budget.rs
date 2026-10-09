mod support;
use astrid::{
    auth::{AuthError, Authentication, BearerToken},
    cancellation::Cancellation,
    context::{ContextBudget, ContextSnapshot},
    events::{EventKind, ExecutionEvent, ExecutionState},
    model::{ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink},
    openai::OpenAiProvider,
    runtime::{self, RunConfig, RunOutcome, RunResult},
    tools::Tools,
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use support::{Confirmation, Server, call, reply};
use tokio::sync::{Notify, mpsc};

fn config(budget: ContextBudget) -> RunConfig {
    RunConfig {
        observability: None,
        model: "test-model".into(),
        task: "repair target.rs".into(),
        max_model_calls: 8,
        permissions: Default::default(),
        context_budget: Some(budget),
    }
}
fn tools(root: &std::path::Path) -> Tools {
    Tools::new(Workspace::new(root).unwrap(), Duration::from_secs(10)).unwrap()
}
async fn recorded(
    provider: &dyn ModelProvider,
    tools: &Tools,
    config: RunConfig,
    cancel: Cancellation,
) -> (RunResult, Vec<ExecutionEvent>) {
    let (sender, mut receiver) = mpsc::channel(2);
    let mut permissions = Confirmation::new(false);
    let consume = async {
        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        events
    };
    let (result, events) = tokio::join!(
        runtime::run(
            provider,
            tools,
            &mut permissions,
            config,
            cancel,
            Some(sender)
        ),
        consume
    );
    let result = result.unwrap();
    let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
    for event in &events {
        replay.transition(event).unwrap();
    }
    assert_eq!(replay, result.state);
    (result, events)
}

#[tokio::test]
async fn budgeted_run_inspects_prunes_compacts_and_keeps_global_call_ids() {
    for reused in [false, true] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("target.rs"),
            "target evidence ".repeat(600),
        )
        .unwrap();
        std::fs::write(root.path().join("noise.rs"), "noise evidence ".repeat(600)).unwrap();
        std::fs::write(root.path().join("latest.rs"), "latest").unwrap();
        let first = call("old", "read_file", json!({"path":"target.rs"}));
        let noise = call("noise", "read_file", json!({"path":"noise.rs"}));
        let latest = call("latest", "read_file", json!({"path":"latest.rs"}));
        let last = if reused {
            reply(std::slice::from_ref(&noise), "reuse")
        } else {
            reply(&[], "done")
        };
        let server = Server::start(vec![
            reply(&[first], "read target"),
            reply(&[noise], "read noise"),
            reply(&[latest], "read latest"),
            last,
        ])
        .await;
        let budget = ContextBudget {
            estimated_context_tokens: 100_000,
            response_reserve_tokens: 1000,
            // Numbered read results retain both raw content and line records.
            // Admit one large protected exchange, but still force pruning when
            // both large reads are present; the assertions below prove this.
            max_request_bytes: 28_000,
            max_summary_bytes: 1200,
            ..Default::default()
        };
        let (result, events) = recorded(
            &server.provider,
            &tools(root.path()),
            config(budget.clone()),
            Cancellation::default(),
        )
        .await;
        let wire_bytes = server.request_bytes.lock().unwrap().clone();
        // Fail before waiting for unused mock replies if admission stops early.
        assert_eq!(result.model_calls, 4, "{:?}", result.outcome);
        assert_eq!(result.tool_calls, 3);
        let requests = server.finish().await;
        if reused {
            assert!(matches!(&result.outcome,RunOutcome::Failed{code,..} if code=="protocol"));
        } else {
            assert_eq!(result.outcome, RunOutcome::Completed);
        }
        let selections = events
            .iter()
            .filter_map(|event| match &event.kind {
                EventKind::ContextSelected { selection } => Some(selection),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(selections.len(), 4);
        for (selection, actual_bytes) in selections.iter().zip(&wire_bytes) {
            assert_eq!(selection.snapshot.serialized_request_bytes, *actual_bytes);
            assert!(budget.admits(&selection.snapshot));
        }
        assert!(selections.iter().any(|selection| {
            selection
                .decisions
                .iter()
                .any(|decision| !decision.retained)
        }));
        assert!(
            selections
                .iter()
                .any(|selection| selection.summary.is_some())
        );
        let final_selection = selections.last().unwrap();
        let final_request = requests.last().unwrap();
        if reused {
            assert!(
                final_request["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|item| item["call_id"] != "noise")
            );
        }
        if let Some(summary) = &final_selection.summary {
            assert!(summary.incomplete && summary.text_bytes <= budget.max_summary_bytes);
            assert!(
                final_request["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["content"].as_str() == Some(summary.text.as_str()))
            );
            assert_eq!(
                summary.source_items,
                final_selection
                    .decisions
                    .iter()
                    .filter(|decision| !decision.retained)
                    .map(|decision| decision.item_id)
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(result.session.messages.len(), if reused { 7 } else { 8 });
        // Forged metadata cannot rewrite protection, lineage, or prepared sizes.
        let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
        for event in &events {
            if let EventKind::ContextSelected { selection } = &event.kind {
                let before = replay.clone();
                let mut forged = event.clone();
                let mut changed = selection.clone();
                changed.decisions[0].retained = false;
                forged.kind = EventKind::ContextSelected { selection: changed };
                assert!(replay.transition(&forged).is_err());
                assert_eq!(replay, before);
                let latest = replay.context_items.iter().rev().find_map(|item| {
                    if let astrid::context::ContextOrigin::Assistant { model_call_id } = item.origin
                    {
                        Some(model_call_id)
                    } else {
                        None
                    }
                });
                if let Some(index) = replay.context_items.iter().position(|item| matches!(item.origin,astrid::context::ContextOrigin::Assistant{model_call_id} if Some(model_call_id)!=latest)) {
                    let mut changed=selection.clone();changed.decisions[index].retained = !changed.decisions[index].retained;
                    forged.kind=EventKind::ContextSelected{selection:changed};
                    assert!(replay.transition(&forged).is_err());assert_eq!(replay,before);
                }
                if selection.summary.is_some() {
                    let mut changed = selection.clone();
                    changed.summary.as_mut().unwrap().source_items.clear();
                    forged.kind = EventKind::ContextSelected { selection: changed };
                    assert!(replay.transition(&forged).is_err());
                    assert_eq!(replay, before);
                }
                replay.transition(event).unwrap();
                let before = replay.clone();
                let mut duplicate = event.clone();
                duplicate.sequence += 1;
                assert!(replay.transition(&duplicate).is_err());
                assert_eq!(replay, before);
            } else {
                if let EventKind::ContextPrepared { snapshot } = &event.kind {
                    let mut forged = event.clone();
                    let mut changed = snapshot.clone();
                    changed.serialized_request_bytes += 1;
                    forged.kind = EventKind::ContextPrepared { snapshot: changed };
                    let before = replay.clone();
                    assert!(replay.transition(&forged).is_err());
                    assert_eq!(replay, before);
                }
                replay.transition(event).unwrap();
            }
        }
    }
}

struct AuthSpy(AtomicUsize);
#[async_trait]
impl Authentication for AuthSpy {
    async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(AuthError::Invalid("unexpected authentication".into()))
    }
}
#[tokio::test]
async fn oversized_protected_request_fails_without_model_invocation_or_auth() {
    let root = tempfile::tempdir().unwrap();
    let auth = Arc::new(AuthSpy(AtomicUsize::new(0)));
    let provider = OpenAiProvider::new(auth.clone()).unwrap();
    let budget = ContextBudget {
        max_request_bytes: 1,
        ..Default::default()
    };
    let (result, events) = recorded(
        &provider,
        &tools(root.path()),
        config(budget),
        Cancellation::default(),
    )
    .await;
    assert!(matches!(result.outcome,RunOutcome::Failed{ref code,..} if code=="context"));
    assert_eq!(result.model_calls, 0);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    assert!(result.session.selection.is_none());
    assert!(!events.iter().any(|event| matches!(
        event.kind,
        EventKind::ModelCallStarted { .. } | EventKind::ContextSelected { .. }
    )));
}

struct CancelMeasurement<'a> {
    provider: &'a OpenAiProvider,
    calls: AtomicUsize,
    cancel: Cancellation,
    return_error: bool,
}
#[async_trait]
impl ModelProvider for CancelMeasurement<'_> {
    fn measure_request(
        &self,
        request: &ModelRequest<'_>,
    ) -> Result<Option<ContextSnapshot>, ModelError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 2 {
            self.cancel.cancel();
            if self.return_error {
                return Err(ModelError::Provider("accounting error".into()));
            }
            let mut snapshot = self.provider.measure_request(request)?.unwrap();
            snapshot.serialized_request_bytes = usize::MAX;
            return Ok(Some(snapshot));
        }
        self.provider.measure_request(request)
    }
    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        sink: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        self.provider.generate(request, sink).await
    }
}
#[tokio::test]
async fn cancellation_during_accounting_preserves_previous_selection_even_on_error() {
    for return_error in [false, true] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("file"), "content").unwrap();
        let read = call("read", "read_file", json!({"path":"file"}));
        let server = Server::start(vec![reply(&[read], "read")]).await;
        let cancel = Cancellation::default();
        let provider = CancelMeasurement {
            provider: &server.provider,
            calls: AtomicUsize::new(0),
            cancel: cancel.clone(),
            return_error,
        };
        let (result, events) = recorded(
            &provider,
            &tools(root.path()),
            config(ContextBudget::default()),
            cancel,
        )
        .await;
        assert_eq!(result.outcome, RunOutcome::Cancelled);
        assert_eq!(result.model_calls, 1);
        let selections = events
            .iter()
            .filter_map(|event| {
                if let EventKind::ContextSelected { selection } = &event.kind {
                    Some(selection)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(selections.len(), 1);
        assert_eq!(result.session.selection.as_ref(), Some(selections[0]));
        assert_eq!(server.finish().await.len(), 1);
    }
}

struct NotifyMeasurement {
    provider: OpenAiProvider,
    calls: AtomicUsize,
    prepared: Arc<Notify>,
}
#[async_trait]
impl ModelProvider for NotifyMeasurement {
    fn measure_request(
        &self,
        request: &ModelRequest<'_>,
    ) -> Result<Option<ContextSnapshot>, ModelError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
            self.prepared.notify_one();
        }
        self.provider.measure_request(request)
    }
    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        sink: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        self.provider.generate(request, sink).await
    }
}
#[tokio::test]
async fn stalled_selection_publication_can_cancel_before_any_model_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let auth = Arc::new(AuthSpy(AtomicUsize::new(0)));
    let prepared = Arc::new(Notify::new());
    let provider = NotifyMeasurement {
        provider: OpenAiProvider::new(auth.clone()).unwrap(),
        calls: AtomicUsize::new(0),
        prepared: prepared.clone(),
    };
    let cancel = Cancellation::default();
    let mut permissions = Confirmation::new(false);
    let (sender, mut receiver) = mpsc::channel(1);
    let execution = runtime::run(
        &provider,
        &tools,
        &mut permissions,
        config(ContextBudget::default()),
        cancel.clone(),
        Some(sender),
    );
    tokio::pin!(execution);
    let mut history = Vec::new();
    loop {
        tokio::select! {result=&mut execution=>panic!("unexpected completion: {result:?}"),event=receiver.recv()=>history.push(event.unwrap())}
        if matches!(history.last().unwrap().kind,EventKind::ContextItemAdded{ref item} if matches!(item.origin,astrid::context::ContextOrigin::OperatingInstructions))
        {
            break;
        }
    }
    // TurnStarted fills the slot; ContextSelected commits then awaits delivery.
    tokio::select! {biased;result=&mut execution=>panic!("unexpected completion: {result:?}"),()=prepared.notified()=>{}}
    cancel.cancel();
    let consume = async {
        while let Some(event) = receiver.recv().await {
            history.push(event);
        }
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(execution, consume)
    })
    .await
    .unwrap();
    let result = result.unwrap();
    assert_eq!(result.outcome, RunOutcome::Cancelled);
    assert_eq!(result.model_calls, 0);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    assert!(result.session.selection.is_some());
    let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
    for event in &history {
        replay.transition(event).unwrap();
    }
    assert_eq!(replay, result.state);
}

struct NoAccounting;
#[async_trait]
impl ModelProvider for NoAccounting {
    async fn generate(
        &self,
        _: &ModelRequest<'_>,
        _: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        panic!("missing accounting must stop before inference")
    }
}
#[tokio::test]
async fn configured_provider_without_accounting_fails_explicitly() {
    let root = tempfile::tempdir().unwrap();
    let (result, _) = recorded(
        &NoAccounting,
        &tools(root.path()),
        config(ContextBudget::default()),
        Cancellation::default(),
    )
    .await;
    assert_eq!(result.model_calls, 0);
    assert!(
        matches!(result.outcome,RunOutcome::Failed{ref message,..} if message.contains("does not expose request accounting"))
    );
}

struct Drift(OpenAiProvider);
#[async_trait]
impl ModelProvider for Drift {
    fn measure_request(
        &self,
        request: &ModelRequest<'_>,
    ) -> Result<Option<ContextSnapshot>, ModelError> {
        let mut snapshot = self.0.measure_request(request)?.unwrap();
        snapshot.serialized_request_bytes += 1;
        snapshot
            .measurements
            .iter_mut()
            .find(|measurement| {
                measurement.source == astrid::context::ContextSource::RequestFraming
            })
            .unwrap()
            .serialized_bytes += 1;
        snapshot.non_opaque_json_size_token_heuristic =
            snapshot.serialized_request_bytes.div_ceil(4);
        Ok(Some(snapshot))
    }
    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        sink: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        self.0.generate(request, sink).await
    }
}
#[tokio::test]
async fn accounting_drift_is_rejected_before_authentication_and_http() {
    let root = tempfile::tempdir().unwrap();
    let auth = Arc::new(AuthSpy(AtomicUsize::new(0)));
    let provider = Drift(OpenAiProvider::new(auth.clone()).unwrap());
    let (result, events) = recorded(
        &provider,
        &tools(root.path()),
        config(ContextBudget::default()),
        Cancellation::default(),
    )
    .await;
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    assert!(
        matches!(result.outcome,RunOutcome::Failed{ref message,..} if message.contains("disagrees with context selection"))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event.kind, EventKind::ContextPrepared { .. }))
    );
}
