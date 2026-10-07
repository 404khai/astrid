use astrid::{
    cancellation::Cancellation,
    model::ToolCall,
    native::{OUTPUT_LIMIT, invoke},
    tools::ToolError,
    workspace::Workspace,
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::symlink};

fn run(root: &std::path::Path, name: &str, args: Value) -> Value {
    let call = ToolCall {
        call_id: "native-bound".into(),
        name: name.into(),
        arguments: args.to_string(),
    };
    let result = invoke(
        &Workspace::new(root).unwrap(),
        &call,
        &Cancellation::default(),
    )
    .unwrap();
    assert!(result.to_string().len() <= OUTPUT_LIMIT);
    result
}

#[test]
fn read_caps_raw_and_serialized_escaped_unicode_without_corruption() {
    let root = tempfile::tempdir().unwrap();
    for (name, text) in [
        ("huge", "é".repeat(100_000)),
        ("escaped", "\u{1}".repeat(100_000)),
    ] {
        fs::write(root.path().join(name), text).unwrap();
        let result = run(root.path(), "read_file", json!({"path":name}));
        assert_eq!(result["truncated"], true);
        assert_eq!(result["coverage"]["complete"], false);
        let text = result["content"].as_str().unwrap();
        assert!(text.len() <= OUTPUT_LIMIT);
        assert_eq!(result["coverage"]["retained_bytes"], text.len());
        assert!(!text.contains('\u{fffd}'));
    }
}

#[test]
fn small_read_and_search_keep_existing_data_shapes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("b"), "needle\r\n").unwrap();
    fs::write(root.path().join("a"), "needle\n").unwrap();
    let result = run(root.path(), "grep", json!({"path":".","pattern":"needle"}));
    assert_eq!(
        result["matches"],
        json!([
        {"path":"a","line":1,"text":"needle"}, {"path":"b","line":1,"text":"needle"}])
    );
    assert_eq!(result["coverage"]["complete"], true);
    assert_eq!(
        run(root.path(), "read_file", json!({"path":"a"}))["content"],
        "needle\n"
    );
}

#[test]
fn oversized_search_input_is_incomplete_rather_than_no_match_evidence() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("huge"), "x".repeat(2 * 1024 * 1024)).unwrap();
    fs::write(
        root.path().join("line"),
        format!("{}needle\nneedle\n", "x".repeat(50_000)),
    )
    .unwrap();
    let result = run(root.path(), "grep", json!({"path":".","pattern":"needle"}));
    assert_eq!(result["coverage"]["complete"], false);
    assert_eq!(result["coverage"]["oversized_files"], 1);
    assert_eq!(result["coverage"]["oversized_lines"], 1);
    assert_eq!(
        result["matches"],
        json!([{"path":"line","line":2,"text":"needle"}])
    );
    let result = run(
        root.path(),
        "grep",
        json!({"path":"huge","pattern":"needle"}),
    );
    assert_eq!(result["matches"], json!([]));
    assert_eq!(result["truncated"], true);
}

#[test]
fn match_flood_is_bounded_and_marks_remaining_search() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("many"), "needle\n".repeat(10_000)).unwrap();
    let result = run(root.path(), "grep", json!({"path":".","pattern":"needle"}));
    assert_eq!(result["truncated"], true);
    assert!(!result["matches"].as_array().unwrap().is_empty());
    assert!(result["matches"].as_array().unwrap().len() < 10_000);
    assert_eq!(result["coverage"]["remaining_files"], 1);
}

#[test]
fn bounded_inventories_are_identical_across_reverse_creation_order() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for index in 0..1300 {
        let name = format!("{index:04}-{}", "x".repeat(index % 200));
        fs::write(first.path().join(&name), "").unwrap();
    }
    for index in (0..1300).rev() {
        let name = format!("{index:04}-{}", "x".repeat(index % 200));
        fs::write(second.path().join(&name), "").unwrap();
    }
    for (tool, args, key) in [
        ("glob", json!({"pattern":"*"}), "files"),
        ("list_directory", json!({"path":"."}), "entries"),
    ] {
        let a = run(first.path(), tool, args.clone());
        let b = run(second.path(), tool, args);
        assert_eq!(a, b);
        assert_eq!(a["truncated"], true);
        assert!(a[key].as_array().unwrap().len() < 1300);
    }
}

#[test]
fn search_excludes_symlinks_git_and_binary_and_rejects_escaping_paths() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "needle").unwrap();
    symlink(outside.path(), root.path().join("escape")).unwrap();
    fs::create_dir(root.path().join(".git")).unwrap();
    fs::write(root.path().join(".git/private"), "needle").unwrap();
    fs::write(root.path().join("binary"), b"needle\0").unwrap();
    let result = run(root.path(), "grep", json!({"path":".","pattern":"needle"}));
    assert_eq!(result["matches"], json!([]));
    assert_eq!(result["coverage"]["excluded_binary_files"], 1);
    assert_eq!(result["coverage"]["complete"], true);
    let workspace = Workspace::new(root.path()).unwrap();
    for (name, args) in [
        ("read_file", json!({"path":"escape/secret"})),
        ("grep", json!({"path":"..","pattern":"x"})),
        ("glob", json!({"pattern":"../*"})),
    ] {
        let call = ToolCall {
            call_id: "bad".into(),
            name: name.into(),
            arguments: args.to_string(),
        };
        assert!(matches!(
            invoke(&workspace, &call, &Cancellation::default()),
            Err(ToolError::PathDenied(_))
        ));
    }
}

#[test]
fn pre_cancelled_inspection_does_no_io() {
    let root = tempfile::tempdir().unwrap();
    let cancel = Cancellation::default();
    cancel.cancel();
    let call = ToolCall {
        call_id: "cancel".into(),
        name: "read_file".into(),
        arguments: json!({"path":"missing"}).to_string(),
    };
    assert!(matches!(
        invoke(&Workspace::new(root.path()).unwrap(), &call, &cancel),
        Err(ToolError::Cancelled)
    ));
}
