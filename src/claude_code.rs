//! Claude Agent SDK provider (ADR 0032). Each Astrid model call runs one bounded
//! Claude Code turn through `apps/claude-agent-bridge`, on whatever account
//! Claude Code is logged into. Built-in Claude Code tools are off; Astrid's
//! tools are declared and every call comes back here for Astrid to execute.
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::{
    context::{ContextMeasurement, ContextSnapshot, ContextSource},
    model::{Message, ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink, ToolCall},
    openai::ResponseContinuation,
    routing::ModelCapability,
    tools,
};

pub const PREFIX: &str = "claude-code/";
const MAX_LINE: u64 = 4 * 1024 * 1024;
const TURN_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// ADR 0025's total visual attempt, from handing the frame to the bridge.
const VISUAL_TIMEOUT: Duration = Duration::from_secs(60);

/// Accepts Claude Code aliases (`opus`, `sonnet`, `haiku`) and `claude-*` IDs.
pub fn valid_model(model: &str) -> bool {
    matches!(model, "opus" | "sonnet" | "haiku" | "fable")
        || (model.starts_with("claude-")
            && model.len() <= 64
            && model
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.'))
}

/// How to start the bridge: a program plus arguments, then the JSON request on stdin.
#[derive(Clone, Debug)]
pub struct Bridge {
    pub command: Vec<String>,
}
impl Bridge {
    /// `node <repo>/apps/claude-agent-bridge/bridge.mjs`, overridable with
    /// `ASTRID_NODE` and `ASTRID_CLAUDE_BRIDGE`.
    pub fn locate() -> Result<Self, String> {
        let node = std::env::var("ASTRID_NODE").unwrap_or_else(|_| "node".into());
        let script = match std::env::var("ASTRID_CLAUDE_BRIDGE") {
            Ok(path) => PathBuf::from(path),
            Err(_) => std::env::current_exe()
                .ok()
                .and_then(|exe| {
                    exe.ancestors()
                        .nth(3)
                        .map(|root| root.join("apps/claude-agent-bridge/bridge.mjs"))
                })
                .ok_or("cannot locate the Claude agent bridge; set ASTRID_CLAUDE_BRIDGE")?,
        };
        let modules = script
            .parent()
            .map(|d| d.join("node_modules/@anthropic-ai/claude-agent-sdk"));
        if !script.is_file() || !modules.is_some_and(|m| m.is_dir()) {
            return Err(format!(
                "Claude agent bridge not installed: run `npm install` in {} (about 400 MB, includes Claude Code)",
                script
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            ));
        }
        Ok(Self {
            command: vec![node, script.display().to_string()],
        })
    }
}

pub struct ClaudeCodeProvider {
    bridge: Bridge,
    cwd: PathBuf,
}

impl ClaudeCodeProvider {
    pub fn new(bridge: Bridge, cwd: PathBuf) -> Self {
        Self { bridge, cwd }
    }

    /// Renders Astrid's history as one prompt. The SDK takes a prompt, not a
    /// Messages array, so every turn replays visible history only.
    pub fn prompt(messages: &[Message]) -> String {
        if let [Message::User(only)] = messages {
            return only.clone();
        }
        let mut out = String::from(
            "Conversation so far, replayed by Astrid. Tool calls were executed by Astrid and their results follow them.\n\n",
        );
        for message in messages {
            match message {
                Message::User(text) => out.push_str(&format!("<user>\n{text}\n</user>\n")),
                Message::Summary(text) => out.push_str(&format!("<summary>\n{text}\n</summary>\n")),
                Message::Assistant(response) => {
                    out.push_str("<assistant>\n");
                    if !response.text.is_empty() {
                        out.push_str(&response.text);
                        out.push('\n');
                    }
                    for call in &response.tool_calls {
                        out.push_str(&format!(
                            "<tool_call id=\"{}\" name=\"{}\">{}</tool_call>\n",
                            call.call_id, call.name, call.arguments
                        ));
                    }
                    out.push_str("</assistant>\n");
                }
                Message::Tool(result) => out.push_str(&format!(
                    "<tool_result id=\"{}\" name=\"{}\">{}</tool_result>\n",
                    result.call_id,
                    result.name,
                    serde_json::to_string(&result.outcome)
                        .expect("JSON tool outcome is serializable")
                )),
            }
        }
        out.push_str("\nContinue as the assistant from here: answer, or call Astrid's tools to make progress.");
        out
    }

    fn request(&self, request: &ModelRequest<'_>) -> Result<Value, ModelError> {
        let model = request
            .model
            .strip_prefix(PREFIX)
            .filter(|m| valid_model(m))
            .ok_or_else(|| {
                ModelError::Provider(format!("unsupported Claude Code model {:?}", request.model))
            })?;
        if request.options.computer_use {
            return Err(ModelError::Provider(
                "InstaBox computer use is not supported through Claude Code".into(),
            ));
        }
        if request.options.effort.is_some() {
            return Err(ModelError::Provider(
                "explicit reasoning effort is not supported through Claude Code".into(),
            ));
        }
        let mut definitions = tools::definitions();
        if request.options.instabox_exec {
            definitions.push(crate::remote::definition());
        }
        let mail: Vec<&str> = [
            (request.options.mail_apple, "apple"),
            (request.options.mail_gmail, "gmail"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect();
        if !mail.is_empty() {
            definitions.extend(crate::mail::definitions(&mail));
        }
        // ADR 0038: local look and presentation tools (look, point_at, sketch, say).
        if request.options.look {
            definitions.extend(crate::computer::look_definitions());
        }
        let tools: Vec<Value> = definitions
            .iter()
            .map(|d| json!({"name":d["name"],"description":d["description"],"parameters":d["parameters"]}))
            .collect();
        Ok(json!({
            "model": model,
            "system": request.instructions,
            "prompt": Self::prompt(request.messages),
            "tools": tools,
            "cwd": self.cwd,
            "client": concat!("astrid/", env!("CARGO_PKG_VERSION")),
        }))
    }

    /// The request plus one approved look frame (ADR 0038). The image goes to
    /// the bridge as base64 PNG beside the prompt, never into history.
    fn visual_request(
        &self,
        request: &ModelRequest<'_>,
        image: &crate::computer::VisionInput,
    ) -> Result<Value, ModelError> {
        if request.model != image.model()
            || crate::computer::vision_destination("claude_agent_sdk", request.model).is_none()
        {
            return Err(ModelError::Provider(
                "unverified image model binding".into(),
            ));
        }
        let mut body = self.request(request)?;
        let url = image.data_url().map_err(ModelError::Provider)?;
        let data = url
            .strip_prefix("data:image/png;base64,")
            .ok_or_else(|| ModelError::Protocol("unexpected image encoding".into()))?;
        body["image"] = json!({"media_type":"image/png","data":data,"text":image.frame_text()});
        Ok(body)
    }

    fn visual_snapshot(body: &Value) -> ContextSnapshot {
        let image = serde_json::to_vec(&body["image"])
            .map(|b| b.len())
            .unwrap_or(0);
        // Exactly the text request's snapshot, plus one image entry.
        let mut text_only = body.clone();
        if let Some(fields) = text_only.as_object_mut() {
            fields.remove("image");
        }
        let mut snapshot = Self::snapshot(&text_only);
        snapshot.serialized_request_bytes += image;
        snapshot.measurements.push(ContextMeasurement {
            source: ContextSource::ImageInput,
            entries: 1,
            serialized_bytes: image,
            contains_opaque_data: true,
        });
        snapshot
    }

    fn snapshot(request: &Value) -> ContextSnapshot {
        use ContextSource::*;
        let size = |v: &Value| serde_json::to_vec(v).map(|b| b.len()).unwrap_or(0);
        let sources = [
            Instructions,
            UserMessages,
            AssistantContinuation,
            ToolResults,
            ToolDefinitions,
            RequestFraming,
            Summary,
        ];
        let mut m = sources.map(|source| ContextMeasurement {
            source,
            entries: 0,
            serialized_bytes: 0,
            contains_opaque_data: false,
        });
        m[0] = ContextMeasurement {
            source: Instructions,
            entries: 1,
            serialized_bytes: size(&request["system"]),
            contains_opaque_data: false,
        };
        m[1] = ContextMeasurement {
            source: UserMessages,
            entries: 1,
            serialized_bytes: size(&request["prompt"]),
            contains_opaque_data: false,
        };
        m[4] = ContextMeasurement {
            source: ToolDefinitions,
            entries: 1,
            serialized_bytes: size(&request["tools"]),
            contains_opaque_data: false,
        };
        let total = size(request);
        m[5] = ContextMeasurement {
            source: RequestFraming,
            entries: 1,
            serialized_bytes: total
                - m[0].serialized_bytes
                - m[1].serialized_bytes
                - m[4].serialized_bytes,
            contains_opaque_data: false,
        };
        ContextSnapshot {
            measurements: m.into(),
            serialized_request_bytes: total,
            non_opaque_json_size_token_heuristic: total.div_ceil(4),
            provider_input_tokens: None,
        }
    }
}

#[async_trait]
impl ModelProvider for ClaudeCodeProvider {
    fn capabilities(&self, _model: &str) -> ModelCapability {
        ModelCapability {
            visible_history: true,
            reasoning_efforts: vec![],
        }
    }
    fn provider_name(&self) -> Option<&'static str> {
        Some("claude_agent_sdk")
    }
    fn measure_request(
        &self,
        request: &ModelRequest<'_>,
    ) -> Result<Option<ContextSnapshot>, ModelError> {
        Ok(Some(Self::snapshot(&self.request(request)?)))
    }

    fn measure_visual_request(
        &self,
        request: &ModelRequest<'_>,
        image: &crate::computer::VisionInput,
    ) -> Result<Option<ContextSnapshot>, ModelError> {
        Ok(Some(Self::visual_snapshot(
            &self.visual_request(request, image)?,
        )))
    }

    async fn generate_visual(
        &self,
        request: &ModelRequest<'_>,
        image: &crate::computer::VisionInput,
        grant: &crate::computer::DisclosureGrant,
        cancel: &crate::cancellation::Cancellation,
        text: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        // The grant is consumed against the live frame right before the bridge
        // starts; the local frame is released on every path out of here.
        let admitted = async {
            let body = self.visual_request(request, image)?;
            text.request_prepared(Self::visual_snapshot(&body)).await?;
            grant
                .consume(image, request.model, cancel)
                .map_err(ModelError::Provider)?;
            Ok::<_, ModelError>(body)
        }
        .await;
        image.release();
        let body = admitted?;
        tokio::select! {biased;
            _ = cancel.cancelled() => Err(ModelError::Provider("visual request cancelled; remote outcome unknown".into())),
            result = tokio::time::timeout(VISUAL_TIMEOUT, self.run(body, text)) => result
                .map_err(|_| ModelError::Provider("visual response deadline (60 seconds); remote outcome unknown".into()))?,
        }
    }

    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        text: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        let body = self.request(request)?;
        text.request_prepared(Self::snapshot(&body)).await?;
        self.run(body, text).await
    }
}

impl ClaudeCodeProvider {
    /// One bridge process: the request on stdin, JSON lines back.
    async fn run(&self, body: Value, text: &mut dyn TextSink) -> Result<ModelResponse, ModelError> {
        let (program, args) = self
            .bridge
            .command
            .split_first()
            .ok_or_else(|| ModelError::Provider("empty bridge command".into()))?;
        let mut child = tokio::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                ModelError::Provider(
                    "could not start the Claude agent bridge (is Node installed? set ASTRID_NODE)"
                        .into(),
                )
            })?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| ModelError::Provider("bridge stdin unavailable".into()))?;
        let payload = serde_json::to_vec(&body)
            .map_err(|_| ModelError::Protocol("request serialization failed".into()))?;
        drop(body);
        stdin.write_all(&payload).await?;
        drop(payload);
        drop(stdin);
        let mut reader = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| ModelError::Provider("bridge stdout unavailable".into()))?,
        );
        let turn = async {
            let mut streamed = String::new();
            loop {
                let mut line = Vec::new();
                let n = (&mut reader)
                    .take(MAX_LINE + 1)
                    .read_until(b'\n', &mut line)
                    .await?;
                if n == 0 {
                    return Err(ModelError::Protocol(
                        "Claude agent bridge exited without a result".into(),
                    ));
                }
                if line.len() as u64 > MAX_LINE {
                    return Err(ModelError::Protocol("bridge line exceeds 4 MiB".into()));
                }
                let Ok(event) = serde_json::from_slice::<Value>(&line) else {
                    continue;
                };
                match event["type"].as_str() {
                    Some("delta") => {
                        let piece = event["text"].as_str().unwrap_or("");
                        streamed.push_str(piece);
                        text.delta(piece).await?;
                    }
                    Some("error") => {
                        return Err(ModelError::Provider(
                            event["message"]
                                .as_str()
                                .unwrap_or("Claude Code failed")
                                .chars()
                                .take(2000)
                                .collect(),
                        ));
                    }
                    Some("done") => return finish(&event),
                    _ => {}
                }
            }
        };
        let result = tokio::time::timeout(TURN_TIMEOUT, turn)
            .await
            .map_err(|_| ModelError::Provider("Claude Code turn timed out".into()))?;
        let _ = child.wait().await;
        result
    }
}

fn finish(event: &Value) -> Result<ModelResponse, ModelError> {
    let text = event["text"].as_str().unwrap_or("").to_owned();
    let mut calls = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for call in event["tool_calls"].as_array().into_iter().flatten() {
        let id = call["id"].as_str().unwrap_or("").to_owned();
        let name = call["name"].as_str().unwrap_or("").to_owned();
        let arguments: Value = serde_json::from_str(call["arguments"].as_str().unwrap_or(""))
            .map_err(|_| ModelError::Protocol("tool arguments are not valid JSON".into()))?;
        if id.is_empty() || name.is_empty() || !arguments.is_object() || !ids.insert(id.clone()) {
            return Err(ModelError::Protocol(
                "invalid or duplicated tool call".into(),
            ));
        }
        calls.push(ToolCall {
            call_id: id,
            name,
            arguments: arguments.to_string(),
        });
    }
    Ok(ModelResponse {
        continuation: ResponseContinuation::from_parts(&text, &calls, None),
        text,
        tool_calls: calls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ToolOutcome, ToolResult};

    fn fake(script: &str) -> ClaudeCodeProvider {
        ClaudeCodeProvider::new(
            Bridge {
                command: vec![
                    "/bin/sh".into(),
                    "-c".into(),
                    format!("cat > /dev/null; {script}"),
                ],
            },
            "/tmp".into(),
        )
    }
    fn request<'a>(messages: &'a [Message], model: &'a str) -> ModelRequest<'a> {
        ModelRequest {
            options: Default::default(),
            model,
            instructions: "You are Astrid.",
            messages,
        }
    }

    #[tokio::test]
    async fn bridge_deltas_and_tool_calls_become_one_response() {
        let provider = fake(
            r#"printf '%s\n' '{"type":"delta","text":"Reading "}' 'noise' '{"type":"delta","text":"it."}' '{"type":"done","text":"Reading it.","tool_calls":[{"id":"toolu_1","name":"read_file","arguments":"{\"path\":\"README.md\",\"start_line\":1,\"end_line\":5}"}]}'"#,
        );
        let messages = [Message::User("Summarize the README".into())];
        let mut streamed = String::new();
        let response = provider
            .generate(&request(&messages, "claude-code/sonnet"), &mut |d: &str| {
                streamed.push_str(d);
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(streamed, "Reading it.");
        assert_eq!(response.text, "Reading it.");
        assert_eq!(response.tool_calls[0].name, "read_file");
        assert_eq!(
            response.tool_calls[0].arguments,
            r#"{"end_line":5,"path":"README.md","start_line":1}"#
        );
        response.continuation.validate_response(&response).unwrap();
    }

    #[tokio::test]
    async fn bridge_errors_bad_json_and_silent_exits_fail_cleanly() {
        let messages = [Message::User("hi".into())];
        for (script, expected) in [
            (
                r#"printf '%s\n' '{"type":"error","message":"Claude Code: rate_limit"}'"#,
                "rate_limit",
            ),
            (
                r#"printf '%s\n' '{"type":"done","text":"","tool_calls":[{"id":"t","name":"shell","arguments":"{bad"}]}'"#,
                "not valid JSON",
            ),
            ("exit 1", "without a result"),
        ] {
            let error = fake(script)
                .generate(&request(&messages, "claude-code/opus"), &mut |_: &str| {
                    Ok(())
                })
                .await
                .unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
        let missing = ClaudeCodeProvider::new(
            Bridge {
                command: vec!["/nonexistent/node".into()],
            },
            "/tmp".into(),
        );
        assert!(
            missing
                .generate(&request(&messages, "claude-code/opus"), &mut |_: &str| Ok(
                    ()
                ))
                .await
                .unwrap_err()
                .to_string()
                .contains("ASTRID_NODE")
        );
    }

    #[test]
    fn requests_replay_visible_history_and_reject_unsupported_options() {
        let provider = fake("true");
        let history = vec![
            Message::User("List the folder".into()),
            Message::Assistant(finish(&json!({"text":"Looking.","tool_calls":[{"id":"t1","name":"list_directory","arguments":"{\"path\":\".\"}"}]})).unwrap()),
            Message::Tool(ToolResult { call_id: "t1".into(), name: "list_directory".into(), outcome: ToolOutcome::Success { data: json!({"entries":["a.txt"]}) } }),
        ];
        let body = provider
            .request(&request(&history, "claude-code/claude-opus-5-5"))
            .unwrap();
        assert_eq!(body["model"], "claude-opus-5-5");
        let prompt = body["prompt"].as_str().unwrap();
        assert!(prompt.contains("<user>\nList the folder\n</user>"));
        assert!(
            prompt.contains(r#"<tool_call id="t1" name="list_directory">{"path":"."}</tool_call>"#)
        );
        assert!(
            prompt.contains(r#"<tool_result id="t1" name="list_directory">"#)
                && prompt.contains("a.txt")
        );
        assert!(
            body["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["name"] == "read_file" && t["parameters"].is_object())
        );
        let single = [Message::User("hi".into())];
        assert_eq!(
            provider
                .request(&request(&single, "claude-code/haiku"))
                .unwrap()["prompt"],
            "hi"
        );
        for bad in [
            "claude-code/gpt-5",
            "claude-code/claude-x;rm",
            "claude-opus-5-5",
        ] {
            assert!(provider.request(&request(&single, bad)).is_err(), "{bad}");
        }
        let mut effort = request(&single, "claude-code/opus");
        effort.options.effort = Some(crate::routing::ReasoningEffort::High);
        assert!(provider.request(&effort).is_err());
    }
}
