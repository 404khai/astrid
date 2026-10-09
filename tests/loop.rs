mod support;
use astrid::{
    model::{Message, ModelError, ModelProvider, ModelRequest, ToolOutcome},
    runtime::RunOutcome,
    tools::Tools,
    workspace::Workspace,
};
use serde_json::{Value, json};
use std::{fs, time::Duration};
use support::{Confirmation, Recording, RunError, Server, call, completed, item, reply};

fn tools(root: &std::path::Path) -> Tools {
    Tools::new(Workspace::new(root).unwrap(), Duration::from_secs(30)).unwrap()
}

#[tokio::test]
async fn fragmented_tool_calls_wait_for_model_completion_and_execute_sequentially() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let write = call(
        "write",
        "write_file",
        json!({"path":"file","content":"héllo","overwrite":false}),
    );
    let read = call("read", "read_file", json!({"path":"file"}));
    let server = Server::start(vec![
        reply(&[write, read], "Inspecting."),
        reply(&[], "Finished."),
    ])
    .await;
    let mut observation = Recording::default();
    let result = support::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        &mut observation,
        "test-model",
        "task",
        20,
    )
    .await
    .unwrap();
    assert_eq!(result.final_text, "Finished.");
    assert_eq!(result.model_calls, 2);
    assert_eq!(result.tool_calls, 2);
    assert_eq!(
        fs::read_to_string(root.path().join("file")).unwrap(),
        "héllo"
    );
    assert_eq!(
        observation
            .observations
            .iter()
            .filter(|v| !v.starts_with("text"))
            .cloned()
            .collect::<Vec<_>>(),
        [
            "model 1 started",
            "model 1 completed",
            "tool write started",
            "tool write completed",
            "tool read started",
            "tool read completed",
            "model 2 started",
            "model 2 completed"
        ]
    );
    let requests = server.finish().await;
    assert_eq!(requests[0]["model"], "test-model");
    assert_eq!(requests[0]["store"], false);
    assert_eq!(requests[0]["stream"], true);
    assert_eq!(requests[0]["tools"][0]["type"], "namespace");
    assert_eq!(
        requests[0]["tools"][0]["tools"].as_array().unwrap().len(),
        7
    );
    let results = requests[1]["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["type"] == "function_call_output")
        .collect::<Vec<_>>();
    assert_eq!(results[0]["call_id"], "write");
    assert_eq!(results[1]["call_id"], "read");
    let result: Value = serde_json::from_str(results[1]["output"].as_str().unwrap()).unwrap();
    assert_eq!(result["data"]["content"], "héllo");
}

#[tokio::test]
async fn completed_tool_item_in_interrupted_response_never_mutates() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let write = call(
        "write",
        "write_file",
        json!({"path":"forbidden","content":"never","overwrite":false}),
    );
    let mut events = reply(&[write], "Partial text.");
    events.pop();
    let server = Server::start(vec![events]).await;
    let mut observation = Recording::default();
    let error = support::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        &mut observation,
        "test",
        "task",
        20,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        RunError::Runtime(RunOutcome::Failed { .. })
    ));
    assert_eq!(observation.text, "Partial text.");
    assert!(observation.results.is_empty());
    assert!(!root.path().join("forbidden").exists());
    assert_eq!(server.finish().await.len(), 1);
}

#[tokio::test]
async fn failed_incomplete_and_malformed_responses_never_dispatch_tools() {
    for event in [
        json!({"type":"response.failed","response":{"error":{"message":"failed"}}}),
        json!({"type":"response.incomplete","response":{"incomplete_details":{"reason":"max_output_tokens"}}}),
        json!({"type":"response.completed","response":{"status":"incomplete","output":[]}}),
    ] {
        let root = tempfile::tempdir().unwrap();
        let tools = tools(root.path());
        let write = call(
            "write",
            "write_file",
            json!({"path":"forbidden","content":"never","overwrite":false}),
        );
        let mut events = reply(&[write], "");
        events.pop();
        events.push(event);
        let server = Server::start(vec![events]).await;
        let mut observation = Recording::default();
        assert!(
            support::run(
                &server.provider,
                &tools,
                &mut Confirmation::new(false),
                &mut observation,
                "test",
                "task",
                20
            )
            .await
            .is_err()
        );
        assert!(observation.results.is_empty());
        assert!(!root.path().join("forbidden").exists());
        server.finish().await;
    }
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let good = call(
        "good",
        "write_file",
        json!({"path":"forbidden","content":"never","overwrite":false}),
    );
    let server = Server::start(vec![vec![completed(vec![
        item(&good),
        json!({"type":"function_call","call_id":"broken"}),
    ])]])
    .await;
    assert!(
        support::run(
            &server.provider,
            &tools,
            &mut Confirmation::new(false),
            &mut Recording::default(),
            "test",
            "task",
            20
        )
        .await
        .is_err()
    );
    assert!(!root.path().join("forbidden").exists());
    server.finish().await;
}

#[tokio::test]
async fn recoverable_errors_and_shell_denial_are_returned_to_the_model() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let server = Server::start(vec![
        reply(
            &[
                call("missing", "read_file", json!({"path":"missing"})),
                call("denied", "shell", json!({"command":"touch forbidden"})),
                call(
                    "invalid",
                    "edit_file",
                    json!({"path":"missing","old_text":"","new_text":"new"}),
                ),
            ],
            "",
        ),
        reply(&[], "I could not execute the command."),
    ])
    .await;
    let mut confirmation = Confirmation::new(false);
    let mut observation = Recording::default();
    support::run(
        &server.provider,
        &tools,
        &mut confirmation,
        &mut observation,
        "test",
        "task",
        20,
    )
    .await
    .unwrap();
    assert_eq!(confirmation.commands, ["touch forbidden"]);
    assert!(!root.path().join("forbidden").exists());
    let requests = server.finish().await;
    let errors = requests[1]["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["type"] == "function_call_output")
        .map(|item| serde_json::from_str::<Value>(item["output"].as_str().unwrap()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        errors
            .iter()
            .map(|v| v["code"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["io_error", "permission_denied", "invalid_arguments"]
    );
}

#[tokio::test]
async fn ceiling_allows_exactly_twenty_calls_and_reports_incomplete_task() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let replies = (1..=20)
        .map(|n| {
            reply(
                &[call(
                    &format!("read_{n}"),
                    "read_file",
                    json!({"path":"missing"}),
                )],
                "",
            )
        })
        .collect();
    let server = Server::start(replies).await;
    let mut observation = Recording::default();
    assert!(matches!(
        support::run(
            &server.provider,
            &tools,
            &mut Confirmation::new(false),
            &mut observation,
            "test",
            "task",
            20
        )
        .await,
        Err(RunError::Runtime(RunOutcome::ModelCallLimitReached {
            limit: 20,
            ..
        }))
    ));
    assert_eq!(observation.results.len(), 20);
    assert_eq!(server.finish().await.len(), 20);
}

#[tokio::test]
async fn provider_http_failure_is_terminal_and_is_not_retried() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let server = Server::http(429, vec![vec![]]).await;
    assert!(matches!(
        support::run(
            &server.provider,
            &tools,
            &mut Confirmation::new(false),
            &mut Recording::default(),
            "test",
            "task",
            20
        )
        .await,
        Err(RunError::Runtime(RunOutcome::Failed { message,.. })) if message.contains("HTTP 429")
    ));
    assert_eq!(server.finish().await.len(), 1);
}

#[tokio::test]
async fn repeated_call_id_cannot_replay_a_mutation() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let server = Server::start(vec![
        reply(
            &[call(
                "same",
                "write_file",
                json!({"path":"first","content":"first","overwrite":false}),
            )],
            "",
        ),
        reply(
            &[call(
                "same",
                "write_file",
                json!({"path":"second","content":"second","overwrite":false}),
            )],
            "",
        ),
    ])
    .await;
    assert!(
        support::run(
            &server.provider,
            &tools,
            &mut Confirmation::new(false),
            &mut Recording::default(),
            "test",
            "task",
            20
        )
        .await
        .is_err()
    );
    assert!(root.path().join("first").exists());
    assert!(!root.path().join("second").exists());
    server.finish().await;
}

#[tokio::test]
async fn only_workspace_root_instructions_are_loaded_and_reasoning_is_replayed() {
    let parent = tempfile::tempdir().unwrap();
    fs::create_dir(parent.path().join("root")).unwrap();
    fs::create_dir(parent.path().join("root/nested")).unwrap();
    fs::write(parent.path().join("AGENTS.md"), "PARENT_MARKER").unwrap();
    fs::write(parent.path().join("root/AGENTS.md"), "ROOT_MARKER").unwrap();
    fs::write(parent.path().join("root/nested/AGENTS.md"), "NESTED_MARKER").unwrap();
    let tools = tools(&parent.path().join("root"));
    let reasoning =
        json!({"type":"reasoning","id":"rs_test","summary":[],"encrypted_content":"opaque"});
    let read = call("read", "read_file", json!({"path":"AGENTS.md"}));
    let server = Server::start(vec![
        vec![completed(vec![reasoning.clone(), item(&read)])],
        reply(&[], "Done"),
    ])
    .await;
    support::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        &mut Recording::default(),
        "test",
        "task",
        20,
    )
    .await
    .unwrap();
    let requests = server.finish().await;
    let instructions = requests[0]["instructions"].as_str().unwrap();
    assert!(instructions.contains("ROOT_MARKER"));
    assert!(!instructions.contains("PARENT_MARKER"));
    assert!(!instructions.contains("NESTED_MARKER"));
    assert!(
        requests[1]["input"]
            .as_array()
            .unwrap()
            .contains(&reasoning)
    );
}

#[tokio::test]
async fn fragmented_arguments_that_disagree_with_completion_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let write = call(
        "write",
        "write_file",
        json!({"path":"forbidden","content":"never","overwrite":false}),
    );
    let mut events = reply(&[write], "");
    events
        .iter_mut()
        .find(|v| v["type"] == "response.function_call_arguments.delta")
        .unwrap()["delta"] = json!("corrupt");
    let server = Server::start(vec![events]).await;
    assert!(
        support::run(
            &server.provider,
            &tools,
            &mut Confirmation::new(false),
            &mut Recording::default(),
            "test",
            "task",
            20
        )
        .await
        .is_err()
    );
    assert!(!root.path().join("forbidden").exists());
    server.finish().await;
}

#[tokio::test]
async fn deterministic_acceptance_fixture_inspects_edits_creates_tests_and_finishes() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        include_str!("fixtures/greeting/Cargo.toml"),
    )
    .unwrap();
    fs::write(
        root.path().join("src/lib.rs"),
        include_str!("fixtures/greeting/src/lib.rs"),
    )
    .unwrap();
    fs::write(
        root.path().join("AGENTS.md"),
        include_str!("fixtures/greeting/AGENTS.md"),
    )
    .unwrap();
    let tools = tools(root.path());
    let server=Server::start(vec![
        reply(&[call("list","list_directory",json!({"path":"."})),call("glob","glob",json!({"pattern":"**/*.rs"})),
            call("grep","grep",json!({"path":"src","pattern":"greeting"})),call("read","read_file",json!({"path":"src/lib.rs"})),
            call("failing_tests","shell",json!({"command":"cargo test --offline"}))],"Inspecting the fixture."),
        reply(&[call("edit","edit_file",json!({"path":"src/lib.rs","old_text":"format!(\"Hello, {}!\", name)","new_text":"format!(\"Hello, {}!\", name.trim())"})),
            call("create","write_file",json!({"path":"GREETING.txt","content":"Hello, Astrid!\n","overwrite":false}))],"Applying the two required changes."),
        reply(&[call("passing_tests","shell",json!({"command":"cargo test --offline"}))],"Verifying the changes."),
        reply(&[],"Both tests pass. Greeting trims whitespace and GREETING.txt was created."),
    ]).await;
    let mut observations = Recording::default();
    let mut confirmation = Confirmation::new(true);
    let result = support::run(
        &server.provider,
        &tools,
        &mut confirmation,
        &mut observations,
        "fixture-model",
        "Fix the fixture and create the required greeting file.",
        20,
    )
    .await
    .unwrap();
    assert_eq!(result.model_calls, 4);
    assert_eq!(result.tool_calls, 8);
    assert_eq!(confirmation.commands.len(), 2);
    let failing = observations
        .results
        .iter()
        .find(|r| r.call_id == "failing_tests")
        .unwrap();
    let passing = observations
        .results
        .iter()
        .find(|r| r.call_id == "passing_tests")
        .unwrap();
    if let ToolOutcome::Success { data } = &failing.outcome {
        assert_eq!(data["exit_code"], 101);
        assert!(data["stdout"].as_str().unwrap().contains("FAILED"));
    } else {
        panic!("failing tests did not execute")
    }
    if let ToolOutcome::Success { data } = &passing.outcome {
        assert_eq!(data["exit_code"], 0);
        assert!(data["stdout"].as_str().unwrap().contains("2 passed"));
    } else {
        panic!("passing tests did not execute")
    }
    assert_eq!(
        fs::read_to_string(root.path().join("GREETING.txt")).unwrap(),
        "Hello, Astrid!\n"
    );
    assert!(
        fs::read_to_string(root.path().join("src/lib.rs"))
            .unwrap()
            .contains("name.trim()")
    );
    let requests = server.finish().await;
    let preceding = requests[3]["input"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["type"] == "function_call_output" && v["call_id"] == "passing_tests")
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(preceding["output"].as_str().unwrap()).unwrap()["data"]["exit_code"],
        0
    );
}

#[tokio::test]
async fn blank_task_and_zero_ceiling_fail_before_provider_access() {
    let root = tempfile::tempdir().unwrap();
    let tools = tools(root.path());
    let server = Server::start(vec![]).await;
    for (task, limit) in [("", 20), ("task", 0)] {
        assert!(matches!(
            support::run(
                &server.provider,
                &tools,
                &mut Confirmation::new(false),
                &mut Recording::default(),
                "model",
                task,
                limit
            )
            .await,
            Err(RunError::Configuration)
        ));
    }
    assert!(server.finish().await.is_empty());
}

#[tokio::test]
async fn text_sink_failure_prevents_tool_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let write = call(
        "write",
        "write_file",
        json!({"path":"forbidden","content":"never","overwrite":false}),
    );
    let server = Server::start(vec![reply(&[write], "text")]).await;
    let messages = [Message::User("task".into())];
    let request = ModelRequest {
        model: "test",
        instructions: "instructions",
        messages: &messages,
    };
    assert!(matches!(
        server
            .provider
            .generate(&request, &mut |_: &str| Err(std::io::Error::other(
                "broken pipe"
            )))
            .await,
        Err(ModelError::Output(_))
    ));
    assert!(!root.path().join("forbidden").exists());
    server.finish().await;
}

#[tokio::test]
async fn missing_content_type_still_requires_successful_completion() {
    for successful in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let write = call(
            "write",
            "write_file",
            json!({"path":"created","content":"ok","overwrite":false}),
        );
        let mut events = reply(&[write], "");
        if !successful {
            events.pop();
        }
        let mut replies = vec![events];
        if successful {
            replies.push(reply(&[], "done"));
        }
        let server = Server::with_content_type(200, replies, None).await;
        let result = support::run(
            &server.provider,
            &tools(root.path()),
            &mut Confirmation::new(false),
            &mut Recording::default(),
            "test",
            "task",
            20,
        )
        .await;
        assert_eq!(result.is_ok(), successful);
        assert_eq!(root.path().join("created").exists(), successful);
        server.finish().await;
    }
}

#[tokio::test]
async fn explicit_non_stream_content_type_fails_without_mutation() {
    let root = tempfile::tempdir().unwrap();
    let write = call(
        "write",
        "write_file",
        json!({"path":"created","content":"ok","overwrite":false}),
    );
    let server =
        Server::with_content_type(200, vec![reply(&[write], "")], Some("application/json")).await;
    let result = support::run(
        &server.provider,
        &tools(root.path()),
        &mut Confirmation::new(false),
        &mut Recording::default(),
        "test",
        "task",
        20,
    )
    .await;
    assert!(matches!(
        result,
        Err(RunError::Runtime(RunOutcome::Failed { .. }))
    ));
    assert!(!root.path().join("created").exists());
    server.finish().await;
}

#[tokio::test]
async fn empty_terminal_output_uses_completed_items_only_after_success() {
    for successful in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let write = call(
            "write",
            "write_file",
            json!({"path":"created","content":"ok","overwrite":false}),
        );
        let mut events = reply(&[write], "");
        events.pop();
        if successful {
            events.push(completed(vec![]));
        }
        let mut replies = vec![events];
        if successful {
            replies.push(vec![json!({"type":"response.output_text.delta","delta":"done"}),
                json!({"type":"response.output_item.done","output_index":0,"item":support::message("done")}), completed(vec![])]);
        }
        let server = Server::with_content_type(200, replies, None).await;
        let result = support::run(
            &server.provider,
            &tools(root.path()),
            &mut Confirmation::new(false),
            &mut Recording::default(),
            "test",
            "task",
            20,
        )
        .await;
        assert_eq!(result.is_ok(), successful);
        assert_eq!(root.path().join("created").exists(), successful);
        if successful {
            assert_eq!(result.unwrap().final_text, "done");
        }
        server.finish().await;
    }
}

#[tokio::test]
async fn targeted_inspection_controls_and_coverage_survive_provider_roundtrip() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("file.rs"), "first\nneedle\nlast\nneedle\n").unwrap();
    fs::write(root.path().join("noise.txt"), "needle\n").unwrap();
    let server = Server::start(vec![
        reply(&[
            call("search", "grep", json!({"path":".","pattern":"needle","include_glob":"**/*.rs","exclude_glob":null,"before_context":1,"after_context":1,"offset":0,"limit":1})),
            call("read", "read_file", json!({"path":"file.rs","start_line":2,"end_line":3})),
        ], "Inspecting targeted excerpts."),
        reply(&[], "Finished."),
    ]).await;
    let result = support::run(
        &server.provider,
        &tools(root.path()),
        &mut Confirmation::new(false),
        &mut Recording::default(),
        "test-model",
        "inspect",
        3,
    )
    .await
    .unwrap();
    assert_eq!(result.tool_calls, 2);
    let requests = server.finish().await;
    let definitions = requests[0]["tools"][0]["tools"].as_array().unwrap();
    let read = definitions
        .iter()
        .find(|definition| definition["name"] == "read_file")
        .unwrap();
    assert_eq!(
        read["parameters"]["properties"]["start_line"]["type"],
        json!(["integer", "null"])
    );
    let outputs: Vec<Value> = requests[1]["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["type"] == "function_call_output")
        .map(|item| serde_json::from_str(item["output"].as_str().unwrap()).unwrap())
        .collect();
    assert_eq!(outputs[0]["data"]["matches"][0]["path"], "file.rs");
    assert_eq!(
        outputs[0]["data"]["matches"][0]["before"],
        json!([{"line":1,"text":"first"}])
    );
    assert_eq!(outputs[0]["data"]["page"]["next_offset"], 1);
    assert_eq!(outputs[1]["data"]["content"], "needle\nlast\n");
    assert_eq!(
        outputs[1]["data"]["lines"],
        json!([{"line":2,"text":"needle"},{"line":3,"text":"last"}])
    );
    assert_eq!(outputs[1]["data"]["coverage"]["complete"], true);
}
