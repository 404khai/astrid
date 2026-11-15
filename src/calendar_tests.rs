use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A fake presence host: answers like `CalendarService` and records requests.
#[derive(Default)]
struct Host {
    requests: Mutex<Vec<Value>>,
    reply: Mutex<Option<Result<Value, String>>>,
}
#[async_trait]
impl CalendarHost for Host {
    async fn call(&self, request: Value, _: &Cancellation) -> Result<Value, String> {
        self.requests.lock().unwrap().push(request.clone());
        if let Some(reply) = self.reply.lock().unwrap().take() {
            return reply;
        }
        match request["op"].as_str() {
            Some("agenda") => Ok(
                json!({"events":[event("EK-1@1", "Design review", 15, true)],
                "reminders":[{"id":"R1","title":"Call mum","due":null,"list":"Reminders","place":null}],"truncated":false}),
            ),
            Some("reminder_create") => Ok(json!({"reminder":{"id":"R2","title":request["title"],
                "due":request["due"],"list":"Reminders","place":request["place"]}})),
            Some("event_move") => Ok(json!({"event":{"id":"EK-1@2","title":"Design review",
                "start":request["start"],"end":request["end"],"all_day":false,"calendar":"Work",
                "location":null,"has_meeting_link":true,"editable":true}})),
            _ => Err("invalid_request".into()),
        }
    }
}

/// Fixed day for unit tests: Monday 2026-11-16, 08:00 at UTC-8.
fn monday() -> Time {
    Time::parse("2026-11-16T08:00:00-08:00").unwrap()
}

fn event(id: &str, title: &str, hour: u32, editable: bool) -> Value {
    json!({"id":id,"title":title,"start":format!("2026-11-16T{hour}:00:00-08:00"),
        "end":format!("2026-11-16T{}:00:00-08:00", hour + 1),"all_day":false,"calendar":"Work",
        "location":"Room 4","has_meeting_link":true,"editable":editable,
        "notes":"secret dial-in","meeting_url":"https://zoom.us/j/1"})
}

#[test]
fn times_parse_format_and_read_aloud() {
    let t = Time::parse("2026-11-16T15:00:00-08:00").unwrap();
    assert_eq!(t.secs, Time::parse("2026-11-16T23:00:00Z").unwrap().secs);
    assert_eq!(t.iso(), "2026-11-16T15:00:00-08:00");
    assert_eq!(t.clock(), "3pm");
    assert_eq!(
        Time::parse("2026-11-16T16:30:00.250+05:30")
            .unwrap()
            .clock(),
        "4:30pm"
    );
    assert_eq!(Time::parse("2026-11-16T12:00:00Z").unwrap().clock(), "noon");
    assert_eq!(
        Time::parse("2026-11-16T09:05:00Z").unwrap().clock(),
        "9:05am"
    );
    let now = monday();
    assert_eq!(t.day_phrase(&now), "today");
    assert_eq!(
        Time::parse("2026-11-17T09:00:00-08:00")
            .unwrap()
            .day_phrase(&now),
        "tomorrow"
    );
    assert_eq!(
        Time::parse("2026-11-19T09:00:00-08:00")
            .unwrap()
            .day_phrase(&now),
        "on Thursday"
    );
    assert_eq!(
        Time::parse("2026-12-01T09:00:00-08:00")
            .unwrap()
            .day_phrase(&now),
        "on Dec 1"
    );
    assert!(now_line(&now).contains("Monday"));
    for bad in [
        "2026-11-16",
        "2026-13-01T00:00:00Z",
        "2026-11-16T25:00:00Z",
        "2026-11-16T10:00:00",
        "soon",
    ] {
        assert_eq!(Time::parse(bad), None, "{bad}");
    }
}

#[test]
fn agenda_covers_whole_local_days_within_bounds() {
    let calendar = Calendar::new(Arc::new(Host::default()));
    let now = monday();
    let request = calendar
        .prepare(
            "calendar_agenda",
            r#"{"start_date":null,"days":null}"#,
            &now,
        )
        .unwrap();
    assert!(!request.is_write());
    let Op::Agenda { from, to } = request.op else {
        panic!()
    };
    assert_eq!(to.secs - from.secs, 86_400);
    assert_eq!(from.day(), now.day());
    for bad in [
        r#"{"start_date":null,"days":15}"#,
        r#"{"start_date":"tomorrow","days":1}"#,
        r#"{"start_date":null,"days":1,"calendar":"Work"}"#,
    ] {
        assert!(
            calendar.prepare("calendar_agenda", bad, &now).is_err(),
            "{bad}"
        );
    }
    assert!(calendar.prepare("event_delete", "{}", &now).is_err());
}

#[test]
fn reminders_ask_in_plain_words_and_reject_bad_times() {
    let calendar = Calendar::new(Arc::new(Host::default()));
    let now = monday();
    let home = calendar
        .prepare(
            "reminder_create",
            r#"{"title":"Water the plants","due":null,"when_arriving":"home"}"#,
            &now,
        )
        .unwrap();
    assert_eq!(
        home.question.as_deref(),
        Some("Can I remind you to water the plants when you get home?")
    );
    let timed = calendar
        .prepare(
            "reminder_create",
            r#"{"title":"Call Ada","due":"2026-11-16T18:00:00-08:00","when_arriving":null}"#,
            &now,
        )
        .unwrap();
    assert_eq!(
        timed.question.as_deref(),
        Some("Can I remind you to call Ada today at 6pm?")
    );
    assert!(timed.description().contains("calendar.write"));
    for bad in [
        r#"{"title":"","due":null,"when_arriving":null}"#,
        r#"{"title":"x","due":"2026-11-15T18:00:00-08:00","when_arriving":null}"#,
        r#"{"title":"x","due":"2028-11-15T18:00:00-08:00","when_arriving":null}"#,
        r#"{"title":"x","due":null,"when_arriving":"work"}"#,
        r#"{"title":"x","due":"6pm","when_arriving":null}"#,
    ] {
        assert!(
            calendar.prepare("reminder_create", bad, &now).is_err(),
            "{bad}"
        );
    }
}

#[tokio::test]
async fn moving_needs_an_event_the_model_has_read_and_names_it_in_the_question() {
    let host = Arc::new(Host::default());
    let calendar = Calendar::new(host.clone());
    let now = monday();
    let move_args = r#"{"event_id":"e1","start":"2026-11-16T16:00:00-08:00","end":null}"#;
    assert!(
        calendar
            .prepare("event_move", move_args, &now)
            .unwrap_err()
            .contains("calendar_agenda")
    );

    let agenda = calendar
        .prepare(
            "calendar_agenda",
            r#"{"start_date":"2026-11-16","days":1}"#,
            &now,
        )
        .unwrap();
    let seen = calendar
        .execute(&agenda, &Cancellation::default())
        .await
        .unwrap();
    let event = &seen["events"][0];
    assert_eq!(event["id"], "e1");
    assert_eq!(event["can_move"], true);
    // Notes, links and the host's own ids never reach the model.
    let text = seen.to_string();
    assert!(!text.contains("secret") && !text.contains("zoom.us") && !text.contains("EK-1"));
    assert_eq!(seen["reminders"][0]["title"], "Call mum");

    let request = calendar.prepare("event_move", move_args, &now).unwrap();
    assert_eq!(
        request.question.as_deref(),
        Some("Can I move your 3pm Design review to 4pm?")
    );
    let Op::Move {
        ref start, ref end, ..
    } = request.op
    else {
        panic!()
    };
    assert_eq!(end.secs - start.secs, 3600, "length is kept");
    let tomorrow = calendar
        .prepare(
            "event_move",
            r#"{"event_id":"e1","start":"2026-11-17T10:00:00-08:00","end":null}"#,
            &now,
        )
        .unwrap();
    assert_eq!(
        tomorrow.question.as_deref(),
        Some("Can I move your 3pm Design review to 10am tomorrow?")
    );
    assert!(calendar
        .prepare("event_move", r#"{"event_id":"e1","start":"2026-11-16T16:00:00-08:00","end":"2026-11-18T16:00:00-08:00"}"#, &now)
        .is_err());

    let moved = calendar
        .execute(&request, &Cancellation::default())
        .await
        .unwrap();
    assert_eq!(
        moved["moved"]["id"], "e1",
        "the same short id after the move"
    );
    let sent = host.requests.lock().unwrap().last().cloned().unwrap();
    assert_eq!(sent["event_id"], "EK-1@1");
    assert_eq!(sent["expect_start"], "2026-11-16T15:00:00-08:00");
}

#[tokio::test]
async fn read_only_events_are_refused_and_host_reasons_are_explained() {
    let host = Arc::new(Host::default());
    *host.reply.lock().unwrap() = Some(Ok(
        json!({"events":[event("H@1","Holiday",9,false)],"reminders":[]}),
    ));
    let calendar = Calendar::new(host.clone());
    let now = monday();
    let agenda = calendar
        .prepare("calendar_agenda", r#"{"start_date":null,"days":1}"#, &now)
        .unwrap();
    calendar
        .execute(&agenda, &Cancellation::default())
        .await
        .unwrap();
    assert!(
        calendar
            .prepare(
                "event_move",
                r#"{"event_id":"e1","start":"2026-11-16T11:00:00-08:00","end":null}"#,
                &now
            )
            .unwrap_err()
            .contains("read-only")
    );
    *host.reply.lock().unwrap() = Some(Err("home_address_unavailable".into()));
    let home = calendar
        .prepare(
            "reminder_create",
            r#"{"title":"Water plants","due":null,"when_arriving":"home"}"#,
            &now,
        )
        .unwrap();
    let error = calendar
        .execute(&home, &Cancellation::default())
        .await
        .unwrap_err();
    assert!(error.contains("ask what time"));
}

mod runtime_flow {
    use super::*;
    use crate::{
        auth::{AuthError, Authentication, BearerToken},
        model::{Message as Msg, ToolOutcome},
        openai::OpenAiProvider,
        runtime::{self, PermissionHandler, RunConfig, RunOutcome},
        tools::{PermissionRequest, Tools},
        workspace::Workspace,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct Auth;
    #[async_trait]
    impl Authentication for Auth {
        async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
            BearerToken::new("test-only".into())
        }
    }
    struct Answers(bool, Vec<String>);
    #[async_trait]
    impl PermissionHandler for Answers {
        async fn decide(&mut self, request: &PermissionRequest) -> std::io::Result<bool> {
            self.1.push(request.command.clone());
            Ok(self.0)
        }
    }

    /// A Responses API mock: agenda, then the given write, then a reply.
    async fn provider(write: Value) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut raw = vec![];
                let mut b = [0u8; 8192];
                loop {
                    let k = socket.read(&mut b).await.unwrap();
                    raw.extend_from_slice(&b[..k]);
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
                            break;
                        }
                    }
                }
                let call = |id: &str, name: &str, args: &Value| {
                    json!([{"type":"function_call","id":format!("i{id}"),"call_id":id,"name":name,
                        "namespace":"astrid","arguments":args.to_string(),"status":"completed"}])
                };
                let output = match calls.fetch_add(1, Ordering::SeqCst) {
                    0 => call(
                        "c1",
                        "calendar_agenda",
                        &json!({"start_date":null,"days":1}),
                    ),
                    1 => call("c2", write["name"].as_str().unwrap(), &write["arguments"]),
                    _ => json!([{"type":"message","id":"m","role":"assistant","status":"completed",
                        "content":[{"type":"output_text","text":"done","annotations":[]}]}]),
                };
                let sse = format!(
                    "data: {}\n\n",
                    json!({"type":"response.completed","response":{"id":"r","status":"completed","output":output}})
                );
                let wire = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{sse}",
                    sse.len()
                );
                socket.write_all(wire.as_bytes()).await.unwrap();
            }
        });
        origin
    }

    /// The fake host's event sits tomorrow at 3pm local, so writes are in range.
    struct Tomorrow(Host, String);
    #[async_trait]
    impl CalendarHost for Tomorrow {
        async fn call(&self, request: Value, cancel: &Cancellation) -> Result<Value, String> {
            if request["op"] == "agenda" {
                self.0.requests.lock().unwrap().push(request);
                let start = Time::parse(&self.1).unwrap();
                let end = Time::at(start.secs + 3600, start.offset_min);
                return Ok(
                    json!({"events":[{"id":"EK-9@1","title":"Design review","start":start.iso(),
                    "end":end.iso(),"all_day":false,"calendar":"Work","location":null,
                    "has_meeting_link":false,"editable":true}],"reminders":[]}),
                );
            }
            self.0.call(request, cancel).await
        }
    }

    async fn run(
        approve: bool,
        write: Value,
        start: String,
    ) -> (runtime::RunResult, Answers, Arc<Tomorrow>) {
        let origin = provider(write).await;
        let host = Arc::new(Tomorrow(Host::default(), start));
        let workspace = tempfile::tempdir().unwrap();
        let tools = Tools::new(
            Workspace::new(workspace.path()).unwrap(),
            Duration::from_secs(1),
        )
        .unwrap()
        .with_calendar(Calendar::new(host.clone()));
        let provider =
            OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{origin}/v1/responses"))
                .unwrap();
        let mut answers = Answers(approve, vec![]);
        let config = RunConfig {
            routing: None,
            observability: None,
            context_budget: Some(Default::default()),
            model: "gpt-5.6-sol".into(),
            task: "move my 3pm".into(),
            max_model_calls: 4,
            permissions: crate::permissions::PermissionPolicy::read_only(),
        };
        let result = runtime::run(
            &provider,
            &tools,
            &mut answers,
            config,
            Cancellation::default(),
            None,
        )
        .await
        .unwrap();
        (result, answers, host)
    }

    fn outcome(result: &runtime::RunResult, id: &str) -> ToolOutcome {
        result
            .session
            .messages
            .iter()
            .find_map(|m| match m {
                Msg::Tool(t) if t.call_id == id => Some(t.outcome.clone()),
                _ => None,
            })
            .unwrap()
    }

    fn tomorrow_at(hour: i64) -> String {
        let now = now();
        let day = Time::at(now.secs + 86_400, now.offset_min).day();
        let secs = day * 86_400 + hour * 3600 - i64::from(now.offset_min) * 60;
        Time::at(secs, now.offset_min).iso()
    }

    fn move_to_4pm() -> Value {
        json!({"name":"event_move","arguments":{"event_id":"e1","start":tomorrow_at(16),"end":null}})
    }

    #[tokio::test]
    async fn reading_needs_no_prompt_and_a_move_runs_only_after_go_ahead() {
        let (result, answers, host) = run(true, move_to_4pm(), tomorrow_at(15)).await;
        assert_eq!(result.outcome, RunOutcome::Completed);
        assert_eq!(answers.1.len(), 1, "one question, for the write only");
        assert!(
            answers.1[0].contains("Can I move your 3pm Design review tomorrow to 4pm?"),
            "{}",
            answers.1[0]
        );
        let ops: Vec<String> = host
            .0
            .requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| r["op"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(ops, ["agenda", "event_move"]);
        assert!(matches!(
            outcome(&result, "c2"),
            ToolOutcome::Success { .. }
        ));
    }

    #[tokio::test]
    async fn stop_leaves_the_calendar_untouched() {
        let (result, answers, host) = run(false, move_to_4pm(), tomorrow_at(15)).await;
        assert_eq!(answers.1.len(), 1);
        let ops: Vec<String> = host
            .0
            .requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| r["op"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(ops, ["agenda"], "a declined move never reaches the host");
        assert!(
            matches!(outcome(&result, "c2"), ToolOutcome::Error { code, .. } if code == "permission_denied")
        );
    }

    #[tokio::test]
    async fn a_reminder_is_one_question_then_one_create() {
        let write = json!({"name":"reminder_create","arguments":{"title":"Buy milk","due":null,"when_arriving":"home"}});
        let (result, answers, host) = run(true, write, tomorrow_at(15)).await;
        assert_eq!(result.outcome, RunOutcome::Completed);
        assert!(answers.1[0].contains("Can I remind you to buy milk when you get home?"));
        let sent = host.0.requests.lock().unwrap().last().cloned().unwrap();
        assert_eq!(sent["op"], "reminder_create");
        assert_eq!(sent["place"], "home");
    }
}
