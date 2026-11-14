use super::*;
use crate::{
    auth::{AuthError, Authentication, BearerToken},
    computer::Computer,
    computer_png::DecodeWorker,
    openai::OpenAiProvider,
    tools::Tools,
    workspace::Workspace,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const FRAME: Frame = Frame {
    width: 320,
    height: 200,
};

fn block(value: Value) -> String {
    format!("Here's the next step.\n```operator\n{value}\n```")
}
fn step(index: u32, total: u32, x: u32, cursor: &str, action: Value) -> Value {
    json!({"index":index,"total":total,"instruction":format!("Do step {index}."),"x":x,"y":20,"cursor":cursor,"action":action,"check":format!("Step {index} shows done.")})
}

#[test]
fn reports_are_validated_against_the_frame() {
    let ok = parse_report(
        &block(json!({"step":step(1, 3, 300, "click", json!({"kind":"click"}))})),
        FRAME,
    )
    .unwrap();
    assert_eq!(ok.verified, None);
    let Next::Step(s) = ok.next else { panic!() };
    assert_eq!((s.index, s.total, s.x, s.y), (1, 3, 300, 20));
    assert_eq!(s.cursor, "click");
    assert_eq!(s.action, Some(Action::Click));

    let typed = parse_report(
        &block(json!({"verified":true,"reason":"open","step":step(2, 2, 10, "text", json!({"kind":"type","text":"Q3 report"}))})),
        FRAME,
    )
    .unwrap();
    assert_eq!(typed.verified, Some(true));
    let Next::Step(s) = typed.next else { panic!() };
    assert_eq!(s.action, Some(Action::Type("Q3 report".into())));
    assert_eq!(
        parse_report(&block(json!({"verified":true,"finish":"All done."})), FRAME)
            .unwrap()
            .next,
        Next::Finish("All done.".into())
    );

    let rejected = [
        "no block at all".to_owned(),
        "```operator\n{not json}\n```".to_owned(),
        block(json!({"step":step(1, 2, 320, "click", Value::Null)})),
        block(json!({"step":step(1, 2, 10, "lasso", Value::Null)})),
        block(json!({"step":step(3, 2, 10, "click", Value::Null)})),
        block(json!({"step":step(1, 9, 10, "click", Value::Null)})),
        block(json!({"step":step(1, 2, 10, "click", json!({"kind":"drag"}))})),
        block(json!({"step":step(1, 2, 10, "text", json!({"kind":"type","text":"a\nb"}))})),
        block(
            json!({"step":step(1, 2, 10, "text", json!({"kind":"type","text":"x".repeat(201)}))}),
        ),
        block(json!({"step":step(1, 2, 10, "click", json!({"kind":"key","key":"cmd+q"}))})),
        block(
            json!({"step":step(1, 2, 10, "click", json!({"kind":"scroll","direction":"down","notches":11}))}),
        ),
        block(json!({"verified":"yes","finish":"x"})),
        block(json!({"finish":"x","step":step(1, 1, 1, "click", Value::Null)})),
        block(json!({"reason":"nothing next"})),
    ];
    for reply in rejected {
        assert!(parse_report(&reply, FRAME).is_err(), "{reply}");
    }
}

fn report(verified: Option<bool>, next: Next) -> Report {
    Report {
        verified,
        reason: String::new(),
        next,
    }
}
fn plain(index: u32) -> Step {
    let Next::Step(s) = parse_report(
        &block(json!({"step":step(index, 5, 10, "click", Value::Null)})),
        FRAME,
    )
    .unwrap()
    .next
    else {
        panic!()
    };
    s
}

#[test]
fn a_step_is_never_advanced_past_without_verification() {
    let mut w = Walkthrough::new("export");
    assert!(matches!(
        w.accept(report(None, Next::Step(plain(2)))),
        Advance::Stopped(_)
    ));
    let mut w = Walkthrough::new("export");
    assert_eq!(
        w.accept(report(None, Next::Step(plain(1)))),
        Advance::Show(plain(1))
    );
    // Unanswered check or a jump ahead: not verified, never skipped.
    assert!(matches!(
        w.accept(report(None, Next::Step(plain(2)))),
        Advance::Stopped(_)
    ));
    assert!(w.done.is_empty());

    let mut w = Walkthrough::new("export");
    w.accept(report(None, Next::Step(plain(1))));
    assert!(matches!(
        w.accept(report(Some(false), Next::Finish("done".into()))),
        Advance::Stopped(_)
    ));

    let mut w = Walkthrough::new("export");
    w.accept(report(None, Next::Step(plain(1))));
    for _ in 0..MAX_ATTEMPTS {
        assert!(matches!(
            w.accept(report(Some(false), Next::Step(plain(1)))),
            Advance::Retry { .. }
        ));
    }
    assert!(matches!(
        w.accept(report(Some(false), Next::Step(plain(1)))),
        Advance::Stopped(_)
    ));

    let mut w = Walkthrough::new("export");
    w.accept(report(None, Next::Step(plain(1))));
    assert_eq!(
        w.accept(report(Some(true), Next::Step(plain(2)))),
        Advance::Show(plain(2))
    );
    assert!(matches!(
        w.accept(report(Some(true), Next::Step(plain(4)))),
        Advance::Stopped(_)
    ));
    let mut w = Walkthrough::new("export");
    w.accept(report(None, Next::Step(plain(1))));
    assert_eq!(
        w.accept(report(Some(true), Next::Finish("Exported.".into()))),
        Advance::Finished("Exported.".into())
    );
    assert_eq!(w.done, vec!["Do step 1.".to_string()]);
    assert!(
        w.task()
            .contains("Steps already done and verified:\n1. Do step 1.")
    );
}

#[test]
fn looks_are_capped_per_session() {
    let mut w = Walkthrough::new("x");
    for _ in 0..MAX_LOOKS {
        w.record_look().unwrap();
    }
    assert!(w.record_look().is_err());
}

#[tokio::test]
async fn the_press_approves_exactly_one_look_and_nothing_else() {
    let look = PermissionRequest {
        command: json!({"capability":"local.look"}).to_string(),
        workspace: "/".into(),
    };
    let other = PermissionRequest {
        command: json!({"capability":"mail.read_only"}).to_string(),
        workspace: "/".into(),
    };
    let mut grant = PressedLook { armed: true };
    assert!(!grant.decide(&other).await.unwrap());
    let mut grant = PressedLook { armed: true };
    assert!(grant.decide(&look).await.unwrap());
    assert!(!grant.decide(&look).await.unwrap());
}

// ---- Whole sessions: mock inference, fake capture host, scripted presses.

struct Auth;
#[async_trait]
impl Authentication for Auth {
    async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
        BearerToken::new("test-only-provider-credential".into())
    }
}

fn png_of(width: u32, height: u32) -> Vec<u8> {
    let mut v = Vec::new();
    {
        let mut e = png::Encoder::new(&mut v, width, height);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().unwrap();
        w.write_image_data(&vec![90; (width * height * 4) as usize])
            .unwrap();
    }
    v
}

#[derive(Default)]
struct FakeHost {
    captures: Mutex<usize>,
    elements: Option<Value>,
}
#[async_trait]
impl crate::presence::LookHost for FakeHost {
    async fn capture(&self, _: u32, _: &Cancellation) -> Result<Vec<u8>, String> {
        *self.captures.lock().unwrap() += 1;
        Ok(png_of(320, 200))
    }
    async fn present(&self, _: Value) -> Result<(), String> {
        Ok(())
    }
    fn elements(&self) -> Option<Value> {
        self.elements.clone()
    }
}

/// Serves the scripted look runs: request 2k asks for a look, 2k+1 (which
/// carries the one image) answers with report k.
async fn mock_model(reports: Vec<String>) -> (String, Arc<Mutex<Vec<Value>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/v1/responses", listener.local_addr().unwrap());
    let seen: Arc<Mutex<Vec<Value>>> = Arc::default();
    let out = seen.clone();
    tokio::spawn(async move {
        let mut n = 0;
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                socket.read_exact(&mut b).await.unwrap();
                head.push(b[0]);
            }
            let head = String::from_utf8(head).unwrap();
            let length = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|n| n.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            let body: Value = serde_json::from_slice(&body).unwrap();
            let images = body.to_string().matches("input_image").count();
            out.lock().unwrap().push(body.clone());
            let item = if n % 2 == 0 {
                assert_eq!(images, 0, "no image before the look");
                json!({"type":"function_call","id":format!("item_{n}"),"call_id":format!("look-{n}"),"name":"look","namespace":"astrid","arguments":"{}","status":"completed"})
            } else {
                assert_eq!(images, 1, "exactly the one approved frame");
                json!({"type":"message","id":"msg","role":"assistant","status":"completed","content":[{"type":"output_text","text":reports[n / 2],"annotations":[]}]})
            };
            let sse = format!(
                "data: {}\n\n",
                json!({"type":"response.completed","response":{"id":"resp","status":"completed","output":[item]}})
            );
            n += 1;
            let wire = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{sse}",
                sse.len()
            );
            let _ = socket.write_all(wire.as_bytes()).await;
        }
    });
    (origin, seen)
}

#[derive(Default)]
struct FakeUi {
    choices: Mutex<VecDeque<Choice>>,
    cards: Mutex<Vec<Card>>,
    events: Mutex<Vec<Value>>,
    acts: Mutex<Vec<Step>>,
    /// One entry per act: Some(error) fails it; missing entries succeed.
    act_errors: Mutex<VecDeque<Option<&'static str>>>,
}
#[async_trait]
impl OperatorUi for FakeUi {
    async fn card(&self, card: Card) -> Choice {
        self.cards.lock().unwrap().push(card);
        self.choices
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Choice::Stop)
    }
    async fn present(&self, event: Value) {
        self.events.lock().unwrap().push(event);
    }
    async fn act(&self, step: &Step) -> Result<(), String> {
        self.acts.lock().unwrap().push(step.clone());
        match self.act_errors.lock().unwrap().pop_front().flatten() {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }
    fn state(&self, _: &str, _: &str) {}
}

async fn session(
    reports: Vec<String>,
    mode: Mode,
    choices: Vec<Choice>,
    act_errors: Vec<Option<&'static str>>,
    elements: Option<Value>,
) -> (Outcome, FakeUi, usize, Vec<Value>) {
    let (origin, seen) = mock_model(reports).await;
    let host = Arc::new(FakeHost {
        elements,
        ..FakeHost::default()
    });
    let dir = tempfile::tempdir().unwrap();
    let tools = Tools::new(Workspace::new(dir.path()).unwrap(), Duration::from_secs(1))
        .unwrap()
        .with_computer(
            Computer::local(
                "test".into(),
                0,
                host.clone(),
                Arc::new(DecodeWorker::inline()),
            )
            .unwrap(),
        );
    let provider = OpenAiProvider::with_endpoint(Arc::new(Auth), origin).unwrap();
    let mut eyes = RuntimeEyes {
        provider: &provider,
        tools: &tools,
        model: "gpt-5.6-sol".into(),
    };
    let ui = FakeUi {
        choices: Mutex::new(choices.into()),
        act_errors: Mutex::new(act_errors.into()),
        ..FakeUi::default()
    };
    let outcome = walk(
        "Export the report",
        mode,
        0,
        &mut eyes,
        &ui,
        &Cancellation::default(),
    )
    .await;
    let captures = *host.captures.lock().unwrap();
    let requests = seen.lock().unwrap().clone();
    (outcome, ui, captures, requests)
}

#[tokio::test]
async fn astrid_does_the_steps_itself_and_asks_only_before_risky_ones() {
    let reports = vec![
        block(json!({"step":step(1, 2, 300, "click", json!({"kind":"click"}))})),
        block(json!({"verified":true,"step":{
            "index":2,"total":2,"instruction":"Type Q3 and send it.","x":100,"y":20,"cursor":"text",
            "action":{"kind":"type","text":"Q3"},"confirm":true,"check":"Sent."}})),
        block(
            json!({"verified":false,"reason":"Not sent yet.","step":step(2, 2, 101, "text", json!({"kind":"type","text":"Q3"}))}),
        ),
        block(json!({"verified":true,"finish":"Exported."})),
    ];
    let (outcome, ui, captures, requests) =
        session(reports, Mode::Do, vec![Choice::DoIt], vec![], None).await;
    assert_eq!(outcome, Outcome::Completed("Exported.".into()));
    // The ask starts it: no card before the first look, then a look after each act.
    assert_eq!(captures, 4);
    assert_eq!(requests.len(), 8);
    assert!(requests[2].to_string().contains("Pending step 1 of 2"));
    let events = ui.events.lock().unwrap().clone();
    assert_eq!(events[0], json!({"v":1,"type":"wave"}));
    assert_eq!(events.iter().filter(|e| e["type"] == "wave").count(), 1);
    // Astrid points with the step's cursor where it acts.
    assert!(
        events.contains(
            &json!({"v":1,"type":"point_at","x":300,"y":20,"display":0,"cursor":"click"})
        )
    );
    assert!(
        events
            .contains(&json!({"v":1,"type":"point_at","x":100,"y":20,"display":0,"cursor":"text"}))
    );
    let acts = ui.acts.lock().unwrap().clone();
    assert_eq!(acts.len(), 3);
    assert_eq!(acts[0].action, Some(Action::Click));
    assert_eq!(acts[1].action, Some(Action::Type("Q3".into())));
    // Only the risky step asked, once.
    let cards = ui.cards.lock().unwrap().clone();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].step, Some((2, 2)));
    assert_eq!(cards[0].buttons, vec![Choice::DoIt, Choice::Stop]);
    assert!(cards[0].text.ends_with("Shall I?"));
}

#[tokio::test]
async fn show_me_points_and_waits_for_done_without_acting() {
    let reports = vec![
        block(json!({"step":step(1, 1, 300, "click", json!({"kind":"click"}))})),
        block(
            json!({"verified":false,"reason":"The menu is still closed.","step":step(1, 1, 301, "click", json!({"kind":"click"}))}),
        ),
        block(json!({"verified":true,"finish":"Exported."})),
    ];
    let (outcome, ui, captures, _) = session(
        reports,
        Mode::Guide,
        vec![Choice::Done, Choice::Done],
        vec![],
        None,
    )
    .await;
    assert_eq!(outcome, Outcome::Completed("Exported.".into()));
    assert_eq!(captures, 3);
    assert!(ui.acts.lock().unwrap().is_empty());
    let cards = ui.cards.lock().unwrap().clone();
    assert_eq!(cards[0].buttons, vec![Choice::Done, Choice::Stop]);
    assert!(
        cards[1]
            .text
            .starts_with("Not yet: The menu is still closed.")
    );
    assert!(
        ui.events
            .lock()
            .unwrap()
            .iter()
            .any(|e| e["type"] == "sketch")
    );
    assert_eq!(mode_for("show me how to export this"), Mode::Guide);
    assert_eq!(mode_for("Where is the export button?"), Mode::Guide);
    assert_eq!(mode_for("like this video"), Mode::Do);
}

#[tokio::test]
async fn a_shifted_screen_is_looked_at_again_and_a_refusal_falls_back_to_showing() {
    // The screen moved under the target: look again, then act.
    let reports = vec![
        block(json!({"step":step(1, 1, 300, "click", json!({"kind":"click"}))})),
        block(
            json!({"verified":false,"reason":"Not clicked.","step":step(1, 1, 302, "click", json!({"kind":"click"}))}),
        ),
        block(json!({"verified":true,"finish":"Liked."})),
    ];
    let (outcome, ui, captures, _) = session(
        reports,
        Mode::Do,
        vec![],
        vec![Some("the screen changed since I looked")],
        None,
    )
    .await;
    assert_eq!(outcome, Outcome::Completed("Liked.".into()));
    assert_eq!((captures, ui.acts.lock().unwrap().len()), (3, 2));
    assert!(ui.cards.lock().unwrap().is_empty());

    // Anything else it can't do itself, it shows instead, and waits.
    let reports = vec![block(
        json!({"step":step(1, 1, 300, "press", json!({"kind":"click"}))}),
    )];
    let (outcome, ui, captures, _) = session(
        reports,
        Mode::Do,
        vec![Choice::Stop],
        vec![Some(
            "Astrid Presence needs Accessibility access in System Settings",
        )],
        None,
    )
    .await;
    assert!(matches!(outcome, Outcome::Stopped(_)));
    assert_eq!(captures, 1);
    let cards = ui.cards.lock().unwrap().clone();
    assert!(
        cards[0]
            .text
            .starts_with("I couldn't do that myself (Astrid Presence needs Accessibility")
    );
    assert_eq!(cards[0].buttons, vec![Choice::Done, Choice::Stop]);
}

#[tokio::test]
async fn element_targets_act_on_the_exact_centre_from_accessibility() {
    let elements = json!([
        [3, "AXButton", "Share", 10, 10, 40, 20],
        [7, "AXButton", "like this video", 200, 150, 60, 30]
    ]);
    let reports = vec![
        block(
            json!({"step":{"index":1,"total":1,"instruction":"Click Like.","element":7,
            "cursor":"click","action":{"kind":"click"},"check":"Like is on."}}),
        ),
        block(json!({"verified":true,"finish":"Liked."})),
    ];
    let (outcome, ui, _, requests) =
        session(reports, Mode::Do, vec![], vec![], Some(elements)).await;
    assert_eq!(outcome, Outcome::Completed("Liked.".into()));
    // The model saw the elements with the look...
    assert!(requests[1].to_string().contains("like this video"));
    // ...and Astrid acted on element 7's centre, not a guessed point.
    let acts = ui.acts.lock().unwrap().clone();
    assert_eq!((acts[0].x, acts[0].y, acts[0].element), (230, 165, Some(7)));

    // An id that isn't on screen is refused, never guessed.
    let reports = vec![block(
        json!({"step":{"index":1,"total":1,"instruction":"Click.","element":99,
        "cursor":"click","action":{"kind":"click"},"check":"x"}}),
    )];
    let (outcome, ui, _, _) = session(
        reports,
        Mode::Do,
        vec![],
        vec![],
        Some(json!([[7, "AXButton", "Like", 200, 150, 60, 30]])),
    )
    .await;
    assert!(
        matches!(&outcome, Outcome::Failed(e) if e.contains("element 99")),
        "{outcome:?}"
    );
    assert!(ui.acts.lock().unwrap().is_empty());
}

#[test]
fn host_elements_are_sanitized_and_bounded() {
    use crate::computer::{MAX_ELEMENTS, sanitize_elements};
    let list = sanitize_elements(
        &json!([
            [1, "AXButton", "Like\nnow", 10, 10, 20, 20],
            [2, "AXButton", "off frame", 310, 10, 20, 20],
            [3, "", "no role", 10, 10, 5, 5],
            [1, "AXLink", "duplicate id", 10, 10, 5, 5],
            "junk",
            [4, "AXLink", "x".repeat(500), 0, 0, 5, 5]
        ]),
        320,
        200,
    )
    .unwrap();
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0], json!([1, "AXButton", "Like now", 10, 10, 20, 20]));
    assert_eq!(list[1][2].as_str().unwrap().len(), 120);
    let many: Vec<Value> = (0..400)
        .map(|i| json!([i, "AXButton", "b", 1, 1, 2, 2]))
        .collect();
    assert_eq!(
        sanitize_elements(&Value::Array(many), 320, 200)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        MAX_ELEMENTS
    );
    assert!(sanitize_elements(&json!([]), 320, 200).is_none());
}

/// Operator on Claude (ADR 0038): a fake Claude agent bridge that asks for a
/// look when it has no frame, and answers the report when it gets one. It
/// records each request so the test can see exactly what was sent.
#[tokio::test]
async fn operator_runs_on_claude_code_with_the_frame_as_an_image() {
    use crate::claude_code::{Bridge, ClaudeCodeProvider};
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("requests");
    std::fs::create_dir(&log).unwrap();
    std::fs::write(
        dir.path().join("report.json"),
        json!({"type":"done","text":block(json!({"step":step(1, 1, 300, "click", json!({"kind":"click"}))})),"tool_calls":[]}).to_string(),
    )
    .unwrap();
    let script = format!(
        r#"n=$(ls {log} | wc -l | tr -d ' '); cat > {log}/$n.json
if grep -q '"image":{{' {log}/$n.json; then cat {dir}/report.json; echo
else printf '%s\n' '{{"type":"done","text":"","tool_calls":[{{"id":"toolu_look","name":"look","arguments":"{{}}"}}]}}'; fi"#,
        log = log.display(),
        dir = dir.path().display()
    );
    let provider = ClaudeCodeProvider::new(
        Bridge {
            command: vec!["/bin/sh".into(), "-c".into(), script],
        },
        dir.path().to_path_buf(),
    );
    let host = Arc::new(FakeHost::default());
    let tools = Tools::new(Workspace::new(dir.path()).unwrap(), Duration::from_secs(1))
        .unwrap()
        .with_computer(
            Computer::local(
                "test".into(),
                0,
                host.clone(),
                Arc::new(DecodeWorker::inline()),
            )
            .unwrap(),
        );
    let mut eyes = RuntimeEyes {
        provider: &provider,
        tools: &tools,
        model: "claude-code/sonnet".into(),
    };
    let ui = FakeUi {
        choices: Mutex::new(vec![Choice::Stop].into()),
        ..FakeUi::default()
    };
    let outcome = walk(
        "Export the report",
        Mode::Guide,
        0,
        &mut eyes,
        &ui,
        &Cancellation::default(),
    )
    .await;
    assert!(matches!(outcome, Outcome::Stopped(_)), "{outcome:?}");
    assert_eq!(*host.captures.lock().unwrap(), 1);
    let cards = ui.cards.lock().unwrap().clone();
    assert_eq!(cards[0].step, Some((1, 1)));
    let read = |n: usize| -> Value {
        serde_json::from_slice(&std::fs::read(log.join(format!("{n}.json"))).unwrap()).unwrap()
    };
    let (first, second) = (read(0), read(1));
    // The look tool is offered to Claude; no image before the approved look.
    assert!(
        first["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "look")
    );
    assert!(first["image"].is_null());
    assert_eq!(first["model"], "sonnet");
    // Exactly the one frame, as a PNG, with its frame text.
    assert_eq!(second["image"]["media_type"], "image/png");
    assert!(
        second["image"]["data"]
            .as_str()
            .unwrap()
            .starts_with("iVBOR")
    );
    assert!(
        second["image"]["text"]
            .as_str()
            .unwrap()
            .contains("320x200")
    );
    assert!(
        !second["prompt"].as_str().unwrap().contains("iVBOR"),
        "the frame never enters the replayed history"
    );
    assert_eq!(std::fs::read_dir(&log).unwrap().count(), 2);
}

#[test]
fn only_named_claude_and_gpt_models_may_see_a_look() {
    use crate::computer::vision_destination;
    assert!(vision_destination("claude_agent_sdk", "claude-code/sonnet").is_some());
    assert!(vision_destination("claude_agent_sdk", "claude-code/opus").is_some());
    assert!(vision_destination("claude_agent_sdk", "claude-code/haiku").is_none());
    assert!(vision_destination("claude_agent_sdk", "gpt-5.6-sol").is_none());
    assert!(vision_destination("openai_codex_subscription", "gpt-5.6-sol").is_some());
    assert!(vision_destination("openai_codex_subscription", "claude-code/sonnet").is_none());
    assert!(vision_destination("opencode_go", "opencode/kimi").is_none());
    assert_eq!(model_label("claude-code/opus"), "Claude (opus)");
    assert_eq!(model_label("gpt-5.6-sol"), "gpt-5.6-sol");
}

#[test]
fn spoken_answers_map_to_offered_buttons_only() {
    use Choice::*;
    let step = [DoIt, ShowMe, Stop];
    assert_eq!(choice_from_speech("Do it", &step), Some(DoIt));
    assert_eq!(choice_from_speech("yes please", &step), Some(DoIt));
    assert_eq!(choice_from_speech("Show me how.", &step), Some(ShowMe));
    assert_eq!(choice_from_speech("I'll do it myself", &step), Some(ShowMe));
    // Stop words win, so a negated "do it" never acts.
    assert_eq!(choice_from_speech("don't do it", &step), Some(Stop));
    assert_eq!(choice_from_speech("no", &step), Some(Stop));
    assert_eq!(choice_from_speech("what is this", &step), None);
    // Buttons not on the card are never pressed.
    let show_only = [ShowMe, Stop];
    assert_eq!(choice_from_speech("do it", &show_only), None);
    assert_eq!(choice_from_speech("yes", &show_only), Some(ShowMe));
    assert_eq!(choice_from_speech("ok done", &[Done, Stop]), Some(Done));
    assert_eq!(
        choice_from_speech("sure, take a look", &[Look, Stop]),
        Some(Look)
    );
    assert_eq!(choice_from_speech("thanks!", &[Close]), Some(Close));
    assert_eq!(choice_from_speech("cancel", &[Close]), Some(Close));
    assert_eq!(choice_from_speech("yes", &[Stop]), None);
}

struct RefusingHost(&'static str);
#[async_trait]
impl crate::presence::LookHost for RefusingHost {
    async fn capture(&self, _: u32, _: &Cancellation) -> Result<Vec<u8>, String> {
        Err(format!("host capture failed: {}", self.0))
    }
    async fn present(&self, _: Value) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn a_failed_look_says_why_instead_of_computer_stopped() {
    for (reason, expected) in [
        (
            "screen_recording_permission_required",
            "quit Astrid Presence",
        ),
        ("frame_png_exceeds_1mib", "1 MiB frame limit"),
        ("something_new", "The look failed (local_capture_failed)."),
    ] {
        let (origin, seen) = mock_model(vec![]).await;
        let dir = tempfile::tempdir().unwrap();
        let tools = Tools::new(Workspace::new(dir.path()).unwrap(), Duration::from_secs(1))
            .unwrap()
            .with_computer(
                Computer::local(
                    "test".into(),
                    0,
                    Arc::new(RefusingHost(reason)),
                    Arc::new(DecodeWorker::inline()),
                )
                .unwrap(),
            );
        let provider = OpenAiProvider::with_endpoint(Arc::new(Auth), origin).unwrap();
        let mut eyes = RuntimeEyes {
            provider: &provider,
            tools: &tools,
            model: "gpt-5.6-sol".into(),
        };
        let ui = FakeUi {
            choices: Mutex::new(vec![].into()),
            ..FakeUi::default()
        };
        let outcome = walk("x", Mode::Do, 0, &mut eyes, &ui, &Cancellation::default()).await;
        let Outcome::Failed(message) = outcome else {
            panic!("{outcome:?}")
        };
        assert!(message.contains(expected), "{reason}: {message}");
        assert!(!message.contains("computer_stopped"));
        // The model asked for the look once and never got an image.
        assert_eq!(seen.lock().unwrap().len(), 1);
    }
}

/// A real-size, real-weight look: a 1512x982 frame with photo-like content
/// (gradients and noise), quantized as the presence host does, close to the
/// 1 MiB cap. Synthetic 320x200 frames hid every size limit on the way.
fn screen_png() -> Vec<u8> {
    let (w, h) = (1512u32, 982u32);
    let mut seed: u64 = 11;
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            for c in 0..3u32 {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let noise = (seed >> 61) as i32 - 4;
                let base = ((x * (c + 1) + y * (3 - c)) % 256) as i32;
                rgb.push(((base + noise).clamp(0, 255) as u8) & 0xF0);
            }
        }
    }
    let mut png_bytes = Vec::new();
    {
        let mut e = png::Encoder::new(&mut png_bytes, w, h);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(&rgb).unwrap();
    }
    png_bytes
}

struct ScreenHost(Vec<u8>);
#[async_trait]
impl crate::presence::LookHost for ScreenHost {
    async fn capture(&self, _: u32, _: &Cancellation) -> Result<Vec<u8>, String> {
        Ok(self.0.clone())
    }
    async fn present(&self, _: Value) -> Result<(), String> {
        Ok(())
    }
}

fn screen_tools(dir: &std::path::Path, frame: &[u8]) -> Tools {
    Tools::new(Workspace::new(dir).unwrap(), Duration::from_secs(1))
        .unwrap()
        .with_computer(
            Computer::local(
                "test".into(),
                0,
                Arc::new(ScreenHost(frame.to_vec())),
                Arc::new(DecodeWorker::inline()),
            )
            .unwrap(),
        )
}

#[tokio::test]
async fn a_full_size_real_screen_gets_through_on_both_providers() {
    let frame = screen_png();
    assert!(
        (600_000..=crate::computer_png::MAX_PNG).contains(&frame.len()),
        "fixture should be near the cap: {}",
        frame.len()
    );
    let report = block(json!({"step":step(1, 1, 1400, "click", json!({"kind":"click"}))}));

    // OpenAI path.
    let (origin, seen) = mock_model(vec![report.clone()]).await;
    let dir = tempfile::tempdir().unwrap();
    let tools = screen_tools(dir.path(), &frame);
    let provider = OpenAiProvider::with_endpoint(Arc::new(Auth), origin).unwrap();
    let mut eyes = RuntimeEyes {
        provider: &provider,
        tools: &tools,
        model: "gpt-5.6-sol".into(),
    };
    let ui = FakeUi {
        choices: Mutex::new(vec![Choice::Stop].into()),
        ..FakeUi::default()
    };
    let outcome = walk(
        "Like this video",
        Mode::Guide,
        0,
        &mut eyes,
        &ui,
        &Cancellation::default(),
    )
    .await;
    assert!(
        matches!(outcome, Outcome::Stopped(_)),
        "openai: {outcome:?}"
    );
    assert_eq!(
        ui.cards.lock().unwrap()[0].step,
        Some((1, 1)),
        "openai reached step 1"
    );
    assert_eq!(seen.lock().unwrap().len(), 2);
    // The production worker re-encodes frames; the result must stay in bounds.
    assert!(
        crate::computer_png::canonicalize(&frame)
            .unwrap()
            .bytes
            .len()
            <= crate::computer_png::MAX_PNG
    );

    // Claude Agent SDK path, through a fake bridge.
    use crate::claude_code::{Bridge, ClaudeCodeProvider};
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("requests");
    std::fs::create_dir(&log).unwrap();
    // Claude answers in the scaled image's pixels (1330x864 for this screen).
    let claude_report = block(json!({"step":step(1, 1, 1000, "click", json!({"kind":"click"}))}));
    std::fs::write(
        dir.path().join("report.json"),
        json!({"type":"done","text":claude_report,"tool_calls":[]}).to_string(),
    )
    .unwrap();
    let script = format!(
        r#"n=$(ls {log} | wc -l | tr -d ' '); cat > {log}/$n.json
if grep -q '"image":{{' {log}/$n.json; then cat {dir}/report.json; echo
else printf '%s\n' '{{"type":"done","text":"","tool_calls":[{{"id":"toolu_look","name":"look","arguments":"{{}}"}}]}}'; fi"#,
        log = log.display(),
        dir = dir.path().display()
    );
    let provider = ClaudeCodeProvider::new(
        Bridge {
            command: vec!["/bin/sh".into(), "-c".into(), script],
        },
        dir.path().to_path_buf(),
    );
    let tools = screen_tools(dir.path(), &frame);
    let mut eyes = RuntimeEyes {
        provider: &provider,
        tools: &tools,
        model: "claude-code/sonnet".into(),
    };
    let ui = FakeUi {
        choices: Mutex::new(vec![Choice::Stop].into()),
        ..FakeUi::default()
    };
    let outcome = walk(
        "Like this video",
        Mode::Guide,
        0,
        &mut eyes,
        &ui,
        &Cancellation::default(),
    )
    .await;
    assert!(
        matches!(outcome, Outcome::Stopped(_)),
        "claude: {outcome:?}"
    );
    assert_eq!(
        ui.cards.lock().unwrap()[0].step,
        Some((1, 1)),
        "claude reached step 1"
    );
    let sent: Value = serde_json::from_slice(&std::fs::read(log.join("1.json")).unwrap()).unwrap();
    let text = sent["image"]["text"].as_str().unwrap();
    assert!(
        text.contains("1512x982") && text.contains("scaled to 1330x864"),
        "{text}"
    );
    use base64::Engine;
    let scaled = base64::engine::general_purpose::STANDARD
        .decode(sent["image"]["data"].as_str().unwrap())
        .unwrap();
    let decoder = png::Decoder::new(std::io::Cursor::new(&scaled[..]));
    let reader = decoder.read_info().unwrap();
    assert_eq!((reader.info().width, reader.info().height), (1330, 864));
    // Its point maps back to display points: 1000 * 1512 / 1330 = 1136.
    let events = ui.events.lock().unwrap().clone();
    assert!(
        events.contains(
            &json!({"v":1,"type":"point_at","x":1136,"y":22,"display":0,"cursor":"click"})
        ),
        "{events:?}"
    );
    assert_eq!(crate::claude_code::claude_view(1280, 800), (1280, 800));
    // The bridge got the whole (scaled) frame, within its 8 MiB request limit.
    let request = std::fs::metadata(log.join("1.json")).unwrap().len();
    assert!(request > 200_000 && request < 8 * 1024 * 1024, "{request}");
}
