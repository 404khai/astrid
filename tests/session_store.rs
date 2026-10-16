mod support;
use astrid::{
    cancellation::Cancellation,
    context::ContextOrigin,
    model::{Message, ToolOutcome, ToolResult},
    permissions::PermissionMode,
    runtime::{self, RunConfig, RunOutcome, Session},
    session_store::{SessionSnapshot, SessionStore, StoredSession},
    tools::Tools,
    workspace::Workspace,
};
use serde_json::json;
use std::{fs, io, path::Path, time::Duration};
use support::{Confirmation, Server, call, reply};
fn state(workspace: &Workspace, task: &str) -> Session {
    let mut session = Session::new(workspace);
    session.messages.push(Message::User(task.into()));
    session.context.add(ContextOrigin::UserTask, Some(0));
    session
}
fn entry(session: Session) -> StoredSession {
    StoredSession {
        session,
        runs: 1,
        outcome: Some(RunOutcome::Completed),
    }
}
fn state_path(directory: &Path) -> std::path::PathBuf {
    fs::read_dir(directory.join("sessions"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("state.json")
}
fn config(task: &str) -> RunConfig {
    RunConfig {
        routing: None,
        observability: None,
        model: "fixture".into(),
        task: task.into(),
        max_model_calls: 3,
        permissions: PermissionMode::Auto.policy(),
        context_budget: Some(Default::default()),
    }
}
#[test]
fn sessions_active_selection_and_private_files_survive_reopen() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let first = state(&workspace, "first chat");
    let first_id = first.id;
    let second = state(&workspace, "second chat");
    let second_id = second.id;
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    store
        .save(&SessionSnapshot::new(
            &workspace,
            1,
            vec![entry(first), entry(second)],
        ))
        .unwrap();
    let path = state_path(directory.path());
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
    drop(store);
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    let snapshot = store.load().unwrap().unwrap();
    assert_eq!(snapshot.active, 1);
    assert_eq!(snapshot.entries[0].session.id, first_id);
    assert_eq!(snapshot.entries[1].session.id, second_id);
    assert!(
        matches!(&snapshot.entries[1].session.messages[0], Message::User(text) if text=="second chat")
    );
}
#[test]
fn exclusive_lock_and_workspace_isolation() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let other = Workspace::new(other.path()).unwrap();
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    assert_eq!(
        SessionStore::open(directory.path(), &workspace)
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    let other_store = SessionStore::open(directory.path(), &other).unwrap();
    assert!(other_store.load().unwrap().is_none());
    drop(store);
    assert!(SessionStore::open(directory.path(), &workspace).is_ok());
}
#[test]
fn invalid_stores_and_failed_saves_preserve_previous_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    let snapshot = SessionSnapshot::new(&workspace, 0, vec![entry(state(&workspace, "first"))]);
    store.save(&snapshot).unwrap();
    let path = state_path(directory.path());
    let original = fs::read(&path).unwrap();
    let invalid = SessionSnapshot::new(&workspace, 5, vec![entry(state(&workspace, "bad"))]);
    assert!(store.save(&invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    let huge = SessionSnapshot::new(
        &workspace,
        0,
        vec![entry(state(&workspace, &"x".repeat(8 * 1024 * 1024)))],
    );
    assert!(store.save(&huge).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    for value in [
        json!({"version":99}),
        {
            let mut v: serde_json::Value = serde_json::from_slice(&original).unwrap();
            v["version"] = json!(99);
            v
        },
        {
            let mut v: serde_json::Value = serde_json::from_slice(&original).unwrap();
            v["entries"][0]["session"]["context"]["items"][0]["message_index"] = json!(8);
            v
        },
    ] {
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(&snapshot).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}
#[cfg(unix)]
#[test]
fn symlink_store_files_and_directories_are_rejected() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    store
        .save(&SessionSnapshot::new(
            &workspace,
            0,
            vec![entry(state(&workspace, "first"))],
        ))
        .unwrap();
    let path = state_path(directory.path());
    let original = fs::read(&path).unwrap();
    let outside = directory.path().join("outside");
    fs::write(&outside, &original).unwrap();
    fs::rename(&path, path.with_extension("backup")).unwrap();
    symlink(&outside, &path).unwrap();
    assert!(store.load().is_err());
    assert!(
        store
            .save(&SessionSnapshot::new(
                &workspace,
                0,
                vec![entry(state(&workspace, "new"))]
            ))
            .is_err()
    );
    assert_eq!(fs::read(outside).unwrap(), original);
    let other = tempfile::tempdir().unwrap();
    symlink(directory.path(), other.path().join("sessions")).unwrap();
    assert!(SessionStore::open(other.path(), &workspace).is_err());
}
#[tokio::test]
async fn restored_followup_preserves_private_continuation_without_replaying_tools() {
    let root = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let tools = Tools::new(workspace.clone(), Duration::from_secs(2)).unwrap();
    let server = Server::start(vec![
        reply(
            &[call(
                "write-once",
                "write_file",
                json!({"path":"once.txt", "content":"created", "overwrite":false}),
            )],
            "",
        ),
        reply(&[], "first answer"),
        reply(&[], "second answer"),
    ])
    .await;
    let result = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config("first chat"),
        Cancellation::default(),
        None,
        Session::new(&workspace),
    )
    .await
    .unwrap();
    assert_eq!(result.tool_calls, 1);
    fs::write(root.path().join("once.txt"), "manual change after run").unwrap();
    let id = result.session.id;
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    store
        .save(&SessionSnapshot::new(
            &workspace,
            0,
            vec![entry(result.session)],
        ))
        .unwrap();
    drop(store);
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    let mut snapshot = store.load().unwrap().unwrap();
    let session = snapshot.entries.remove(0).session;
    let result = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config("followup"),
        Cancellation::default(),
        None,
        session,
    )
    .await
    .unwrap();
    assert_eq!(result.session.id, id);
    assert_eq!(result.outcome, RunOutcome::Completed);
    assert_eq!(result.tool_calls, 0);
    assert_eq!(
        fs::read_to_string(root.path().join("once.txt")).unwrap(),
        "manual change after run"
    );
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests[2].to_string().contains("first chat"));
    assert!(requests[2].to_string().contains("first answer"));
}
#[tokio::test]
async fn incomplete_tool_batch_survives_storage_but_is_never_resumed() {
    let root = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let mut session = state(&workspace, "first");
    let response=astrid::openai::completed_response(json!({"status":"completed","output":[{"type":"function_call","call_id":"a","name":"read_file","arguments":"{\"path\":\"x\"}"},{"type":"function_call","call_id":"b","name":"write_file","arguments":"{\"path\":\"x\",\"content\":\"should not run\",\"overwrite\":false}"}]})).unwrap();
    let model = Default::default();
    session.messages.push(Message::Assistant(response));
    session.context.add(
        ContextOrigin::Assistant {
            model_call_id: model,
        },
        Some(1),
    );
    session.messages.push(Message::Tool(ToolResult {
        call_id: "a".into(),
        name: "read_file".into(),
        outcome: ToolOutcome::Error {
            code: "missing".into(),
            message: "missing".into(),
        },
    }));
    session.context.add(
        ContextOrigin::ToolResult {
            model_call_id: model,
            tool_call_id: Default::default(),
            requested_path: Some("x".into()),
            path_truncated: false,
        },
        Some(2),
    );
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    store
        .save(&SessionSnapshot::new(
            &workspace,
            0,
            vec![StoredSession {
                session,
                runs: 1,
                outcome: Some(RunOutcome::Cancelled),
            }],
        ))
        .unwrap();
    let mut snapshot = store.load().unwrap().unwrap();
    let session = snapshot.entries.remove(0).session;
    let server = Server::start(vec![]).await;
    let tools = Tools::new(workspace, Duration::from_secs(2)).unwrap();
    assert!(
        runtime::run_in_session(
            &server.provider,
            &tools,
            &mut Confirmation::new(false),
            config("continue"),
            Cancellation::default(),
            None,
            session
        )
        .await
        .is_err()
    );
    assert!(server.requests.lock().unwrap().is_empty());
    assert!(!root.path().join("x").exists());
}

#[test]
fn stored_response_cannot_bypass_provider_completion_validation() {
    let response = astrid::openai::completed_response(json!({"status":"completed", "output":[{"type":"message", "role":"assistant", "content":[{"type":"output_text", "text":"original"}]}]})).unwrap();
    let mut value = serde_json::to_value(&response).unwrap();
    value["text"] = json!("tampered");
    assert!(serde_json::from_value::<astrid::model::ModelResponse>(value).is_err());
    let invalid = json!({"text":"", "tool_calls":[], "continuation":[{"type":"message", "role":"user", "content":[]}]});
    assert!(serde_json::from_value::<astrid::model::ModelResponse>(invalid).is_err());
}

fn private_reply() -> Vec<serde_json::Value> {
    let mut events = reply(&[], "visible answer");
    events.last_mut().unwrap()["response"]["output"].as_array_mut().unwrap().insert(0,
        json!({"type":"reasoning","id":"opaque-history","encrypted_content":"PERSISTENT-PRIVATE-SENTINEL","summary":[]}));
    events
}

#[tokio::test]
async fn known_native_model_change_persists_visible_mode_across_restart_without_private_replay() {
    use astrid::routing::{ReplayMode, ReplayReason};
    let root = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let tools = Tools::new(workspace.clone(), Duration::from_secs(5)).unwrap();
    let server = Server::start(vec![
        private_reply(),
        reply(&[], "changed model"),
        reply(&[], "back to initial"),
    ])
    .await;
    let mut handler = Confirmation::new(false);
    let first = runtime::run(
        &server.provider,
        &tools,
        &mut handler,
        config("first"),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        first.session.replay_mode(),
        ReplayMode::ProviderContinuation
    );
    assert_eq!(first.session.dispatch_origins().len(), 1);
    let original = serde_json::to_string(&first.session.messages).unwrap();
    let mut next = config("change model");
    next.model = "different-model".into();
    let second = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut handler,
        next,
        Cancellation::default(),
        None,
        first.session,
    )
    .await
    .unwrap();
    assert_eq!(second.session.replay_mode(), ReplayMode::VisibleHistory);
    assert!(
        second.state.models.values().all(
            |m| m.route.as_ref().unwrap().replay_reason == ReplayReason::ModelOrProviderChanged
        )
    );
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    store
        .save(&SessionSnapshot::new(
            &workspace,
            0,
            vec![StoredSession {
                session: second.session,
                runs: 2,
                outcome: Some(RunOutcome::Completed),
            }],
        ))
        .unwrap();
    drop(store);
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    let restored = store.load().unwrap().unwrap().entries.remove(0).session;
    assert_eq!(restored.dispatch_origins().len(), 2);
    assert_eq!(
        serde_json::to_string(&restored.messages[..2]).unwrap(),
        original
    );
    let third = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut handler,
        config("initial model again, routing off"),
        Cancellation::default(),
        None,
        restored,
    )
    .await
    .unwrap();
    assert_eq!(third.session.replay_mode(), ReplayMode::VisibleHistory);
    assert!(third.state.models.values().all(|m| m.route.as_ref().unwrap().replay_reason == ReplayReason::SessionVisibleHistory));
    let requests = server.finish().await;
    assert!(
        requests[1..]
            .iter()
            .all(|r| !r.to_string().contains("PERSISTENT-PRIVATE-SENTINEL"))
    );
    assert!(
        serde_json::to_string(&third.session.messages)
            .unwrap()
            .contains("PERSISTENT-PRIVATE-SENTINEL")
    );
}

#[tokio::test]
async fn v1_migration_is_read_only_until_successful_v2_save_and_unknown_history_is_visible() {
    use astrid::routing::{ReplayMode, ReplayReason};
    let root = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let tools = Tools::new(workspace.clone(), Duration::from_secs(5)).unwrap();
    let server = Server::start(vec![private_reply(), reply(&[], "migrated followup")]).await;
    let mut handler = Confirmation::new(false);
    let first = runtime::run(
        &server.provider,
        &tools,
        &mut handler,
        config("old task"),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    store
        .save(&SessionSnapshot::new(
            &workspace,
            0,
            vec![entry(first.session)],
        ))
        .unwrap();
    let path = state_path(directory.path());
    let mut legacy: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    legacy["version"] = json!(1);
    for entry in legacy["entries"].as_array_mut().unwrap() {
        entry["session"].as_object_mut().unwrap().remove("origins");
        entry["session"]
            .as_object_mut()
            .unwrap()
            .remove("replay_mode");
    }
    let bytes = serde_json::to_vec(&legacy).unwrap();
    fs::write(&path, &bytes).unwrap();
    let loaded = store.load().unwrap().unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        loaded.entries[0].session.replay_mode(),
        ReplayMode::VisibleHistory
    );
    assert!(loaded.entries[0].session.dispatch_origins().is_empty());
    let bad = SessionSnapshot::new(
        &workspace,
        9,
        vec![entry(loaded.entries[0].session.clone())],
    );
    assert!(store.save(&bad).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let result = runtime::run_in_session(
        &server.provider,
        &tools,
        &mut handler,
        config("continue migrated"),
        Cancellation::default(),
        None,
        loaded.entries.into_iter().next().unwrap().session,
    )
    .await
    .unwrap();
    assert!(
        result
            .state
            .models
            .values()
            .all(|m| m.route.as_ref().unwrap().replay_reason == ReplayReason::UnverifiedHistory)
    );
    assert_eq!(result.session.dispatch_origins().len(), 1);
    store
        .save(&SessionSnapshot::new(
            &workspace,
            0,
            vec![StoredSession {
                session: result.session,
                runs: 2,
                outcome: Some(result.outcome),
            }],
        ))
        .unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["version"], 2);
    assert!(saved.to_string().contains("PERSISTENT-PRIVATE-SENTINEL"));
    assert!(
        !server.finish().await[1]
            .to_string()
            .contains("PERSISTENT-PRIVATE-SENTINEL")
    );
}

#[tokio::test]
async fn v2_rejects_partial_or_foreign_provenance_without_overwriting_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let tools = Tools::new(workspace.clone(), Duration::from_secs(5)).unwrap();
    let server = Server::start(vec![private_reply()]).await;
    let first = runtime::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config("task"),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    server.finish().await;
    let store = SessionStore::open(directory.path(), &workspace).unwrap();
    let snapshot = SessionSnapshot::new(&workspace, 0, vec![entry(first.session)]);
    store.save(&snapshot).unwrap();
    let path = state_path(directory.path());
    let original: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let mut partial = original.clone();
    partial["entries"][0]["session"]
        .as_object_mut()
        .unwrap()
        .remove("origins");
    let mut foreign = original.clone();
    foreign["entries"][0]["session"]["origins"]
        [astrid::events::ModelCallId::default().to_string()] =
        json!({"provider":"openai_codex_subscription","model":"fixture","effort":null});
    let mut legacy_smuggling = original.clone();
    legacy_smuggling["version"] = json!(1);
    let duplicate_version = serde_json::to_string(&original).unwrap().replacen(
        "\"version\":2",
        "\"version\":2,\"version\":2",
        1,
    );
    fs::write(&path, duplicate_version.as_bytes()).unwrap();
    assert!(store.load().is_err());
    assert!(store.save(&snapshot).is_err());
    assert_eq!(fs::read(&path).unwrap(), duplicate_version.as_bytes());
    for value in [partial, foreign, legacy_smuggling] {
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(&snapshot).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[tokio::test]
async fn unsupported_visible_handoff_returns_unchanged_session_without_model_access() {
    struct Unsupported;
    #[async_trait::async_trait]
    impl astrid::model::ModelProvider for Unsupported {
        async fn generate(
            &self,
            _: &astrid::model::ModelRequest<'_>,
            _: &mut dyn astrid::model::TextSink,
        ) -> Result<astrid::model::ModelResponse, astrid::model::ModelError> {
            panic!("required visible-history support must be checked before provider access")
        }
    }
    let root = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let tools = Tools::new(workspace, Duration::from_secs(5)).unwrap();
    let server = Server::start(vec![private_reply()]).await;
    let first = runtime::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        config("first"),
        Cancellation::default(),
        None,
    )
    .await
    .unwrap();
    server.finish().await;
    let before = serde_json::to_string(&first.session).unwrap();
    let rejected = runtime::run_in_session(
        &Unsupported,
        &tools,
        &mut Confirmation::new(false),
        config("different provider"),
        Cancellation::default(),
        None,
        first.session,
    )
    .await
    .unwrap_err();
    assert!(rejected.message.contains("visible-history"));
    assert_eq!(serde_json::to_string(&rejected.session).unwrap(), before);
}
