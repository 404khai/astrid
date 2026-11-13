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
    act_fails: bool,
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
        if self.act_fails {
            Err("the screen changed".into())
        } else {
            Ok(())
        }
    }
    fn state(&self, _: &str, _: &str) {}
}

async fn session(
    reports: Vec<String>,
    choices: Vec<Choice>,
    act_fails: bool,
) -> (Outcome, FakeUi, usize, Vec<Value>) {
    let (origin, seen) = mock_model(reports).await;
    let host = Arc::new(FakeHost::default());
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
        act_fails,
        ..FakeUi::default()
    };
    let outcome = walk(
        "Export the report",
        "gpt-5.6-sol",
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
async fn a_walkthrough_retries_an_unverified_step_then_does_one_for_the_user() {
    let reports = vec![
        block(json!({"step":step(1, 2, 300, "click", json!({"kind":"click"}))})),
        block(
            json!({"verified":false,"reason":"The menu is still closed.","step":step(1, 2, 301, "click", json!({"kind":"click"}))}),
        ),
        block(
            json!({"verified":true,"step":step(2, 2, 100, "text", json!({"kind":"type","text":"Q3"}))}),
        ),
        block(json!({"verified":true,"finish":"Exported."})),
    ];
    let choices = vec![
        Choice::Look,
        Choice::ShowMe,
        Choice::Done,
        Choice::ShowMe,
        Choice::Done,
        Choice::DoIt,
        Choice::Close,
    ];
    let (outcome, ui, captures, requests) = session(reports, choices, false).await;
    assert_eq!(outcome, Outcome::Completed("Exported.".into()));
    // One look per press: start, Done, Done, Do it.
    assert_eq!(captures, 4);
    assert_eq!(requests.len(), 8);
    // Every model request after a look carries the pending step's check.
    assert!(requests[2].to_string().contains("Pending step 1 of 2"));
    let events = ui.events.lock().unwrap().clone();
    // The wave plays once, as soon as the first look is granted.
    assert_eq!(events[0], json!({"v":1,"type":"wave"}));
    assert_eq!(events.iter().filter(|e| e["type"] == "wave").count(), 1);
    // Show me points with the step's cursor and circles the target.
    assert!(
        events.contains(
            &json!({"v":1,"type":"point_at","x":300,"y":20,"display":0,"cursor":"click"})
        )
    );
    assert!(
        events
            .contains(&json!({"v":1,"type":"point_at","x":100,"y":20,"display":0,"cursor":"text"}))
    );
    assert!(events.iter().any(|e| e["type"] == "sketch"));
    // Do it acted exactly once, on step 2 as the card named it.
    let acts = ui.acts.lock().unwrap().clone();
    assert_eq!(acts.len(), 1);
    assert_eq!(acts[0].action, Some(Action::Type("Q3".into())));
    let cards = ui.cards.lock().unwrap().clone();
    assert!(
        cards[0].text.contains("gpt-5.6-sol")
            && cards[0].buttons == vec![Choice::Look, Choice::Stop]
    );
    assert_eq!(cards[1].step, Some((1, 2)));
    assert_eq!(
        cards[1].buttons,
        vec![Choice::DoIt, Choice::ShowMe, Choice::Stop]
    );
    assert!(cards[1].text.contains("then look again to check"));
    assert!(
        cards[3]
            .text
            .starts_with("Not yet: The menu is still closed.")
    );
    assert!(cards[5].text.contains("type “Q3”"));
    assert_eq!(cards.last().unwrap().text, "Exported.");
}

#[tokio::test]
async fn declining_the_first_look_captures_and_sends_nothing() {
    let (outcome, ui, captures, requests) = session(vec![], vec![Choice::Stop], false).await;
    assert!(matches!(outcome, Outcome::Stopped(_)));
    assert_eq!((captures, requests.len()), (0, 0));
    assert!(
        ui.events.lock().unwrap().is_empty(),
        "no wave without a granted look"
    );
}

#[tokio::test]
async fn a_refused_action_shows_the_step_and_looks_only_after_done() {
    let reports = vec![block(
        json!({"step":step(1, 1, 300, "press", json!({"kind":"click"}))}),
    )];
    let (outcome, ui, captures, _) = session(
        reports,
        vec![Choice::Look, Choice::DoIt, Choice::Stop],
        true,
    )
    .await;
    assert!(matches!(outcome, Outcome::Stopped(_)));
    assert_eq!(captures, 1);
    let cards = ui.cards.lock().unwrap().clone();
    assert!(
        cards[2]
            .text
            .starts_with("I couldn't do that (the screen changed)")
    );
    assert_eq!(cards[2].buttons, vec![Choice::Done, Choice::Stop]);
}
