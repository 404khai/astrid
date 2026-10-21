mod support;
use astrid::{
    cancellation::Cancellation,
    context::ContextBudget,
    events::{EventKind, ExecutionState},
    model::{Message, ModelProvider, ModelRequest},
    runtime::{self, RunConfig, RunOutcome},
    tools::Tools,
    workspace::Workspace,
};
use serde_json::json;
use std::time::Duration;
use support::{Confirmation, Server, call, reply};

fn config(budget: Option<ContextBudget>) -> RunConfig {
    RunConfig {
        routing: None,
        observability: None,
        model: "fixture".into(),
        task: "inspect".into(),
        max_model_calls: 4,
        context_budget: budget,
        permissions: Default::default(),
    }
}
fn tools(root: &std::path::Path) -> Tools {
    Tools::new(Workspace::new(root).unwrap(), Duration::from_secs(2)).unwrap()
}

#[tokio::test]
async fn aggregate_escaped_unicode_outputs_stop_later_mutation_at_both_admission_limits() {
    for bytes in [true, false] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("large"), "世界🙂 \"\\\n".repeat(6000)).unwrap();
        let t = tools(root.path());
        let server = Server::start(vec![reply(
            &[
                call("first", "read_file", json!({"path":"large"})),
                call("second", "read_file", json!({"path":"large"})),
                call(
                    "never",
                    "write_file",
                    json!({"path":"forbidden","content":"not written","overwrite":false}),
                ),
            ],
            "read",
        )])
        .await;
        let budget = if bytes {
            ContextBudget {
                max_request_bytes: 100_000,
                estimated_context_tokens: 1_000_000,
                ..Default::default()
            }
        } else {
            ContextBudget {
                estimated_context_tokens: 29_096,
                ..Default::default()
            }
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let consume = async {
            let mut events = Vec::new();
            while let Some(event) = rx.recv().await {
                events.push(event);
            }
            events
        };
        let mut permission = Confirmation::new(true);
        let (result, events) = tokio::join!(
            runtime::run(
                &server.provider,
                &t,
                &mut permission,
                config(Some(budget.clone())),
                Cancellation::default(),
                Some(tx)
            ),
            consume
        );
        let result = result.unwrap();
        assert!(
            matches!(&result.outcome, RunOutcome::Failed { code, .. } if code=="context"),
            "{:?}",
            result.outcome
        );
        assert!(
            !root.path().join("forbidden").exists(),
            "remaining tool executed after aggregate overflow"
        );
        assert_eq!(result.model_calls, 1);
        let results = result
            .session
            .messages
            .iter()
            .filter_map(|m| {
                if let Message::Tool(r) = m {
                    Some(r)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            results.len(),
            2,
            "first output should fit independently: {:?}",
            result.outcome
        );
        assert_eq!(results[0].outcome, results[1].outcome);
        let instructions = server.requests.lock().unwrap()[0]["instructions"]
            .as_str()
            .unwrap()
            .to_owned();
        let measure = |messages: &[Message]| {
            server
                .provider
                .measure_request(&ModelRequest {
                    options: Default::default(),
                    model: "fixture",
                    instructions: &instructions,
                    messages,
                })
                .unwrap()
                .unwrap()
        };
        assert!(budget.admits(&measure(&result.session.messages[..3])));
        assert!(!budget.admits(&measure(&result.session.messages)));
        assert!(
            serde_json::to_string(&results[0])
                .unwrap()
                .contains("世界🙂")
        );
        // The completed results plus mandatory instructions/request are what
        // crosses the limit, not a per-tool output-limit violation.
        let complete_outcomes = events
            .iter()
            .filter_map(|e| {
                if let EventKind::ToolCallCompleted { outcome } = &e.kind {
                    Some(outcome)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(complete_outcomes.len(), 2);
        for (result, outcome) in results.iter().zip(complete_outcomes) {
            assert_eq!(&result.outcome, outcome);
        }
        assert!(!root.path().join("forbidden").exists());
        let skipped = result
            .state
            .tools
            .values()
            .find(|s| s.call.call_id == "never")
            .unwrap();
        assert_eq!(skipped.status, astrid::events::Status::Skipped);
        let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
        for event in &events {
            replay.transition(event).unwrap();
        }
        assert_eq!(replay, result.state);
        let before = serde_json::to_vec(&result.session).unwrap();
        let error = runtime::run_in_session(
            &server.provider,
            &t,
            &mut permission,
            config(Some(budget)),
            Cancellation::default(),
            None,
            result.session,
        )
        .await
        .unwrap_err();
        assert!(error.message.contains("incomplete tool batch"));
        assert_eq!(serde_json::to_vec(&error.session).unwrap(), before);
        assert_eq!(server.finish().await.len(), 1);
    }
}

#[tokio::test]
async fn oversized_tool_request_batch_is_rejected_before_any_tool_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let t = tools(root.path());
    let server = Server::start(vec![reply(
        &[
            call(
                "huge",
                "write_file",
                json!({"path":"forbidden","content":"x".repeat(120_000),"overwrite":false}),
            ),
            call("later", "read_file", json!({"path":"forbidden"})),
        ],
        "write",
    )])
    .await;
    let result = runtime::run(
        &server.provider,
        &t,
        &mut Confirmation::new(true),
        config(Some(ContextBudget {
            max_request_bytes: 30_000,
            ..Default::default()
        })),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    assert!(matches!(result.outcome, RunOutcome::Failed { ref code, .. } if code=="context"));
    assert!(!root.path().join("forbidden").exists());
    assert!(
        result
            .state
            .tools
            .values()
            .all(|tool| tool.status == astrid::events::Status::Skipped)
    );
    assert!(
        !result
            .session
            .messages
            .iter()
            .any(|m| matches!(m, Message::Tool(_)))
    );
    assert_eq!(server.finish().await.len(), 1);
}

#[tokio::test]
async fn a_complete_final_exchange_overflow_stops_without_spending_another_model_call() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("large"), "large data\n".repeat(10_000)).unwrap();
    let t = tools(root.path());
    let server = Server::start(vec![reply(
        &[
            call(
                "committed",
                "write_file",
                json!({"path":"committed","content":"original","overwrite":false}),
            ),
            call("read", "read_file", json!({"path":"large"})),
        ],
        "read",
    )])
    .await;
    let result = runtime::run(
        &server.provider,
        &t,
        &mut Confirmation::new(false),
        config(Some(ContextBudget {
            max_request_bytes: 20_000,
            ..Default::default()
        })),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    assert!(matches!(result.outcome, RunOutcome::Failed { ref code, .. } if code=="context"));
    assert_eq!(result.model_calls, 1);
    assert_eq!(
        std::fs::read_to_string(root.path().join("committed")).unwrap(),
        "original"
    );
    assert_eq!(
        result.state.tools.values().next().unwrap().status,
        astrid::events::Status::Completed
    );
    assert!(matches!(
        result.session.messages.last(),
        Some(Message::Tool(_))
    ));
    assert_eq!(server.finish().await.len(), 1);
}

#[tokio::test]
async fn under_budget_and_explicitly_unbudgeted_runs_keep_complete_exchanges() {
    for budget in [Some(ContextBudget::default()), None] {
        let root = tempfile::tempdir().unwrap();
        let t = tools(root.path());
        let server = Server::start(vec![
            reply(
                &[
                    call(
                        "write",
                        "write_file",
                        json!({"path":"file","content":"small","overwrite":false}),
                    ),
                    call("read", "read_file", json!({"path":"file"})),
                ],
                "tools",
            ),
            reply(&[], "done"),
        ])
        .await;
        let result = runtime::run(
            &server.provider,
            &t,
            &mut Confirmation::new(true),
            config(budget),
            Cancellation::default(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.outcome, RunOutcome::Completed);
        assert_eq!(
            std::fs::read_to_string(root.path().join("file")).unwrap(),
            "small"
        );
        assert_eq!(result.model_calls, 2);
        assert_eq!(server.finish().await.len(), 2);
    }
}

struct CancelAtResult<'a> {
    provider: &'a astrid::openai::OpenAiProvider,
    cancel: Cancellation,
    error: bool,
}
#[async_trait::async_trait]
impl ModelProvider for CancelAtResult<'_> {
    fn measure_request(
        &self,
        request: &ModelRequest<'_>,
    ) -> Result<Option<astrid::context::ContextSnapshot>, astrid::model::ModelError> {
        if request
            .messages
            .iter()
            .any(|message| matches!(message, Message::Tool(_)))
        {
            self.cancel.cancel();
            if self.error {
                return Err(astrid::model::ModelError::Protocol(
                    "accounting interrupted".into(),
                ));
            }
        }
        self.provider.measure_request(request)
    }
    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        sink: &mut dyn astrid::model::TextSink,
    ) -> Result<astrid::model::ModelResponse, astrid::model::ModelError> {
        self.provider.generate(request, sink).await
    }
}

#[tokio::test]
async fn cancellation_during_prefix_measurement_retains_results_and_wins_over_accounting_error() {
    for error in [false, true] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("file"), "retained").unwrap();
        let t = tools(root.path());
        let server = Server::start(vec![reply(
            &[
                call("read", "read_file", json!({"path":"file"})),
                call(
                    "never",
                    "write_file",
                    json!({"path":"forbidden","content":"not written","overwrite":false}),
                ),
            ],
            "read",
        )])
        .await;
        let cancel = Cancellation::default();
        let provider = CancelAtResult {
            provider: &server.provider,
            cancel: cancel.clone(),
            error,
        };
        let result = runtime::run(
            &provider,
            &t,
            &mut Confirmation::new(true),
            config(Some(ContextBudget::default())),
            cancel,
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.outcome, RunOutcome::Cancelled);
        assert!(result.state.cancellation_acknowledged);
        assert_eq!(result.model_calls, 1);
        assert_eq!(
            result
                .session
                .messages
                .iter()
                .filter(|m| matches!(m, Message::Tool(_)))
                .count(),
            1
        );
        assert!(!root.path().join("forbidden").exists());
        assert!(result.session.selection.is_some());
        assert_eq!(server.finish().await.len(), 1);
    }
}
