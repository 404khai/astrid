mod support;
use astrid::{
    cancellation::Cancellation,
    events::{EventKind, ExecutionState},
    model::Message,
    permissions::PermissionMode,
    runtime::{self, PermissionHandler, RunConfig, RunOutcome, Session},
    tools::{PermissionRequest, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::json;
use std::{io, time::Duration};
use support::{Confirmation, Server, call, reply};

fn config(task: &str, mode: PermissionMode) -> RunConfig {
    RunConfig {
        model: "fixture".into(),
        task: task.into(),
        max_model_calls: 4,
        permissions: mode.policy(),
        context_budget: Some(Default::default()),
    }
}
fn tools(root: &std::path::Path) -> Tools {
    Tools::new(Workspace::new(root).unwrap(), Duration::from_secs(5)).unwrap()
}

#[tokio::test]
async fn followup_pruning_and_replay_keep_historical_submission_whole() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let large = "historical data ".repeat(1800);
    let server = Server::start(vec![
        reply(&[], &large),
        reply(&[], &large),
        reply(&[], "recent answer"),
        reply(&[], "current answer"),
    ])
    .await;
    let mut session = Session::new(tools.workspace());
    for task in ["original task", large.as_str(), "recent question"] {
        let result = runtime::run_in_session(
            &server.provider,
            &tools,
            &mut Confirmation::new(false),
            config(task, PermissionMode::Auto),
            Cancellation::default(),
            None,
            session,
        )
        .await
        .unwrap();
        assert_eq!(result.outcome, RunOutcome::Completed);
        session = result.session;
    }
    let mut config = config("current question", PermissionMode::Auto);
    config.context_budget.as_mut().unwrap().max_request_bytes = 14_000;
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    let collect = async {
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }
        events
    };
    let mut permissions = Confirmation::new(false);
    let (result, events) = tokio::join!(
        runtime::run_in_session(
            &server.provider,
            &tools,
            &mut permissions,
            config,
            Cancellation::default(),
            Some(tx),
            session
        ),
        collect
    );
    let result = result.unwrap();
    assert_eq!(result.outcome, RunOutcome::Completed);
    let obsolete_question = result
        .session
        .context
        .items
        .iter()
        .find(|item| item.message_index == Some(2))
        .unwrap()
        .id;
    let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
    for event in &events {
        if let EventKind::ContextSelected { .. } = &event.kind {
            let mut invalid = event.clone();
            if let EventKind::ContextSelected { selection } = &mut invalid.kind {
                let decision = selection
                    .decisions
                    .iter_mut()
                    .find(|decision| decision.item_id == obsolete_question)
                    .unwrap();
                assert!(!decision.retained);
                decision.retained = true;
                selection.summary = None;
            }
            let before = replay.clone();
            assert!(replay.transition(&invalid).is_err());
            assert_eq!(replay, before);
        }
        replay.transition(event).unwrap();
    }
    assert_eq!(replay, result.state);
    let requests = server.finish().await;
    let users = requests[3]["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["role"] == "user")
        .map(|item| item["content"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(users.contains(&"current question"));
    assert!(users.contains(&"recent question"));
    assert!(!users.contains(&large.as_str()));
}

#[tokio::test]
async fn followups_preserve_conversation_ids_context_and_per_run_events() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let server = Server::start(vec![
        reply(&[], "first answer"),
        reply(&[], "second answer"),
        reply(&[], "independent answer"),
    ])
    .await;
    let mut permission = Confirmation::new(false);
    let first = runtime::run(
        &server.provider,
        &tools,
        &mut permission,
        config("first question", PermissionMode::Auto),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    let first_run = first.state.run_id;
    let session_id = first.session.id;
    let prior_items = first.session.context.items.len();
    let (tx, mut rx) = tokio::sync::mpsc::channel(2);
    let collect = async {
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }
        events
    };
    let (second, events) = tokio::join!(
        runtime::run_in_session(
            &server.provider,
            &tools,
            &mut permission,
            config("second question", PermissionMode::Unbound),
            Cancellation::default(),
            Some(tx),
            first.session
        ),
        collect
    );
    let second = second.unwrap();
    assert_eq!(second.outcome, RunOutcome::Completed);
    assert_eq!(second.session.id, session_id);
    assert_ne!(second.state.run_id, first_run);
    assert_eq!(second.session.messages.len(), 4);
    assert!(second.session.context.items.len() > prior_items);
    let mut replay = ExecutionState::new(session_id, second.state.run_id);
    for event in &events {
        assert_eq!(event.session_id, session_id);
        replay.transition(event).unwrap();
    }
    assert_eq!(replay, second.state);
    assert!(events.iter().any(|event| matches!(&event.kind, EventKind::ContextItemAdded { item } if item.message_index == Some(2))));
    assert!(events.iter().any(|event| matches!(&event.kind, EventKind::PermissionsConfigured { policy } if *policy == PermissionMode::Unbound.policy())));
    let independent = runtime::run(
        &server.provider,
        &tools,
        &mut permission,
        config("independent question", PermissionMode::Auto),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    assert_ne!(independent.session.id, session_id);
    let requests = server.finish().await;
    assert!(requests[1]["input"].to_string().contains("first answer"));
    assert!(requests[1]["input"].to_string().contains("first question"));
    assert!(requests[1]["input"].to_string().contains("second question"));
    assert!(!requests[2]["input"].to_string().contains("first answer"));
    assert!(permission.commands.is_empty());
}

struct CancelAtApproval(Cancellation);
#[async_trait]
impl PermissionHandler for CancelAtApproval {
    async fn decide(&mut self, _: &PermissionRequest) -> io::Result<bool> {
        self.0.cancel();
        Ok(true)
    }
}

#[tokio::test]
async fn incomplete_cancelled_batch_is_preserved_and_never_replayed() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let server = Server::start(vec![reply(
        &[
            call(
                "write-once",
                "write_file",
                json!({"path":"committed", "content":"one", "overwrite":false}),
            ),
            call(
                "cancelled-shell",
                "shell",
                json!({"command":"touch forbidden"}),
            ),
        ],
        "working",
    )])
    .await;
    let cancel = Cancellation::default();
    let first = runtime::run(
        &server.provider,
        &tools,
        &mut CancelAtApproval(cancel.clone()),
        config("mutate", PermissionMode::Auto),
        cancel,
        None,
    )
    .await
    .unwrap();
    assert_eq!(first.outcome, RunOutcome::Cancelled);
    assert_eq!(
        std::fs::read_to_string(root.path().join("committed")).unwrap(),
        "one"
    );
    assert!(!root.path().join("forbidden").exists());
    let id = first.session.id;
    let messages = first.session.messages.len();
    let error = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut Confirmation::new(true),
        config("continue", PermissionMode::Unbound),
        Cancellation::default(),
        None,
        first.session,
    )
    .await
    .unwrap_err();
    assert!(error.message.contains("incomplete tool batch"));
    assert_eq!(error.session.id, id);
    assert_eq!(error.session.messages.len(), messages);
    assert_eq!(server.finish().await.len(), 1);
    assert!(!root.path().join("forbidden").exists());
}

#[tokio::test]
async fn rejected_workspace_and_configuration_return_unchanged_session() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let server = Server::start(vec![]).await;
    let session = Session::new(&Workspace::new(other.path()).unwrap());
    let id = session.id;
    let error = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut Confirmation::new(true),
        config("question", PermissionMode::Auto),
        Cancellation::default(),
        None,
        session,
    )
    .await
    .unwrap_err();
    assert!(error.message.contains("workspace differs"));
    assert_eq!(error.session.id, id);
    assert!(error.session.messages.is_empty());
    let session = Session::new(tools.workspace());
    let id = session.id;
    let error = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut Confirmation::new(true),
        config("", PermissionMode::Auto),
        Cancellation::default(),
        None,
        session,
    )
    .await
    .unwrap_err();
    assert_eq!(error.session.id, id);
    assert!(error.session.messages.is_empty());
    let mut session = Session::new(tools.workspace());
    session
        .messages
        .push(Message::User("untracked message".into()));
    let id = session.id;
    let error = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut Confirmation::new(true),
        config("question", PermissionMode::Auto),
        Cancellation::default(),
        None,
        session,
    )
    .await
    .unwrap_err();
    assert_eq!(error.session.id, id);
    assert!(error.message.contains("lack context provenance"));
    assert_eq!(error.session.messages.len(), 1);
    assert!(server.finish().await.is_empty());
}

#[tokio::test]
async fn reused_provider_call_id_across_runs_never_reexecutes_mutation() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let server = Server::start(vec![
        reply(
            &[call(
                "unique-write",
                "write_file",
                json!({"path":"once", "content":"original", "overwrite":false}),
            )],
            "",
        ),
        reply(&[], "done"),
        reply(
            &[call(
                "unique-write",
                "write_file",
                json!({"path":"once", "content":"replayed", "overwrite":true}),
            )],
            "",
        ),
    ])
    .await;
    let first = runtime::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config("first task", PermissionMode::Unbound),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(first.outcome, RunOutcome::Completed);
    let second = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config("follow up", PermissionMode::Unbound),
        Cancellation::default(),
        None,
        first.session,
    )
    .await
    .unwrap();
    assert!(matches!(second.outcome, RunOutcome::Failed { .. }));
    assert!(second.state.tools.is_empty());
    assert_eq!(
        std::fs::read_to_string(root.path().join("once")).unwrap(),
        "original"
    );
    assert_eq!(server.finish().await.len(), 3);
}

#[tokio::test]
async fn cancelled_model_without_committed_batch_can_accept_followup() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let server = Server::start(vec![reply(&[], "continued")]).await;
    let cancel = Cancellation::default();
    cancel.cancel();
    let first = runtime::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config("cancelled task", PermissionMode::Auto),
        cancel,
        None,
    )
    .await
    .unwrap();
    assert_eq!(first.outcome, RunOutcome::Cancelled);
    assert!(matches!(&first.session.messages[..], [Message::User(_)]));
    let second = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config("follow up", PermissionMode::Auto),
        Cancellation::default(),
        None,
        first.session,
    )
    .await
    .unwrap();
    assert_eq!(second.outcome, RunOutcome::Completed);
    let requests = server.finish().await;
    assert_eq!(requests.len(), 1);
    assert!(requests[0]["input"].to_string().contains("follow up"));
}

#[tokio::test]
async fn presets_enforce_edit_and_shell_approvals_and_keep_path_validation() {
    for mode in [
        PermissionMode::Ask,
        PermissionMode::Auto,
        PermissionMode::Unbound,
    ] {
        let root = tempfile::tempdir().unwrap();
        let tools = tools(root.path());
        let server = Server::start(vec![
            reply(
                &[
                    call(
                        "edit",
                        "write_file",
                        json!({"path":"written", "content":"yes", "overwrite":false}),
                    ),
                    call("execute", "shell", json!({"command":"touch executed"})),
                    call(
                        "outside",
                        "write_file",
                        json!({"path":"../escape", "content":"no", "overwrite":true}),
                    ),
                ],
                "",
            ),
            reply(&[], "done"),
        ])
        .await;
        let mut permission = Confirmation::new(false);
        let result = runtime::run(
            &server.provider,
            &tools,
            &mut permission,
            config("work", mode),
            Cancellation::default(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.outcome, RunOutcome::Completed);
        assert_eq!(
            root.path().join("written").exists(),
            mode != PermissionMode::Ask
        );
        assert_eq!(
            root.path().join("executed").exists(),
            mode == PermissionMode::Unbound
        );
        assert_eq!(
            permission.commands.len(),
            match mode {
                PermissionMode::Ask => 3,
                PermissionMode::Auto => 1,
                PermissionMode::Unbound => 0,
            }
        );
        assert!(result.session.messages.iter().any(|message| matches!(message, Message::Tool(result) if result.call_id == "outside" && result.is_error())));
        server.finish().await;
    }
}
