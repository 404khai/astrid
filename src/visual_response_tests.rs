//! ADR 0025 synthetic images and loopback only; no live APIs or credentials.
use super::*;
use crate::model::{ModelProvider, ModelRequest, TextSink};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::{mpsc, oneshot};

struct StreamServer {
    origin: String,
    chunks: mpsc::Sender<String>,
    requests: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for StreamServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl StreamServer {
    async fn new(headers: bool) -> (Self, oneshot::Receiver<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let (chunks, mut rx) = mpsc::channel::<String>(4);
        let (seen, seen_rx) = oneshot::channel();
        let requests = Arc::new(AtomicUsize::new(0));
        let count = requests.clone();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            count.fetch_add(1, Ordering::SeqCst);
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                socket.read_exact(&mut b).await.unwrap();
                head.push(b[0]);
                assert!(head.len() < 16384);
            }
            let head = String::from_utf8(head).unwrap();
            let length = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            assert!(length < 2_000_000);
            assert!(head.contains("application/json"));
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            let body: Value = serde_json::from_slice(&body).unwrap();
            assert!(body.to_string().contains("data:image/png;base64,"));
            assert_eq!(body["store"], false);
            if headers {
                socket.write_all(b"HTTP/1.1 200 Mock\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n").await.unwrap();
                socket
                    .write_all(
                        format!(
                            "data: {}\n\n",
                            json!({"type":"response.output_text.delta","delta":"seen"})
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
            let _ = seen.send(());
            while let Some(chunk) = rx.recv().await {
                if socket.write_all(chunk.as_bytes()).await.is_err() {
                    break;
                }
            }
        });
        (
            Self {
                origin,
                chunks,
                requests,
                task,
            },
            seen_rx,
        )
    }
}
struct Visible<'a> {
    image: &'a VisionInput,
    first: Option<oneshot::Sender<()>>,
    text: String,
    expected: crate::context::ContextSnapshot,
}
#[async_trait]
impl TextSink for Visible<'_> {
    async fn request_prepared(
        &mut self,
        snapshot: crate::context::ContextSnapshot,
    ) -> io::Result<()> {
        assert_eq!(snapshot, self.expected);
        Ok(())
    }
    async fn delta(&mut self, text: &str) -> io::Result<()> {
        // Assert cleanup immediately, not merely after advancing beyond expiry.
        assert!(
            self.image.data_url().is_err(),
            "canonical frame survived upload admission"
        );
        self.text.push_str(text);
        if let Some(first) = self.first.take() {
            let _ = first.send(());
        }
        Ok(())
    }
}
fn request() -> ModelRequest<'static> {
    ModelRequest {
        model: "gpt-5.6-sol",
        instructions: "Describe only the synthetic fixture",
        messages: &[],
        options: Default::default(),
    }
}
async fn image_fixture() -> (ComputerRun, VisionInput, MockHttp) {
    let wire =
        MockHttp::start(|_, _, _| panic!("no real machine calls in visual timing test")).await;
    let mut run = service(&wire).begin(RunId::default());
    run.install(
        crate::computer_png::canonicalize(&fixture()).unwrap(),
        Instant::now(),
        &Cancellation::default(),
    )
    .unwrap();
    let image = run
        .take_vision(
            ModelCallId::default(),
            "openai_codex_subscription",
            "gpt-5.6-sol",
        )
        .unwrap()
        .unwrap();
    (run, image, wire)
}

#[tokio::test]
async fn visual_completion_after_image_expiry_releases_frame_and_keeps_exact_context() {
    let (run, image, wire) = image_fixture().await;
    let (server, _) = StreamServer::new(true).await;
    let provider =
        OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{}/v1/responses", server.origin))
            .unwrap();
    let request = request();
    let expected = provider
        .measure_visual_request(&request, &image)
        .unwrap()
        .unwrap();
    let grant = image.approve();
    let cancel = Cancellation::default();
    let (first, seen) = oneshot::channel();
    let finished = AtomicBool::new(false);
    let mut sink = Visible {
        image: &image,
        first: Some(first),
        text: String::new(),
        expected,
    };
    let generation = async {
        let r = provider
            .generate_visual(&request, &image, &grant, &cancel, &mut sink)
            .await;
        finished.store(true, Ordering::SeqCst);
        r
    };
    let drive = async {
        seen.await.unwrap();
        assert!(run.frame.lock().unwrap().is_none());
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(11)).await;
        assert!(
            !finished.load(Ordering::SeqCst),
            "partial text became synthetic completion"
        );
        server
            .chunks
            .send(completed(vec![message("seen")]))
            .await
            .unwrap();
    };
    let (result, ()) = tokio::join!(generation, drive);
    let response = result.unwrap();
    assert_eq!(response.text, "seen");
    assert!(response.tool_calls.is_empty());
    assert_eq!(sink.text, "seen");
    assert!(
        !serde_json::to_string(&Message::Assistant(response))
            .unwrap()
            .contains("data:image")
    );
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
    assert!(wire.requests.lock().unwrap().is_empty());
    assert!(
        run.actions
            .request_for(
                ToolCallId::default(),
                "stale-call".into(),
                Arguments {
                    frame_id: image.binding.frame_id,
                    action: desktop::Action::Click {
                        x: 1,
                        y: 1,
                        button: desktop::Button::Left
                    }
                },
                &cancel
            )
            .is_err()
    );
    assert!(grant.consume(&image, "gpt-5.6-sol", &cancel).is_err());
}

#[tokio::test]
async fn slow_visual_response_has_fixed_sixty_second_deadline_and_no_retry() {
    let (_run, image, _wire) = image_fixture().await;
    let (server, _) = StreamServer::new(true).await;
    let provider =
        OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{}/v1/responses", server.origin))
            .unwrap();
    let request = request();
    let expected = provider
        .measure_visual_request(&request, &image)
        .unwrap()
        .unwrap();
    let grant = image.approve();
    let cancel = Cancellation::default();
    let (first, seen) = oneshot::channel();
    let mut sink = Visible {
        image: &image,
        first: Some(first),
        text: String::new(),
        expected,
    };
    let generation = provider.generate_visual(&request, &image, &grant, &cancel, &mut sink);
    let drive = async {
        seen.await.unwrap();
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(60)).await;
    };
    let (result, ()) = tokio::join!(generation, drive);
    let error = result.unwrap_err().to_string();
    assert!(error.contains("60 seconds from POST admission"), "{error}");
    assert_eq!(sink.text, "seen");
    assert!(image.data_url().is_err());
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn visual_cancellation_after_upload_keeps_partial_text_and_no_completion() {
    let (_run, image, _wire) = image_fixture().await;
    let (server, _) = StreamServer::new(true).await;
    let provider =
        OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{}/v1/responses", server.origin))
            .unwrap();
    let request = request();
    let expected = provider
        .measure_visual_request(&request, &image)
        .unwrap()
        .unwrap();
    let grant = image.approve();
    let cancel = Cancellation::default();
    let (first, seen) = oneshot::channel();
    let mut sink = Visible {
        image: &image,
        first: Some(first),
        text: String::new(),
        expected,
    };
    let generation = provider.generate_visual(&request, &image, &grant, &cancel, &mut sink);
    let drive = async {
        seen.await.unwrap();
        assert!(image.data_url().is_err());
        cancel.cancel();
    };
    let (result, ()) = tokio::join!(generation, drive);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("cancelled; remote outcome unknown")
    );
    assert_eq!(sink.text, "seen");
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn stalled_headers_still_fail_with_original_image_deadline() {
    let (_run, image, _wire) = image_fixture().await;
    let (server, seen) = StreamServer::new(false).await;
    let provider =
        OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{}/v1/responses", server.origin))
            .unwrap();
    let request = request();
    let expected = provider
        .measure_visual_request(&request, &image)
        .unwrap()
        .unwrap();
    let grant = image.approve();
    let cancel = Cancellation::default();
    let (first, _) = oneshot::channel();
    let mut sink = Visible {
        image: &image,
        first: Some(first),
        text: String::new(),
        expected,
    };
    let generation = provider.generate_visual(&request, &image, &grant, &cancel, &mut sink);
    let drive = async {
        seen.await.unwrap();
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(10)).await;
    };
    let (result, ()) = tokio::join!(generation, drive);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("visual upload deadline")
    );
    assert!(sink.text.is_empty());
    assert!(image.data_url().is_err());
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

struct WaitingAuth(Mutex<Option<oneshot::Sender<()>>>);
#[async_trait]
impl Authentication for WaitingAuth {
    async fn bearer_token(&self) -> Result<BearerToken, AuthError> {
        if let Some(started) = self.0.lock().unwrap().take() {
            let _ = started.send(());
        }
        std::future::pending().await
    }
}
#[tokio::test]
async fn authentication_wait_cannot_spend_response_allowance_or_send_an_image() {
    let (_run, image, _wire) = image_fixture().await;
    let api =
        MockHttp::start(|_, _, _| panic!("authentication never completed; no POST permitted"))
            .await;
    let (started, waiting) = oneshot::channel();
    let provider = OpenAiProvider::with_endpoint(
        Arc::new(WaitingAuth(Mutex::new(Some(started)))),
        format!("{}/v1/responses", api.origin),
    )
    .unwrap();
    let request = request();
    let expected = provider
        .measure_visual_request(&request, &image)
        .unwrap()
        .unwrap();
    let grant = image.approve();
    let cancel = Cancellation::default();
    let (first, _) = oneshot::channel();
    let mut sink = Visible {
        image: &image,
        first: Some(first),
        text: String::new(),
        expected,
    };
    let generation = provider.generate_visual(&request, &image, &grant, &cancel, &mut sink);
    let drive = async {
        waiting.await.unwrap();
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(10)).await;
    };
    let (result, ()) = tokio::join!(generation, drive);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("visual upload deadline")
    );
    assert!(image.data_url().is_err());
    assert!(api.requests.lock().unwrap().is_empty());
    assert!(sink.text.is_empty());
}

#[tokio::test]
async fn dropping_visual_future_during_header_wait_releases_frame() {
    let (_run, image, _wire) = image_fixture().await;
    let (server, seen) = StreamServer::new(false).await;
    let provider =
        OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{}/v1/responses", server.origin))
            .unwrap();
    let request = request();
    let expected = provider
        .measure_visual_request(&request, &image)
        .unwrap()
        .unwrap();
    let grant = image.approve();
    let cancel = Cancellation::default();
    let (first, _) = oneshot::channel();
    let mut sink = Visible {
        image: &image,
        first: Some(first),
        text: String::new(),
        expected,
    };
    let mut generation =
        Box::pin(provider.generate_visual(&request, &image, &grant, &cancel, &mut sink));
    tokio::select! {biased;_=seen=>{},r=&mut generation=>panic!("headers unexpectedly completed: {r:?}")}
    drop(generation);
    assert!(image.data_url().is_err());
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn rejected_visual_headers_release_images_without_echo_or_retry() {
    for (status, content_type) in [
        (403, "application/json"),
        (200, "application/json"),
        (200, ""),
    ] {
        let (_run, image, _wire) = image_fixture().await;
        let api = MockHttp::start(move |n, _, _| {
            assert_eq!(n, 0, "no retry");
            (
                status,
                content_type,
                "private-provider-body-or-screen-data".into(),
            )
        })
        .await;
        let provider =
            OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{}/v1/responses", api.origin))
                .unwrap();
        let request = request();
        let expected = provider
            .measure_visual_request(&request, &image)
            .unwrap()
            .unwrap();
        let grant = image.approve();
        let cancel = Cancellation::default();
        let (first, _) = oneshot::channel();
        let mut sink = Visible {
            image: &image,
            first: Some(first),
            text: String::new(),
            expected,
        };
        let error = provider
            .generate_visual(&request, &image, &grant, &cancel, &mut sink)
            .await
            .unwrap_err()
            .to_string();
        assert!(!error.contains("private-provider"));
        assert!(image.data_url().is_err());
        assert_eq!(api.requests.lock().unwrap().len(), 1);
        assert!(sink.text.is_empty());
    }
}

#[tokio::test]
async fn invalid_destination_rejects_before_auth_and_releases_the_frame() {
    let (_run, image, _wire) = image_fixture().await;
    let approved = OpenAiProvider::new(Arc::new(Auth)).unwrap();
    let request = request();
    let expected = approved
        .measure_visual_request(&request, &image)
        .unwrap()
        .unwrap();
    let (started, mut waiting) = oneshot::channel();
    let provider = OpenAiProvider::with_endpoint(
        Arc::new(WaitingAuth(Mutex::new(Some(started)))),
        "https://unapproved.example/v1/responses",
    )
    .unwrap();
    let grant = image.approve();
    let cancel = Cancellation::default();
    let (first, _) = oneshot::channel();
    let mut sink = Visible {
        image: &image,
        first: Some(first),
        text: String::new(),
        expected,
    };
    let error = provider
        .generate_visual(&request, &image, &grant, &cancel, &mut sink)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("destination"));
    assert!(waiting.try_recv().is_err());
    assert!(image.data_url().is_err());
}

#[tokio::test]
async fn header_latency_counts_toward_sixty_seconds_instead_of_resetting_deadline() {
    let (_run, image, _wire) = image_fixture().await;
    let (server, body_seen) = StreamServer::new(false).await;
    let provider =
        OpenAiProvider::with_endpoint(Arc::new(Auth), format!("{}/v1/responses", server.origin))
            .unwrap();
    let request = request();
    let expected = provider
        .measure_visual_request(&request, &image)
        .unwrap()
        .unwrap();
    let grant = image.approve();
    let cancel = Cancellation::default();
    let (first, text_seen) = oneshot::channel();
    let mut sink = Visible {
        image: &image,
        first: Some(first),
        text: String::new(),
        expected,
    };
    let started = Instant::now();
    let generation = provider.generate_visual(&request, &image, &grant, &cancel, &mut sink);
    let drive = async {
        body_seen.await.unwrap();
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(8)).await;
        // Keep real local I/O responsive while making the header latency explicit.
        tokio::time::resume();
        server.chunks.send(format!("HTTP/1.1 200 Mock\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {}\n\n",json!({"type":"response.output_text.delta","delta":"seen"}))).await.unwrap();
        text_seen.await.unwrap();
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(52)).await;
    };
    let (result, ()) = tokio::join!(generation, drive);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("60 seconds from POST admission")
    );
    assert!(
        Instant::now().duration_since(started) < Duration::from_secs(61),
        "header admission reset the response allowance"
    );
    assert_eq!(sink.text, "seen");
    assert!(image.data_url().is_err());
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}
