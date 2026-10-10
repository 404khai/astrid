use astrid::{
    cancellation::Cancellation,
    model::ToolCall,
    native::{OUTPUT_LIMIT, invoke},
    tools::{ToolError, definitions},
    workspace::Workspace,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn inspect(root: &Path, name: &str, args: Value) -> Result<Value, ToolError> {
    let result = invoke(
        &Workspace::new(root).unwrap(),
        &ToolCall {
            call_id: "targeted".into(),
            name: name.into(),
            arguments: args.to_string(),
        },
        &Cancellation::default(),
    )?;
    assert!(result.to_string().len() <= OUTPUT_LIMIT);
    Ok(result)
}

#[test]
fn ranges_are_inclusive_numbered_and_preserve_raw_line_endings() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("file"), "first\r\n\r\ncafé\nlast").unwrap();
    let result = inspect(
        root.path(),
        "read_file",
        json!({"path":"file","start_line":2,"end_line":3}),
    )
    .unwrap();
    assert_eq!(result["content"], "\r\ncafé\n");
    assert_eq!(
        result["lines"],
        json!([{"line":2,"text":""},{"line":3,"text":"café"}])
    );
    assert_eq!(result["truncated"], false);
    assert_eq!(result["coverage"]["complete"], true);
    assert_eq!(result["coverage"]["requested_end_reached"], true);
    assert_eq!(result["coverage"]["reached_eof"], false);
    let result = inspect(
        root.path(),
        "read_file",
        json!({"path":"file","start_line":4,"end_line":9}),
    )
    .unwrap();
    assert_eq!(result["lines"], json!([{"line":4,"text":"last"}]));
    assert_eq!(result["coverage"]["reached_eof"], true);
    assert_eq!(result["coverage"]["complete"], true);
    let exact = inspect(
        root.path(),
        "read_file",
        json!({"path":"file","start_line":4,"end_line":4}),
    )
    .unwrap();
    assert_eq!(exact["coverage"]["requested_end_reached"], true);
    let result = inspect(
        root.path(),
        "read_file",
        json!({"path":"file","start_line":9}),
    )
    .unwrap();
    assert_eq!(result["content"], "");
    assert_eq!(result["lines"], json!([]));
    assert_eq!(result["coverage"]["range_start_reached"], false);
    assert_eq!(result["truncated"], false);
}

#[test]
fn distant_ranges_skip_large_prefix_without_returning_it() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("file"),
        format!("{}target\nend\n", "noise\n".repeat(40_000)),
    )
    .unwrap();
    let result = inspect(
        root.path(),
        "read_file",
        json!({"path":"file","start_line":40001,"end_line":40001}),
    )
    .unwrap();
    assert_eq!(result["content"], "target\n");
    assert_eq!(result["lines"], json!([{"line":40001,"text":"target"}]));
    assert_eq!(result["coverage"]["complete"], true);
    assert!(result.to_string().len() < 1024);
}

#[test]
fn read_limits_distinguish_output_from_unreached_scan_range() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("large"), "x".repeat(17 * 1024 * 1024)).unwrap();
    let result = inspect(
        root.path(),
        "read_file",
        json!({"path":"large","start_line":2}),
    )
    .unwrap();
    assert_eq!(result["content"], "");
    assert_eq!(result["coverage"]["truncation_reason"], "scan_limit");
    assert_eq!(result["coverage"]["range_start_reached"], false);
    assert_eq!(result["coverage"]["scanned_bytes"], 16 * 1024 * 1024 + 1);
    fs::write(root.path().join("unicode"), "🦀\u{1}".repeat(100_000)).unwrap();
    let result = inspect(
        root.path(),
        "read_file",
        json!({"path":"unicode","start_line":1,"end_line":1}),
    )
    .unwrap();
    assert_eq!(result["coverage"]["truncation_reason"], "output_limit");
    assert_eq!(result["coverage"]["last_line_partial"], true);
    assert!(!result["content"].as_str().unwrap().contains('\u{fffd}'));
    assert_eq!(
        result["coverage"]["retained_bytes"],
        result["content"].as_str().unwrap().len()
    );
    fs::write(root.path().join("many"), "\n".repeat(5000)).unwrap();
    let result = inspect(root.path(), "read_file", json!({"path":"many"})).unwrap();
    assert_eq!(result["lines"].as_array().unwrap().len(), 1024);
    assert_eq!(result["coverage"]["last_line_partial"], false);
    assert_eq!(result["truncated"], true);
}

#[test]
fn null_options_keep_old_calls_working_and_empty_files_have_no_lines() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("empty"), "").unwrap();
    let old = inspect(root.path(), "read_file", json!({"path":"empty"})).unwrap();
    let new = inspect(
        root.path(),
        "read_file",
        json!({"path":"empty","start_line":null,"end_line":null}),
    )
    .unwrap();
    assert_eq!(old, new);
    assert_eq!(old["lines"], json!([]));
    assert_eq!(old["truncated"], false);
    assert_eq!(inspect(root.path(), "grep", json!({"path":".","pattern":"x","limit":null,"offset":null,"before_context":null,"after_context":null,"include_glob":null,"exclude_glob":null})).unwrap()["matches"], json!([]));
}

#[test]
fn grep_filters_before_inventory_limits_and_returns_numbered_context() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    for index in 0..1100 {
        fs::write(root.path().join(format!("{index:04}.txt")), "needle\n").unwrap();
    }
    fs::write(
        root.path().join("src/a.rs"),
        "before\r\nneedle\nneedle again\nafter",
    )
    .unwrap();
    fs::write(root.path().join("src/skip.rs"), "needle").unwrap();
    let result = inspect(root.path(), "grep", json!({"path":".","pattern":"needle","include_glob":"**/*.rs","exclude_glob":"**/skip.rs","before_context":1,"after_context":1})).unwrap();
    assert_eq!(result["coverage"]["complete"], true);
    assert_eq!(
        result["matches"],
        json!([
            {"path":"src/a.rs","line":2,"text":"needle","before":[{"line":1,"text":"before"}],"after":[{"line":3,"text":"needle again"}],"context_truncated":false},
            {"path":"src/a.rs","line":3,"text":"needle again","before":[{"line":2,"text":"needle"}],"after":[{"line":4,"text":"after"}],"context_truncated":false}
        ])
    );
}

#[test]
fn pagination_has_no_duplicate_or_missing_matches_across_files() {
    let root = tempfile::tempdir().unwrap();
    for file in ["c", "b", "a"] {
        fs::write(root.path().join(file), "needle\nno\nneedle\nneedle\n").unwrap();
    }
    let mut all = Vec::new();
    let mut offset = 0;
    loop {
        let result = inspect(
            root.path(),
            "grep",
            json!({"path":".","pattern":"needle","offset":offset,"limit":2}),
        )
        .unwrap();
        all.extend(result["matches"].as_array().unwrap().iter().cloned());
        if let Some(next) = result["page"]["next_offset"].as_u64() {
            assert!(next > offset);
            assert_eq!(result["page"]["stop_reason"], "match_limit");
            assert_eq!(result["truncated"], true);
            offset = next;
        } else {
            assert_eq!(result["coverage"]["complete"], true);
            break;
        }
    }
    let expected: Vec<_> = ["a", "b", "c"]
        .iter()
        .flat_map(|path| [1, 3, 4].map(|line| json!({"path":path,"line":line,"text":"needle"})))
        .collect();
    assert_eq!(all, expected);
    let beyond = inspect(
        root.path(),
        "grep",
        json!({"path":".","pattern":"needle","offset":100}),
    )
    .unwrap();
    assert_eq!(beyond["matches"], json!([]));
    assert_eq!(beyond["page"]["next_offset"], Value::Null);
}

#[test]
fn byte_limited_pages_and_oversized_context_report_truthful_coverage() {
    let root = tempfile::tempdir().unwrap();
    let line = format!("needle{}\n", "x".repeat(10_000));
    fs::write(root.path().join("file"), line.repeat(8)).unwrap();
    let mut offset = 0;
    let mut count = 0;
    loop {
        let result = inspect(
            root.path(),
            "grep",
            json!({"path":"file","pattern":"needle","limit":100,"offset":offset}),
        )
        .unwrap();
        count += result["matches"].as_array().unwrap().len();
        if let Some(next) = result["page"]["next_offset"].as_u64() {
            assert_eq!(result["page"]["stop_reason"], "output_limit");
            assert!(next > offset);
            offset = next;
        } else {
            break;
        }
    }
    assert_eq!(count, 8);
    fs::write(
        root.path().join("file"),
        format!("{}\nneedle\nafter\n", "x".repeat(50_000)),
    )
    .unwrap();
    let result = inspect(
        root.path(),
        "grep",
        json!({"path":"file","pattern":"needle","before_context":1,"after_context":1}),
    )
    .unwrap();
    assert_eq!(result["matches"][0]["context_truncated"], true);
    assert_eq!(result["matches"][0]["before"], json!([]));
    assert_eq!(
        result["matches"][0]["after"],
        json!([{"line":3,"text":"after"}])
    );
    assert_eq!(result["coverage"]["oversized_lines"], 1);
    assert_eq!(result["page"]["next_offset"], Value::Null);
    assert_eq!(result["coverage"]["complete"], false);
}

#[test]
fn exact_final_page_needs_no_empty_followup_and_oversized_records_are_explicit() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("file"), "needle\nneedle\n").unwrap();
    let result = inspect(
        root.path(),
        "grep",
        json!({"path":"file","pattern":"needle","limit":2}),
    )
    .unwrap();
    assert_eq!(result["page"]["next_offset"], Value::Null);
    assert_eq!(result["coverage"]["complete"], true);
    fs::write(
        root.path().join("file"),
        format!("{}\nneedle\n", "\u{1}".repeat(16_000)),
    )
    .unwrap();
    let result = inspect(root.path(), "grep", json!({"path":"file","pattern":"."})).unwrap();
    assert_eq!(result["coverage"]["oversized_matches"], 1);
    assert_eq!(result["coverage"]["complete"], false);
    assert_eq!(
        result["matches"],
        json!([{"path":"file","line":2,"text":"needle"}])
    );
}

#[test]
fn invalid_options_are_recoverable_and_filters_cannot_escape_workspace() {
    let root = tempfile::tempdir().unwrap();
    for (name, args) in [
        ("read_file", json!({"path":"missing","start_line":0})),
        (
            "read_file",
            json!({"path":"missing","start_line":4,"end_line":3}),
        ),
        ("read_file", json!({"path":"missing","start_line":-1})),
        ("grep", json!({"path":".","pattern":"x","limit":0})),
        ("grep", json!({"path":".","pattern":"x","limit":1025})),
        (
            "grep",
            json!({"path":".","pattern":"x","before_context":21}),
        ),
        ("grep", json!({"path":".","pattern":"x","after_context":21})),
        ("grep", json!({"path":".","pattern":"x","offset":-1})),
        ("grep", json!({"path":".","pattern":"x","include_glob":"["})),
    ] {
        assert!(
            matches!(
                inspect(root.path(), name, args),
                Err(ToolError::InvalidArguments(_))
            ),
            "{name}"
        );
    }
    assert!(matches!(
        inspect(
            root.path(),
            "grep",
            json!({"path":".","pattern":"x","include_glob":"../*"})
        ),
        Err(ToolError::PathDenied(_))
    ));
}

#[test]
fn provider_schema_exposes_nullable_controls_in_strict_required_properties() {
    for definition in definitions() {
        let properties = definition["parameters"]["properties"].as_object().unwrap();
        let required = definition["parameters"]["required"].as_array().unwrap();
        assert_eq!(properties.len(), required.len());
        if definition["name"] == "read_file" {
            assert_eq!(properties["start_line"]["type"], json!(["integer", "null"]));
        }
        if definition["name"] == "grep" {
            assert_eq!(
                properties["include_glob"]["type"],
                json!(["string", "null"])
            );
            assert_eq!(properties["limit"]["maximum"], 1024);
        }
    }
}
