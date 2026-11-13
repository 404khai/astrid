use super::*;
use crate::model::{ToolOutcome, ToolResult};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn key() -> ApiKey {
    ApiKey::new("sk-ant-test-0000000000000000".into()).unwrap()
}

fn sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|e| format!("event: {}\ndata: {}\n\n", e["type"].as_str().unwrap(), e))
        .collect()
}

/// One-shot mock: records the request (headers + JSON body), replies with `reply`.
async fn mock(
    status: u16,
    content_type: &'static str,
    reply: String,
) -> (String, tokio::task::JoinHandle<(String, Value)>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/v1/messages", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut raw = vec![];
        let mut b = [0u8; 8192];
        let (head, body) = loop {
            let n = socket.read(&mut b).await.unwrap();
            raw.extend_from_slice(&b[..n]);
            let text = String::from_utf8_lossy(&raw).into_owned();
            if let Some(i) = text.find("\r\n\r\n") {
                let len: usize = text
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap())
                    })
                    .unwrap_or(0);
                if raw.len() >= i + 4 + len {
                    break (
                        text[..i].to_owned(),
                        serde_json::from_slice::<Value>(&raw[i + 4..i + 4 + len]).unwrap(),
                    );
                }
            }
        };
        let wire = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
            reply.len()
        );
        socket.write_all(wire.as_bytes()).await.unwrap();
        (head, body)
    });
    (origin, task)
}

fn request<'a>(messages: &'a [Message], model: &'a str) -> ModelRequest<'a> {
    ModelRequest {
        options: Default::default(),
        model,
        instructions: "You are Astrid.",
        messages,
    }
}

fn turn_events() -> Vec<Value> {
    vec![
        json!({"type":"message_start","message":{"id":"msg_1","model":"claude-opus-5-5","usage":{"input_tokens":10}}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig-abc"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Let me "}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"look."}}),
        json!({"type":"content_block_stop","index":1}),
        json!({"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"toolu_1","name":"read_file","input":{}}}),
        json!({"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"READ"}}),
        json!({"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"ME.md\",\"start_line\":null,\"end_line\":null}"}}),
        json!({"type":"content_block_stop","index":2}),
        json!({"type":"ping"}),
        json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":30}}),
        json!({"type":"message_stop"}),
    ]
}

#[tokio::test]
async fn streams_text_and_tool_calls_and_keeps_thinking_for_replay() {
    let (url, server) = mock(200, "text/event-stream", sse(&turn_events())).await;
    let provider = ClaudeProvider::with_endpoint(key(), url).unwrap();
    let messages = [Message::User("Summarize the README".into())];
    let mut streamed = String::new();
    let mut sink = |d: &str| {
        streamed.push_str(d);
        Ok(())
    };
    let response = provider
        .generate(&request(&messages, "claude-opus-5-5"), &mut sink)
        .await
        .unwrap();
    assert_eq!(streamed, "Let me look.");
    assert_eq!(response.text, "Let me look.");
    assert_eq!(
        response.tool_calls,
        vec![ToolCall {
            call_id: "toolu_1".into(),
            name: "read_file".into(),
            arguments: r#"{"end_line":null,"path":"README.md","start_line":null}"#.into()
        }]
    );
    // The stored continuation validates against the visible response.
    response.continuation.validate_response(&response).unwrap();
    let (head, body) = server.await.unwrap();
    let head = head.to_ascii_lowercase();
    assert!(head.contains("x-api-key: sk-ant-test-0000000000000000"));
    assert!(head.contains("anthropic-version: 2023-06-01"));
    assert!(head.contains("anthropic-beta: server-side-fallback-2026-07-01"));
    assert!(!head.contains("authorization:"));
    assert_eq!(body["model"], "claude-opus-5-5");
    assert_eq!(body["fallbacks"], "default");
    assert_eq!(body["system"], "You are Astrid.");
    assert!(
        body.get("thinking").is_none(),
        "thinking is always on for Opus 5.5; never sent disabled"
    );
    let tool = body["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "read_file")
        .unwrap();
    assert!(tool["input_schema"].is_object() && tool["eager_input_streaming"] == true);

    // Native replay sends the raw blocks back, thinking signature included;
    // tool results from one batch merge into one user turn.
    let history = vec![
        Message::User("Summarize the README".into()),
        Message::Assistant(response.clone()),
        Message::Tool(ToolResult {
            call_id: "toolu_1".into(),
            name: "read_file".into(),
            outcome: ToolOutcome::Success {
                data: json!({"text":"hi"}),
            },
        }),
        Message::User("thanks".into()),
    ];
    let mut native = request(&history, "claude-opus-5-5");
    native.options.replay = ReplayMode::ProviderContinuation;
    let body = ClaudeProvider::body(&native).unwrap();
    let turns = body["messages"].as_array().unwrap();
    assert_eq!(
        turns
            .iter()
            .map(|t| t["role"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["user", "assistant", "user"]
    );
    assert_eq!(turns[1]["content"][0]["type"], "thinking");
    assert_eq!(turns[1]["content"][0]["signature"], "sig-abc");
    assert_eq!(turns[2]["content"][0]["type"], "tool_result");
    assert_eq!(turns[2]["content"][0]["tool_use_id"], "toolu_1");
    assert_eq!(turns[2]["content"][1]["text"], "thanks");
    // Visible-history replay (e.g. after a provider switch) drops thinking.
    let mut visible = request(&history, "claude-opus-5-5");
    visible.options.replay = ReplayMode::VisibleHistory;
    let body = ClaudeProvider::body(&visible).unwrap();
    assert_eq!(body["messages"][1]["content"][0]["type"], "text");
    assert_eq!(body["messages"][1]["content"][1]["type"], "tool_use");
}

#[tokio::test]
async fn refusals_truncation_bad_tool_json_and_http_errors_fail_cleanly() {
    let cases = [
        (
            vec![
                json!({"type":"message_delta","delta":{"stop_reason":"refusal"}}),
                json!({"type":"message_stop"}),
            ],
            "declined",
        ),
        (
            vec![
                json!({"type":"message_delta","delta":{"stop_reason":"max_tokens"}}),
                json!({"type":"message_stop"}),
            ],
            "max_tokens",
        ),
        (
            vec![
                json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"t","name":"shell","input":{}}}),
                json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"command\":\"ls"}}),
                json!({"type":"message_delta","delta":{"stop_reason":"tool_use"}}),
                json!({"type":"message_stop"}),
            ],
            "not valid JSON",
        ),
        (
            vec![
                json!({"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}),
            ],
            "Overloaded",
        ),
        (
            vec![json!({"type":"message_start","message":{}})],
            "before message_stop",
        ),
    ];
    for (events, expected) in cases {
        let (url, _server) = mock(200, "text/event-stream", sse(&events)).await;
        let provider = ClaudeProvider::with_endpoint(key(), url).unwrap();
        let messages = [Message::User("hi".into())];
        let error = provider
            .generate(&request(&messages, "claude-sonnet-5-5"), &mut |_: &str| {
                Ok(())
            })
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "{error} lacks {expected}"
        );
    }
    let (url, _server) = mock(401, "application/json", json!({"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}).to_string()).await;
    let provider = ClaudeProvider::with_endpoint(key(), url).unwrap();
    let messages = [Message::User("hi".into())];
    let error = provider
        .generate(
            &request(&messages, "claude-opus-5-5"),
            &mut |_: &str| Ok(()),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, ModelError::Http { status: 401, ref message } if message == "invalid x-api-key")
    );
}

#[test]
fn models_effort_and_keys_are_explicit() {
    assert!(supported_model("claude-opus-5-5") && supported_model("claude-haiku-4-5"));
    assert!(!supported_model("claude-opus-5-5-20260401") && !supported_model("claude-3-opus"));
    let messages = [Message::User("hi".into())];
    assert!(ClaudeProvider::body(&request(&messages, "claude-opus-4")).is_err());
    let mut r = request(&messages, "claude-opus-5-5");
    r.options.effort = Some(ReasoningEffort::Xhigh);
    assert_eq!(
        ClaudeProvider::body(&r).unwrap()["output_config"]["effort"],
        "xhigh"
    );
    r.options.effort = Some(ReasoningEffort::Disabled);
    assert!(
        ClaudeProvider::body(&r).is_err(),
        "thinking cannot be disabled on Opus 5.5"
    );
    let haiku = ClaudeProvider::body(&request(&messages, "claude-haiku-4-5")).unwrap();
    assert!(haiku.get("fallbacks").is_none());
    let snapshot = ClaudeProvider::snapshot(&haiku);
    assert_eq!(snapshot.serialized_request_bytes, bytes(&haiku));
    assert!(ApiKey::new("short".into()).is_err());
    assert!(ApiKey::new("sk-ant-has\"quote-000000000000".into()).is_err());
    assert_eq!(format!("{:?}", key()), "ApiKey([private])");
}
