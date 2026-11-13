use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn emlx(message: &str) -> Vec<u8> {
    format!(
        "{}\n{message}<?xml version=\"1.0\"?><plist/>",
        message.len()
    )
    .into_bytes()
}

const MULTIPART: &str = "From: =?UTF-8?B?w4lsb2RpZQ==?= <elodie@example.com>\r\nTo: me@example.com\r\nSubject: =?utf-8?Q?Flight_to_Lagos_=E2=9C=88?= =?utf-8?Q?_confirmed?=\r\nDate: Tue, 10 Nov 2026 09:14:00 +0000\r\nContent-Type: multipart/mixed; boundary=\"b1\"\r\n\r\n--b1\r\nContent-Type: multipart/alternative; boundary=\"b2\"\r\n\r\n--b2\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\nYour booking ref is QX7=\r\n42. Seat 14A.\r\n--b2\r\nContent-Type: text/html\r\n\r\n<p>HTML version</p>\r\n--b2--\r\n--b1\r\nContent-Type: application/pdf; name=\"ticket.pdf\"\r\nContent-Disposition: attachment; filename=\"ticket.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0=\r\n--b1--\r\n";

#[test]
fn emlx_multipart_decodes_headers_prefers_plain_and_lists_attachments() {
    let m = parse_emlx(&emlx(MULTIPART), "INBOX.mbox/1.emlx".into()).unwrap();
    assert_eq!(m.summary.from, "Élodie <elodie@example.com>");
    assert_eq!(m.summary.subject, "Flight to Lagos ✈ confirmed");
    assert_eq!(m.summary.date, "Tue, 10 Nov 2026 09:14:00 +0000");
    assert_eq!(m.to, "me@example.com");
    assert_eq!(m.body, "Your booking ref is QX742. Seat 14A.");
    assert_eq!(m.attachments, vec!["ticket.pdf"]);
    assert!(!m.truncated);
    assert_eq!(m.summary.snippet, "Your booking ref is QX742. Seat 14A.");
}

#[test]
fn html_only_and_base64_bodies_become_text_and_long_bodies_are_clipped() {
    let html = "Subject: hi\nContent-Type: text/html\n\n<style>p{}</style><p>Hello&nbsp;<b>there</b></p><p>Line &amp; two</p>";
    let m = parse_rfc822(html.as_bytes(), "x".into()).unwrap();
    assert_eq!(m.body, "Hello there\n\nLine & two");
    let encoded = STANDARD.encode("a".repeat(MAX_BODY + 50));
    let big = format!("Subject: big\nContent-Transfer-Encoding: base64\n\n{encoded}");
    let m = parse_rfc822(big.as_bytes(), "y".into()).unwrap();
    assert_eq!(m.body.len(), MAX_BODY);
    assert!(m.truncated);
    assert_eq!(m.summary.snippet.chars().count(), MAX_SNIPPET);
    assert!(parse_emlx(b"not a number\nFrom: x", "z".into()).is_err());
}

#[test]
fn encoded_words_keep_plain_text_and_spacing() {
    assert_eq!(
        decode_words("Re: =?UTF-8?Q?caf=C3=A9?= tomorrow"),
        "Re: café tomorrow"
    );
    assert_eq!(
        decode_words("=?utf-8?B?SGk=?= =?utf-8?B?IHRoZXJl?="),
        "Hi there"
    );
    assert_eq!(decode_words("broken =?x"), "broken =?x");
}

struct WalkFinder;
#[async_trait]
impl Finder for WalkFinder {
    async fn find(&self, root: &Path, query: &str, limit: usize) -> Result<Vec<PathBuf>, String> {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk(&p, out)
                } else if p.extension().is_some_and(|e| e == "emlx") {
                    out.push(p)
                }
            }
        }
        let mut all = vec![];
        walk(root, &mut all);
        all.sort();
        let q = query.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|p| {
                String::from_utf8_lossy(&std::fs::read(p).unwrap())
                    .to_lowercase()
                    .contains(&q)
            })
            .take(limit)
            .collect())
    }
}

#[tokio::test]
async fn apple_mail_searches_reads_and_confines_ids_to_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let mbox = dir.path().join("V10/INBOX.mbox/Messages");
    std::fs::create_dir_all(&mbox).unwrap();
    std::fs::write(mbox.join("1.emlx"), emlx(MULTIPART)).unwrap();
    std::fs::write(mbox.join("2.emlx"), emlx("Subject: Dentist\n\nTuesday 3pm")).unwrap();
    std::fs::write(
        dir.path().join("outside.emlx"),
        emlx("Subject: secret\n\nx"),
    )
    .unwrap();
    let source = AppleMail::new(dir.path().join("V10"), Arc::new(WalkFinder));
    let cancel = Cancellation::default();
    let hits = source.search("booking", 10, &cancel).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "INBOX.mbox/Messages/1.emlx");
    let m = source.read(&hits[0].id, &cancel).await.unwrap();
    assert!(m.body.contains("QX742"));
    for bad in [
        "../outside.emlx",
        "/etc/passwd",
        "INBOX.mbox/Messages/1.txt",
        "INBOX.mbox/../../outside.emlx",
    ] {
        assert!(source.read(bad, &cancel).await.is_err(), "{bad}");
    }
}

struct Fixed;
#[async_trait]
impl MailSource for Fixed {
    fn name(&self) -> &'static str {
        "apple"
    }
    async fn search(
        &self,
        q: &str,
        limit: usize,
        _: &Cancellation,
    ) -> Result<Vec<Summary>, String> {
        Ok((0..30)
            .map(|i| Summary {
                id: format!("{i}"),
                from: "a".into(),
                subject: q.into(),
                date: "d".into(),
                snippet: "s".into(),
            })
            .take(limit + 5)
            .collect())
    }
    async fn read(&self, _: &str, _: &Cancellation) -> Result<Message, String> {
        Err("nope".into())
    }
}

#[tokio::test]
async fn tool_arguments_are_validated_and_results_bounded() {
    let mail = Mail::new(vec![Arc::new(Fixed)]).unwrap();
    assert!(Mail::new(vec![]).is_err());
    let ok = mail
        .prepare(
            "mail_search",
            r#"{"source":"apple","query":"flight","limit":3}"#,
        )
        .unwrap();
    assert_eq!(
        ok.op,
        Op::Search {
            query: "flight".into(),
            limit: 3
        }
    );
    let description = ok.description();
    assert!(
        description.contains("mail.read_only")
            && description.contains("flight")
            && description.contains("cannot send")
    );
    let result = mail.execute(&ok, &Cancellation::default()).await.unwrap();
    assert_eq!(result["results"].as_array().unwrap().len(), 3);
    for (tool, args) in [
        ("mail_search", r#"{"source":"gmail","query":"x"}"#),
        ("mail_search", r#"{"source":"apple","query":""}"#),
        (
            "mail_search",
            r#"{"source":"apple","query":"x","limit":21}"#,
        ),
        (
            "mail_search",
            r#"{"source":"apple","query":"x","send":true}"#,
        ),
        ("mail_read", r#"{"source":"apple"}"#),
        ("mail_send", r#"{"source":"apple","id":"1"}"#),
    ] {
        assert!(mail.prepare(tool, args).is_err(), "{tool} {args}");
    }
    let defs = definitions(&mail.source_names());
    assert_eq!(defs.len(), 2);
    assert!(defs[0].to_string().contains("\"enum\":[\"apple\"]"));
}

#[test]
fn client_config_reads_google_desktop_json() {
    let c = ClientConfig::parse(r#"{"installed":{"client_id":"id.apps.googleusercontent.com","client_secret":"s","redirect_uris":["http://localhost"]}}"#).unwrap();
    assert_eq!(c.client_id, "id.apps.googleusercontent.com");
    assert!(ClientConfig::parse(r#"{"installed":{"client_id":""}}"#).is_err());
}

#[test]
fn sign_in_url_requests_only_gmail_readonly_with_pkce_and_checks_state() {
    let (verifier, challenge) = login::pkce();
    assert_eq!(verifier.len(), 64);
    assert_eq!(
        challenge,
        URL_SAFE_NO_PAD.encode(<sha2::Sha256 as sha2::Digest>::digest(verifier.as_bytes()))
    );
    let client = ClientConfig {
        client_id: "cid".into(),
        client_secret: "cs".into(),
    };
    let url = login::authorize_url(
        &Endpoints::default(),
        &client,
        "http://127.0.0.1:5000/",
        &challenge,
        "st8",
    );
    assert!(url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fgmail.readonly&"));
    assert!(url.contains("code_challenge_method=S256") && url.contains("state=st8"));
    assert!(
        !url.contains("cs"),
        "client secret never goes in the browser URL"
    );
    assert_eq!(
        login::parse_redirect("GET /?state=st8&code=abc HTTP/1.1", "st8").unwrap(),
        "abc"
    );
    assert!(login::parse_redirect("GET /?state=evil&code=abc HTTP/1.1", "st8").is_err());
    assert!(login::parse_redirect("GET /?error=access_denied&state=st8 HTTP/1.1", "st8").is_err());
}

/// Minimal local HTTP server: replies by path prefix.
async fn mock(
    reply: fn(&str, &str) -> (u16, String),
) -> (String, Arc<std::sync::Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(std::sync::Mutex::new(vec![]));
    let log = seen.clone();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0; 16384];
            let n = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..n]).into_owned();
            let line = request.lines().next().unwrap_or("").to_owned();
            log.lock().unwrap().push(request.clone());
            let (status, body) = reply(&line, &request);
            let wire = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(wire.as_bytes()).await.unwrap();
        }
    });
    (origin, seen)
}

fn endpoints(origin: &str) -> Endpoints {
    Endpoints {
        auth: format!("{origin}/auth"),
        token: format!("{origin}/token"),
        revoke: format!("{origin}/revoke"),
        api: origin.into(),
    }
}

fn gmail_reply(line: &str, _: &str) -> (u16, String) {
    if line.starts_with("POST /token") {
        return (200, json!({"access_token":"at","expires_in":3600,"scope":"https://www.googleapis.com/auth/gmail.readonly"}).to_string());
    }
    if line.contains("/messages?") {
        return (
            200,
            json!({"messages":[{"id":"m1"},{"id":"bad/../x"},{"id":"m2"}]}).to_string(),
        );
    }
    if line.contains("/messages/m1?") && line.contains("format=full") {
        let body = URL_SAFE.encode("Lunch Friday?\nThanks");
        return (200, json!({"id":"m1","snippet":"Lunch Friday?","payload":{"mimeType":"multipart/mixed","headers":[{"name":"From","value":"Ada <ada@example.com>"},{"name":"Subject","value":"Lunch"},{"name":"To","value":"me"}],"parts":[{"mimeType":"multipart/alternative","filename":"","parts":[{"mimeType":"text/plain","filename":"","body":{"data":body}}]},{"mimeType":"image/png","filename":"menu.png","body":{"attachmentId":"A"}}]}}).to_string());
    }
    if line.contains("/messages/m") {
        let id = if line.contains("/m1?") { "m1" } else { "m2" };
        return (200, json!({"id":id,"snippet":"Lunch &amp; plans","payload":{"headers":[{"name":"From","value":"Ada"},{"name":"Subject","value":"Lunch"},{"name":"Date","value":"Mon"}]}}).to_string());
    }
    (404, "{}".into())
}

#[tokio::test]
async fn gmail_searches_and_reads_with_a_refreshed_readonly_token() {
    let (origin, seen) = mock(gmail_reply).await;
    let gmail = Gmail::new(
        ClientConfig {
            client_id: "cid".into(),
            client_secret: "cs".into(),
        },
        "rt".into(),
        endpoints(&origin),
    )
    .unwrap();
    let cancel = Cancellation::default();
    let hits = gmail
        .search("from:ada newer_than:7d", 5, &cancel)
        .await
        .unwrap();
    assert_eq!(
        hits.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
        ["m1", "m2"]
    );
    assert_eq!(hits[0].snippet, "Lunch & plans");
    let m = gmail.read("m1", &cancel).await.unwrap();
    assert_eq!(m.body, "Lunch Friday?\nThanks");
    assert_eq!(m.attachments, vec!["menu.png"]);
    assert!(gmail.read("../x", &cancel).await.is_err());
    let requests = seen.lock().unwrap().clone();
    // One token refresh is reused; every API call is a GET with the bearer token.
    assert_eq!(
        requests
            .iter()
            .filter(|r| r.starts_with("POST /token"))
            .count(),
        1
    );
    for r in requests.iter().filter(|r| !r.starts_with("POST /token")) {
        assert!(r.starts_with("GET /gmail/v1/users/me/messages"), "{r}");
        assert!(r.to_ascii_lowercase().contains("authorization: bearer at"));
    }
}

#[tokio::test]
async fn gmail_refuses_tokens_with_extra_scopes_and_reports_revocation() {
    let (origin, _) = mock(|line, _| {
        if line.starts_with("POST /token") {
            (200, json!({"access_token":"at","expires_in":3600,"scope":"https://www.googleapis.com/auth/gmail.readonly https://mail.google.com/"}).to_string())
        } else {
            (200, "{}".into())
        }
    })
    .await;
    let gmail = Gmail::new(
        ClientConfig {
            client_id: "c".into(),
            client_secret: "s".into(),
        },
        "rt".into(),
        endpoints(&origin),
    )
    .unwrap();
    assert!(
        gmail
            .search("x", 1, &Cancellation::default())
            .await
            .unwrap_err()
            .contains("beyond gmail.readonly")
    );
    let (origin, _) = mock(|_, _| (400, json!({"error":"invalid_grant"}).to_string())).await;
    let gmail = Gmail::new(
        ClientConfig {
            client_id: "c".into(),
            client_secret: "s".into(),
        },
        "rt".into(),
        endpoints(&origin),
    )
    .unwrap();
    assert!(
        gmail
            .search("x", 1, &Cancellation::default())
            .await
            .unwrap_err()
            .contains("gmail-login")
    );
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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
    struct Counting(AtomicUsize);
    #[async_trait]
    impl MailSource for Counting {
        fn name(&self) -> &'static str {
            "gmail"
        }
        async fn search(
            &self,
            _: &str,
            _: usize,
            _: &Cancellation,
        ) -> Result<Vec<Summary>, String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(vec![Summary {
                id: "m1".into(),
                from: "Ada".into(),
                subject: "Lunch".into(),
                date: "Mon".into(),
                snippet: "Friday?".into(),
            }])
        }
        async fn read(&self, _: &str, _: &Cancellation) -> Result<Message, String> {
            Err("unused".into())
        }
    }

    async fn provider() -> (String, Arc<std::sync::Mutex<Vec<Value>>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let bodies = Arc::new(std::sync::Mutex::new(vec![]));
        let log = bodies.clone();
        tokio::spawn(async move {
            let mut n = 0;
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut raw = vec![];
                let mut b = [0u8; 4096];
                let body = loop {
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
                            break serde_json::from_slice::<Value>(&raw[i + 4..i + 4 + len])
                                .unwrap();
                        }
                    }
                };
                log.lock().unwrap().push(body);
                let output = if n == 0 {
                    json!([{"type":"function_call","id":"i1","call_id":"c1","name":"mail_search","namespace":"astrid","arguments":"{\"source\":\"gmail\",\"query\":\"lunch\"}","status":"completed"}])
                } else {
                    json!([{"type":"message","id":"m","role":"assistant","status":"completed","content":[{"type":"output_text","text":"done","annotations":[]}]}])
                };
                n += 1;
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
        (origin, bodies)
    }

    async fn run(approve: bool) -> (runtime::RunResult, Answers, Arc<Counting>, Vec<Value>) {
        let (origin, bodies) = provider().await;
        let source = Arc::new(Counting(AtomicUsize::new(0)));
        let workspace = tempfile::tempdir().unwrap();
        let tools = Tools::new(
            Workspace::new(workspace.path()).unwrap(),
            Duration::from_secs(1),
        )
        .unwrap()
        .with_mail(Mail::new(vec![source.clone()]).unwrap());
        let provider =
            OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{origin}/v1/responses"))
                .unwrap();
        let mut answers = Answers(approve, vec![]);
        let config = RunConfig {
            routing: None,
            observability: None,
            context_budget: Some(Default::default()),
            model: "gpt-5.6-sol".into(),
            task: "Find Ada's lunch email".into(),
            max_model_calls: 3,
            permissions: Default::default(),
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
        let bodies = bodies.lock().unwrap().clone();
        (result, answers, source, bodies)
    }

    fn tool_outcome(result: &runtime::RunResult) -> ToolOutcome {
        result
            .session
            .messages
            .iter()
            .find_map(|m| match m {
                Msg::Tool(t) if t.call_id == "c1" => Some(t.outcome.clone()),
                _ => None,
            })
            .unwrap()
    }

    #[tokio::test]
    async fn approved_mail_search_reaches_the_model_after_one_prompt() {
        let (result, answers, source, bodies) = run(true).await;
        assert_eq!(result.outcome, RunOutcome::Completed);
        assert_eq!(answers.1.len(), 1);
        assert!(answers.1[0].contains("mail.read_only") && answers.1[0].contains("lunch"));
        assert_eq!(source.0.load(Ordering::SeqCst), 1);
        let tools = bodies[0]["tools"].to_string();
        assert!(tools.contains("mail_search") && tools.contains("\"enum\":[\"gmail\"]"));
        let ToolOutcome::Success { data } = tool_outcome(&result) else {
            panic!()
        };
        assert_eq!(data["results"][0]["subject"], "Lunch");
        assert!(bodies[1].to_string().contains("Friday?"));
    }

    #[tokio::test]
    async fn declined_mail_search_never_touches_the_source() {
        let (result, answers, source, _) = run(false).await;
        assert_eq!(answers.1.len(), 1);
        assert_eq!(source.0.load(Ordering::SeqCst), 0);
        assert!(
            matches!(tool_outcome(&result), ToolOutcome::Error { code, .. } if code == "permission_denied")
        );
    }
}
