use super::*;
use crate::model::{ToolOutcome, ToolResult};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn info() -> ModelInfo {
    ModelInfo {
        id: "glm-5.3".into(),
        efforts: vec!["low".into(), "high".into(), "max".into()],
        reasoning_field: Some("reasoning_content".into()),
    }
}
fn key() -> ApiKey {
    ApiKey::new("oc-test-0000000000000000".into()).unwrap()
}
fn request<'a>(messages: &'a [Message]) -> ModelRequest<'a> {
    ModelRequest {
        options: Default::default(),
        model: "opencode/glm-5.3",
        instructions: "You are Astrid.",
        messages,
    }
}
fn sse(chunks: &[Value]) -> String {
    chunks
        .iter()
        .map(|c| format!("data: {c}\n\n"))
        .collect::<String>()
        + "data: [DONE]\n\n"
}
async fn mock(status: u16, reply: String) -> (String, tokio::task::JoinHandle<(String, Value)>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/chat/completions", listener.local_addr().unwrap());
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
            "HTTP/1.1 {status} X\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
            reply.len()
        );
        socket.write_all(wire.as_bytes()).await.unwrap();
        (head, body)
    });
    (url, task)
}

#[tokio::test]
async fn streams_text_reasoning_and_split_tool_calls() {
    let chunks = vec![
        json!({"choices":[{"index":0,"delta":{"role":"assistant","reasoning_content":"Need the "}}]}),
        json!({"choices":[{"index":0,"delta":{"reasoning_content":"README."}}]}),
        json!({"choices":[{"index":0,"delta":{"content":"Reading it."}}]}),
        json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"read_file","arguments":"{\"path\":"}}]}}]}),
        json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"README.md\",\"start_line\":null,\"end_line\":null}"}}]}}]}),
        json!({"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}),
        json!({"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":5}}),
    ];
    let (url, server) = mock(200, sse(&chunks)).await;
    let provider = OpenCodeProvider::with_endpoint(key(), info(), url).unwrap();
    let messages = [Message::User("Summarize the README".into())];
    let mut streamed = String::new();
    let response = provider
        .generate(&request(&messages), &mut |d: &str| {
            streamed.push_str(d);
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(streamed, "Reading it.");
    assert_eq!(
        response.tool_calls,
        vec![ToolCall {
            call_id: "call_1".into(),
            name: "read_file".into(),
            arguments: r#"{"end_line":null,"path":"README.md","start_line":null}"#.into()
        }]
    );
    response.continuation.validate_response(&response).unwrap();
    let (head, body) = server.await.unwrap();
    let head = head.to_ascii_lowercase();
    assert!(head.contains("authorization: bearer oc-test-0000000000000000"));
    assert!(
        head.contains("user-agent: astrid/"),
        "OpenCode asks clients to identify themselves"
    );
    let session = OpenCodeProvider::session_id(&request(&messages));
    assert!(head.contains(&format!("x-opencode-session: {session}")));
    assert_eq!(body["model"], "glm-5.3");
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["tools"][0]["type"], "function");

    // Same conversation keeps the same session id as it grows.
    let history = vec![
        Message::User("Summarize the README".into()),
        Message::Assistant(response.clone()),
        Message::Tool(ToolResult {
            call_id: "call_1".into(),
            name: "read_file".into(),
            outcome: ToolOutcome::Success {
                data: json!({"text":"hi"}),
            },
        }),
    ];
    let mut native = request(&history);
    assert_eq!(OpenCodeProvider::session_id(&native), session);
    native.options.replay = ReplayMode::ProviderContinuation;
    let body = provider.body(&native).unwrap();
    assert_eq!(body["messages"][2]["reasoning_content"], "Need the README.");
    assert_eq!(body["messages"][2]["tool_calls"][0]["id"], "call_1");
    assert_eq!(body["messages"][3]["role"], "tool");
    assert_eq!(body["messages"][3]["tool_call_id"], "call_1");
    native.options.replay = ReplayMode::VisibleHistory;
    let body = provider.body(&native).unwrap();
    assert!(body["messages"][2].get("reasoning_content").is_none());
}

#[tokio::test]
async fn length_filter_bad_arguments_http_errors_and_missing_done_fail_cleanly() {
    let cases = [
        (
            vec![json!({"choices":[{"delta":{"content":"x"},"finish_reason":"length"}]})],
            "output limit",
        ),
        (
            vec![json!({"choices":[{"delta":{},"finish_reason":"content_filter"}]})],
            "content filter",
        ),
        (
            vec![
                json!({"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c","function":{"name":"shell","arguments":"{\"command\":"}}]},"finish_reason":"tool_calls"}]}),
            ],
            "not valid JSON",
        ),
        (
            vec![json!({"error":{"message":"Rate limited"}})],
            "Rate limited",
        ),
    ];
    for (chunks, expected) in cases {
        let (url, _s) = mock(200, sse(&chunks)).await;
        let provider = OpenCodeProvider::with_endpoint(key(), info(), url).unwrap();
        let messages = [Message::User("hi".into())];
        let error = provider
            .generate(&request(&messages), &mut |_: &str| Ok(()))
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "{error} lacks {expected}"
        );
    }
    let (url, _s) = mock(
        200,
        "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n".into(),
    )
    .await;
    let provider = OpenCodeProvider::with_endpoint(key(), info(), url).unwrap();
    let messages = [Message::User("hi".into())];
    assert!(
        provider
            .generate(&request(&messages), &mut |_: &str| Ok(()))
            .await
            .unwrap_err()
            .to_string()
            .contains("before completion")
    );
    let (url, _s) = mock(
        401,
        json!({"error":{"message":"Invalid API key"}}).to_string(),
    )
    .await;
    let provider = OpenCodeProvider::with_endpoint(key(), info(), url).unwrap();
    let error = provider
        .generate(&request(&messages), &mut |_: &str| Ok(()))
        .await
        .unwrap_err();
    assert!(
        matches!(error, ModelError::Http { status: 401, ref message } if message == "Invalid API key")
    );
}

#[test]
fn catalog_keeps_chat_completions_tool_models_and_effort_is_exact() {
    let catalog = json!({"data":[
        {"id":"glm-5.3","capabilities":{"tools":{"supported":true},"reasoning":{"effort":["low","high","max"]}},"api":{"chat-completions":{"compatibility":{"reasoning_field":"reasoning_content"}}}},
        {"id":"grok-4.7","capabilities":{"tools":{"supported":true}},"api":{"responses":{}}},
        {"id":"no-tools","capabilities":{"tools":{"supported":false}},"api":{"chat-completions":{}}},
        {"id":"bad id/../x","capabilities":{"tools":{"supported":true}},"api":{"chat-completions":{}}},
    ]});
    assert_eq!(parse_catalog(&catalog), vec![info()]);
    let provider = OpenCodeProvider::new(key(), info()).unwrap();
    assert_eq!(
        provider.capability().reasoning_efforts,
        vec![
            ReasoningEffort::Low,
            ReasoningEffort::High,
            ReasoningEffort::Max
        ]
    );
    let messages = [Message::User("hi".into())];
    let mut r = request(&messages);
    r.options.effort = Some(ReasoningEffort::High);
    assert_eq!(provider.body(&r).unwrap()["reasoning_effort"], "high");
    r.options.effort = Some(ReasoningEffort::Medium);
    assert!(provider.body(&r).is_err());
    let mut other = request(&messages);
    other.model = "opencode/kimi-k3";
    assert!(provider.body(&other).is_err());
    assert!(ApiKey::new("short".into()).is_err());
    assert_eq!(format!("{:?}", key()), "ApiKey([private])");
}
