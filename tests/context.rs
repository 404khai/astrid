mod support;

use astrid::{
    context::{ContextOrigin, ContextSnapshot, ContextSource},
    events::{EventKind, ExecutionState},
    model::{Message, ModelProvider, ModelRequest, TextSink, ToolOutcome, ToolResult},
    openai::completed_response,
    tools::Tools,
    workspace::Workspace,
};
use async_trait::async_trait;
use serde_json::json;
use std::{io, time::Duration};
use support::{Confirmation, Recording, Server, call, item, reply};

#[derive(Default)]
struct Inspect {
    snapshots: Vec<ContextSnapshot>,
}
#[async_trait]
impl TextSink for Inspect {
    async fn request_prepared(&mut self, snapshot: ContextSnapshot) -> io::Result<()> {
        self.snapshots.push(snapshot);
        Ok(())
    }
    async fn delta(&mut self, _: &str) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn snapshot_matches_sent_unicode_json_and_keeps_opaque_continuation_private() {
    let server = Server::start(vec![reply(&[], "done")]).await;
    let private = "PRIVATE-CONTINUATION-SENTINEL".repeat(4096);
    let first = call("a", "read_file", json!({"path":"héllo"}));
    let second = call("b", "grep", json!({"pattern":"\\\"\n"}));
    let assistant = completed_response(json!({
        "status":"completed",
        "output":[
            {"type":"reasoning","encrypted_content":private},
            item(&first), item(&second), support::message("ok")
        ]
    }))
    .unwrap();
    let messages = vec![
        Message::User("find 🦀 and \"quotes\"\n".into()),
        Message::Assistant(assistant),
        Message::Tool(ToolResult {
            call_id: "a".into(),
            name: "read_file".into(),
            outcome: ToolOutcome::Success {
                data: json!({"text":"héllo\n\"🦀\"\\"}),
            },
        }),
        Message::Tool(ToolResult {
            call_id: "b".into(),
            name: "grep".into(),
            outcome: ToolOutcome::Error {
                code: "denied".into(),
                message: "no".into(),
            },
        }),
    ];
    let mut inspect = Inspect::default();
    server
        .provider
        .generate(
            &ModelRequest {
                model: "test-model",
                instructions: "system\n🦀",
                messages: &messages,
            },
            &mut inspect,
        )
        .await
        .unwrap();
    let actual_wire_bytes = server.request_bytes.lock().unwrap()[0];
    let requests = server.finish().await;
    let body = &requests[0];
    let snapshot = &inspect.snapshots[0];
    assert_eq!(snapshot.serialized_request_bytes, actual_wire_bytes);
    assert_eq!(
        snapshot.serialized_request_bytes,
        serde_json::to_vec(body).unwrap().len()
    );
    assert_eq!(
        snapshot
            .measurements
            .iter()
            .map(|m| m.serialized_bytes)
            .sum::<usize>(),
        snapshot.serialized_request_bytes
    );
    assert_eq!(snapshot.provider_input_tokens, None);
    assert_eq!(snapshot.measurements.len(), 7);
    let continuation = snapshot
        .measurements
        .iter()
        .find(|m| m.source == ContextSource::AssistantContinuation)
        .unwrap();
    assert_eq!(continuation.entries, 4);
    assert!(continuation.contains_opaque_data);
    assert!(continuation.serialized_bytes > private.len());
    let outcomes = snapshot
        .measurements
        .iter()
        .find(|m| m.source == ContextSource::ToolResults)
        .unwrap();
    assert_eq!(outcomes.entries, 2);
    let nonopaque = snapshot.serialized_request_bytes - continuation.serialized_bytes;
    assert_eq!(
        snapshot.non_opaque_json_size_token_heuristic,
        nonopaque.div_ceil(4)
    );
    assert!(snapshot.non_opaque_json_size_token_heuristic < private.len() / 4);
    let public = serde_json::to_string(snapshot).unwrap();
    assert!(!public.contains("PRIVATE-CONTINUATION-SENTINEL"));
    assert!(!public.contains("test-only-credential"));
    assert!(
        !snapshot
            .inspection_lines()
            .join("\n")
            .contains("PRIVATE-CONTINUATION-SENTINEL")
    );
    assert!(
        snapshot
            .inspection_lines()
            .join("\n")
            .contains("opaque token cost unknown")
    );
    // Accounting preserves all protocol records, including a denied outcome.
    assert_eq!(body["input"].as_array().unwrap().len(), 7);
}

#[tokio::test]
async fn runtime_context_snapshots_correlate_with_calls_and_replay_before_text() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("file"), "héllo\n\"🦀\"").unwrap();
    let read = call("read", "read_file", json!({"path":"file"}));
    let server = Server::start(vec![reply(&[read], "inspect"), reply(&[], "done")]).await;
    let tools = Tools::new(
        Workspace::new(root.path()).unwrap(),
        Duration::from_secs(30),
    )
    .unwrap();
    let mut recording = Recording::default();
    let result = support::run(
        &server.provider,
        &tools,
        &mut Confirmation::new(false),
        &mut recording,
        "test-model",
        "read file",
        4,
    )
    .await
    .unwrap();
    let requests = server.finish().await;
    let mut replay = ExecutionState::new(result.session.id, result.state.run_id);
    let mut snapshots = 0;
    for event in &recording.events {
        if let EventKind::ContextItemAdded { item } = &event.kind {
            let mut forged = event.clone();
            let mut incorrect = item.clone();
            incorrect.message_index = Some(usize::MAX);
            forged.kind = EventKind::ContextItemAdded { item: incorrect };
            let before = replay.clone();
            assert!(replay.transition(&forged).is_err());
            assert_eq!(replay, before);
        }
        if let EventKind::ContextPrepared { snapshot } = &event.kind {
            let id = event.model_call_id.unwrap();
            assert!(!replay.models[&id].first_text);
            assert!(replay.models[&id].context.is_none());
            assert_eq!(
                snapshot.serialized_request_bytes,
                serde_json::to_vec(&requests[snapshots]).unwrap().len()
            );
            replay.transition(event).unwrap();
            assert_eq!(replay.models[&id].context.as_ref(), Some(snapshot));
            let mut duplicate = event.clone();
            duplicate.sequence += 1;
            let unchanged = replay.clone();
            assert!(replay.transition(&duplicate).is_err());
            assert_eq!(replay, unchanged);
            snapshots += 1;
        } else {
            replay.transition(event).unwrap();
        }
    }
    assert_eq!(snapshots, 2);
    assert_eq!(replay, result.state);
    assert_eq!(result.session.messages.len(), 4);
    assert_eq!(result.session.context.items, result.state.context_items);
    let message_items = result
        .session
        .context
        .items
        .iter()
        .filter(|item| item.message_index.is_some())
        .collect::<Vec<_>>();
    assert_eq!(message_items.len(), result.session.messages.len());
    for (index, item) in message_items.iter().enumerate() {
        assert_eq!(item.message_index, Some(index));
        assert!(!item.added_reason.is_empty());
    }
    let file_item = message_items
        .iter()
        .find(|item| matches!(item.origin, ContextOrigin::ToolResult { .. }))
        .unwrap();
    assert!(
        matches!(&file_item.origin, ContextOrigin::ToolResult { requested_path: Some(path), path_truncated: false, .. } if path == "file")
    );
    let public = serde_json::to_string(&result.session.context).unwrap();
    assert!(!public.contains("héllo"));
    assert!(!public.contains("🦀"));
}

#[test]
fn file_provenance_labels_truncated_paths_without_splitting_unicode() {
    let path = "🦀".repeat(400);
    let call = call("read", "read_file", json!({"path": path}));
    let (retained, truncated) = astrid::context::requested_path(&call);
    assert!(truncated);
    assert_eq!(retained.unwrap().len(), 1024);
}
