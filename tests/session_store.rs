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
