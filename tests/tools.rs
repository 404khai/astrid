mod support;
use astrid::{model::ToolOutcome, workspace::Workspace};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::symlink,
    time::{Duration, Instant},
};
use support::CheckedTools as Tools;
use support::{Confirmation, call};

fn tools(root: &std::path::Path) -> Tools {
    Tools::new(Workspace::new(root).unwrap(), Duration::from_secs(5)).unwrap()
}
fn code(outcome: &ToolOutcome) -> &str {
    if let ToolOutcome::Error { code, .. } = outcome {
        code
    } else {
        panic!("expected failure: {outcome:?}")
    }
}
fn data(outcome: &ToolOutcome) -> &Value {
    if let ToolOutcome::Success { data } | ToolOutcome::TimedOut { data } = outcome {
        data
    } else {
        panic!("expected success: {outcome:?}")
    }
}

#[tokio::test]
async fn nested_workspace_cannot_read_search_or_mutate_parent_repository() {
    let repository = tempfile::tempdir().unwrap();
    let nested = repository.path().join("nested");
    fs::create_dir(&nested).unwrap();
    let nested = fs::canonicalize(nested).unwrap();
    fs::write(repository.path().join("user.txt"), "existing user work").unwrap();
    fs::write(nested.join("local.txt"), "local").unwrap();
    let executor = tools(&nested);
    let mut confirmation = Confirmation::new(false);
    for (name, arguments) in [
        ("read_file", json!({"path":"../user.txt"})),
        ("grep", json!({"path":"..","pattern":"user"})),
        ("list_directory", json!({"path":".."})),
        ("glob", json!({"pattern":"../**"})),
        (
            "edit_file",
            json!({"path":"../user.txt","old_text":"existing","new_text":"changed"}),
        ),
        (
            "write_file",
            json!({"path":"../new.txt","content":"changed","overwrite":false}),
        ),
    ] {
        let result = executor
            .execute(&call(name, name, arguments), &mut confirmation)
            .await;
        assert_eq!(code(&result.outcome), "path_denied", "{name}");
    }
    let result = executor
        .execute(
            &call(
                "read-local",
                "read_file",
                json!({"path": nested.join("local.txt").display().to_string()}),
            ),
            &mut confirmation,
        )
        .await;
    assert_eq!(data(&result.outcome)["content"], "local");
    assert_eq!(
        fs::read_to_string(repository.path().join("user.txt")).unwrap(),
        "existing user work"
    );
    assert!(!repository.path().join("new.txt").exists());
    assert!(confirmation.commands.is_empty());
}

#[tokio::test]
async fn exact_edits_fail_without_mutation_for_zero_multiple_and_overlapping_matches() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let mut confirm = Confirmation::new(false);
    fs::write(root.path().join("file"), "aaa café café").unwrap();
    for (target, count) in [("missing", 0), ("café", 2), ("aa", 2)] {
        let result = tools
            .execute(
                &call(
                    "edit",
                    "edit_file",
                    json!({"path":"file","old_text":target,"new_text":"replacement"}),
                ),
                &mut confirm,
            )
            .await;
        assert_eq!(code(&result.outcome), "edit_ambiguous");
        if let ToolOutcome::Error { message, .. } = result.outcome {
            assert!(message.contains(&format!("found {count}")));
        }
        assert_eq!(
            fs::read_to_string(root.path().join("file")).unwrap(),
            "aaa café café"
        );
    }
    let result = tools
        .execute(
            &call(
                "edit",
                "edit_file",
                json!({"path":"file","old_text":"aaa","new_text":"é"}),
            ),
            &mut confirm,
        )
        .await;
    assert!(!result.is_error());
    assert_eq!(
        fs::read_to_string(root.path().join("file")).unwrap(),
        "é café café"
    );
}

#[tokio::test]
async fn paths_deny_parent_absolute_symlink_dangling_symlink_and_hard_link_mutation() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "unchanged").unwrap();
    symlink(outside.path(), root.path().join("escape")).unwrap();
    symlink(outside.path().join("new"), root.path().join("dangling")).unwrap();
    fs::hard_link(outside.path().join("secret"), root.path().join("linked")).unwrap();
    let tools = tools(root.path());
    let mut confirmation = Confirmation::new(false);
    for path in [
        "../new".to_owned(),
        outside.path().join("new").display().to_string(),
        "escape/new".into(),
        "dangling".into(),
        "linked".into(),
    ] {
        let result = tools
            .execute(
                &call(
                    "write",
                    "write_file",
                    json!({"path":path,"content":"changed","overwrite":true}),
                ),
                &mut confirmation,
            )
            .await;
        assert_eq!(code(&result.outcome), "path_denied");
    }
    for path in ["escape/secret", "../secret"] {
        assert_eq!(
            code(
                &tools
                    .execute(
                        &call("read", "read_file", json!({"path":path})),
                        &mut confirmation
                    )
                    .await
                    .outcome
            ),
            "path_denied"
        );
    }
    assert_eq!(
        fs::read_to_string(outside.path().join("secret")).unwrap(),
        "unchanged"
    );
    assert!(!outside.path().join("new").exists());
}

#[tokio::test]
async fn writing_requires_explicit_overwrite_and_preserves_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let mut confirm = Confirmation::new(false);
    let created = tools
        .execute(
            &call(
                "new",
                "write_file",
                json!({"path":"file","content":"first","overwrite":false}),
            ),
            &mut confirm,
        )
        .await;
    assert!(!created.is_error());
    fs::set_permissions(root.path().join("file"), fs::Permissions::from_mode(0o755)).unwrap();
    let rejected = tools
        .execute(
            &call(
                "write",
                "write_file",
                json!({"path":"file","content":"second","overwrite":false}),
            ),
            &mut confirm,
        )
        .await;
    assert!(rejected.is_error());
    assert_eq!(
        fs::read_to_string(root.path().join("file")).unwrap(),
        "first"
    );
    let written = tools
        .execute(
            &call(
                "write",
                "write_file",
                json!({"path":"file","content":"second","overwrite":true}),
            ),
            &mut confirm,
        )
        .await;
    assert!(!written.is_error());
    assert_eq!(
        fs::metadata(root.path().join("file"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
}

#[tokio::test]
async fn searches_are_deterministic_and_do_not_follow_symlinks_or_git_metadata() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::create_dir(root.path().join(".git")).unwrap();
    fs::write(root.path().join("src/a.rs"), "one\nneedle\n").unwrap();
    fs::write(root.path().join("b.rs"), "needle\n").unwrap();
    fs::write(root.path().join(".git/private.rs"), "needle").unwrap();
    fs::write(root.path().join("binary"), b"needle\0").unwrap();
    fs::write(outside.path().join("outside.rs"), "needle").unwrap();
    symlink(outside.path(), root.path().join("escape")).unwrap();
    let tools = tools(root.path());
    let mut confirm = Confirmation::new(false);
    let result = tools
        .execute(
            &call("glob", "glob", json!({"pattern":"**/*.rs"})),
            &mut confirm,
        )
        .await;
    assert_eq!(data(&result.outcome)["files"], json!(["b.rs", "src/a.rs"]));
    let result = tools
        .execute(
            &call("grep", "grep", json!({"path":".","pattern":"needle"})),
            &mut confirm,
        )
        .await;
    assert_eq!(
        data(&result.outcome)["matches"],
        json!([{"path":"b.rs","line":1,"text":"needle"},{"path":"src/a.rs","line":2,"text":"needle"}])
    );
    assert!(
        tools
            .execute(
                &call("bad", "grep", json!({"path":".","pattern":"["})),
                &mut confirm
            )
            .await
            .is_error()
    );
}

#[tokio::test]
async fn rejected_shell_never_spawns_and_every_invocation_needs_confirmation() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let mut confirm = Confirmation::new(false);
    for id in ["first", "second"] {
        let result = tools
            .execute(
                &call(id, "shell", json!({"command":"touch forbidden"})),
                &mut confirm,
            )
            .await;
        assert_eq!(code(&result.outcome), "permission_denied");
    }
    assert_eq!(confirm.commands.len(), 2);
    assert!(!root.path().join("forbidden").exists());
}

#[tokio::test]
async fn shell_captures_streams_status_and_uses_fresh_workspace_state() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let mut confirm = Confirmation::new(true);
    let result = tools
        .execute(
            &call(
                "shell",
                "shell",
                json!({"command":"printf out; printf err >&2; exit 7"}),
            ),
            &mut confirm,
        )
        .await;
    let captured = data(&result.outcome);
    assert_eq!(captured["stdout"], "out");
    assert_eq!(captured["stderr"], "err");
    assert_eq!(captured["exit_code"], 7);
    assert_eq!(captured["timed_out"], false);
    assert_eq!(captured["output"]["stdout"]["observed_bytes"], 3);
    assert_eq!(captured["output"]["stderr"]["captured_bytes"], 3);
    assert_eq!(captured["output"]["stdout"]["complete"], true);
    tools
        .execute(
            &call(
                "cd",
                "shell",
                json!({"command":"cd /; export ASTRID_TEST_STATE=changed"}),
            ),
            &mut confirm,
        )
        .await;
    let result = tools
        .execute(
            &call(
                "fresh",
                "shell",
                json!({"command":"pwd; printf '%s' \"${ASTRID_TEST_STATE-unset}\""}),
            ),
            &mut confirm,
        )
        .await;
    assert_eq!(
        data(&result.outcome)["stdout"],
        format!("{}\nunset", tools.workspace().root().display())
    );
}

#[tokio::test]
async fn shell_timeout_stops_descendants_before_later_mutation() {
    let root = tempfile::tempdir().unwrap();
    let tools = Tools::new(
        Workspace::new(root.path()).unwrap(),
        Duration::from_millis(100),
    )
    .unwrap();
    let mut confirm = Confirmation::new(true);
    let start = Instant::now();
    let result = tools
        .execute(
            &call(
                "hang",
                "shell",
                json!({"command":"sleep 0.4; touch too_late"}),
            ),
            &mut confirm,
        )
        .await;
    assert_eq!(data(&result.outcome)["timed_out"], true);
    assert!(start.elapsed() < Duration::from_secs(2));
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!root.path().join("too_late").exists());
}

#[tokio::test]
async fn invalid_arguments_and_unknown_tools_are_recoverable() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let mut confirm = Confirmation::new(false);
    for requested in [
        call("bad", "unknown", json!({})),
        call("bad", "write_file", json!({"path":"file","content":"data"})),
        call("bad", "read_file", json!({"path":"file","extra":true})),
    ] {
        assert_eq!(
            code(&tools.execute(&requested, &mut confirm).await.outcome),
            "invalid_arguments"
        );
    }
    assert!(!root.path().join("file").exists());
}
