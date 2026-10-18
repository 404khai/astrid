use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const ID: &str = "m_0123456789abcdefghijklmnop";
const USAGE: &str = r#"{"machines":5,"snapshots":10,"storedBytes":214748364800,"computedAt":null,"limits":{"maxMachines":5,"maxSnapshots":10,"maxStoredGiB":200},"private":"SECRET"}"#;

async fn server(
    replies: Vec<(u16, String)>,
) -> (DiscoveryClient, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = DiscoveryClient::new("ibk_test_only").unwrap();
    client.origin = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let mut paths = Vec::new();
        for (status, body) in replies {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).await.unwrap();
                bytes.push(byte[0]);
                assert!(bytes.len() < 8192);
            }
            let request = String::from_utf8(bytes).unwrap();
            assert!(
                request
                    .to_lowercase()
                    .contains("x-api-key: ibk_test_only\r\n")
            );
            assert!(!request.to_lowercase().contains("authorization:"));
            paths.push(request.lines().next().unwrap().to_owned());
            let response = format!(
                "HTTP/1.1 {status} Mock\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            // Oversize/cancel tests may close the connection before the server finishes.
            let _ = stream.write_all(response.as_bytes()).await;
        }
        paths
    });
    (client, task)
}

async fn discover(client: &DiscoveryClient, id: Option<&str>) -> Result<Discovery, Error> {
    client
        .discover(true, id, &Cancellation::default(), Duration::from_secs(2))
        .await
}

#[tokio::test]
async fn metadata_only_requests_projection_and_quota_are_honest() {
    let page = format!(
        r#"{{"machines":[{{"id":"{ID}","state":"suspended","metadata":{{"secret":"SECRET"}},"name":"SECRET"}}],"nextCursor":null}}"#
    );
    let (client, task) = server(vec![(200, page), (200, USAGE.into())]).await;
    let result = discover(&client, None).await.unwrap();
    assert!(result.list_complete);
    assert!(!result.storage_measurement_available);
    assert_eq!(result.machines[0].state, "suspended");
    assert_eq!(result.usage.machines, result.usage.limits.max_machines);
    assert_eq!(result.usage.limits.max_stored_gi_b, 200);
    let text = serde_json::to_string(&result).unwrap();
    assert!(!text.contains("SECRET"));
    assert!(!text.contains("ibk_"));
    assert_eq!(
        task.await.unwrap(),
        [
            "GET /v1/machines?limit=100 HTTP/1.1",
            "GET /v1/usage HTTP/1.1"
        ]
    );
}

#[tokio::test]
async fn explicit_selection_does_not_wake_stopped_machine() {
    let (client, task) = server(vec![
        (200, format!(r#"{{"id":"{ID}","state":"stopped"}}"#)),
        (200, USAGE.into()),
    ])
    .await;
    let result = discover(&client, Some(ID)).await.unwrap();
    assert!(!result.list_complete);
    assert_eq!(result.machines[0].state, "stopped");
    assert_eq!(
        task.await.unwrap(),
        [
            format!("GET /v1/machines/{ID} HTTP/1.1"),
            "GET /v1/usage HTTP/1.1".into()
        ]
    );
}

#[tokio::test]
async fn status_failures_are_categorical_and_never_echo_server_secrets() {
    for (status, expected) in [
        (401, Error::Authentication),
        (403, Error::Forbidden),
        (404, Error::NotFound),
        (429, Error::Quota),
        (503, Error::Unavailable),
        (500, Error::Transport),
        (302, Error::Transport),
    ] {
        let (client, task) = server(vec![(
            status,
            r#"{"error":{"message":"SECRET ibk_test_only"}}"#.into(),
        )])
        .await;
        let error = discover(&client, Some(ID)).await.err().unwrap();
        assert_eq!(error, expected);
        assert!(!format!("{error:?} {error}").contains("SECRET"));
        assert_eq!(task.await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn denial_invalid_id_and_precancel_never_dispatch() {
    let (client, task) = server(vec![]).await;
    let cancel = Cancellation::default();
    assert_eq!(
        client
            .discover(false, None, &cancel, Duration::from_secs(1))
            .await
            .err(),
        Some(Error::Denied)
    );
    for id in ["dev", "../usage", "m_abc?secret", "m_é", ""] {
        assert_eq!(
            client
                .discover(true, Some(id), &cancel, Duration::from_secs(1))
                .await
                .err(),
            Some(Error::InvalidId)
        );
    }
    cancel.cancel();
    assert_eq!(
        client
            .discover(true, None, &cancel, Duration::from_secs(1))
            .await
            .err(),
        Some(Error::Cancelled)
    );
    assert!(task.await.unwrap().is_empty());
    assert!(matches!(DiscoveryClient::new(""), Err(Error::MissingKey)));
    assert!(matches!(
        DiscoveryClient::new("ibk_bad\nSECRET"),
        Err(Error::InvalidKey)
    ));
}

#[tokio::test]
async fn malformed_oversize_and_wrong_machine_responses_fail_closed() {
    for body in [
        "SECRET".into(),
        "x".repeat(MAX_BODY + 1),
        r#"{"machines":[]}"#.into(), // missing cursor cannot mean complete
        r#"{"machines":[],"nextCursor":42}"#.into(),
        r#"{"machines":[{"id":"bad","state":"running"}],"nextCursor":null}"#.into(),
    ] {
        let (client, task) = server(vec![(200, body)]).await;
        assert_eq!(
            discover(&client, None).await.err(),
            Some(Error::InvalidResponse)
        );
        task.await.unwrap();
    }
    let (client, task) = server(vec![(
        200,
        r#"{"id":"m_abcdefghijklmnopqrstuvwxyz","state":"running"}"#.into(),
    )])
    .await;
    assert_eq!(
        discover(&client, Some(ID)).await.err(),
        Some(Error::InvalidResponse)
    );
    task.await.unwrap();
}

#[tokio::test]
async fn pagination_and_unknown_states_never_invent_complete_or_running() {
    let page =
        format!(r#"{{"machines":[{{"id":"{ID}","state":"SECRET"}}],"nextCursor":"SECRET"}}"#);
    let (client, task) = server(vec![(200, page), (200, USAGE.into())]).await;
    let result = discover(&client, None).await.unwrap();
    assert!(!result.list_complete);
    assert_eq!(result.machines[0].state, "unknown");
    assert!(!serde_json::to_string(&result).unwrap().contains("SECRET"));
    task.await.unwrap();
}

#[tokio::test]
async fn cancellation_and_deadline_cover_stalled_response_body() {
    for cancelled in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = DiscoveryClient::new("ibk_test_only").unwrap();
        client.origin = format!("http://{}", listener.local_addr().unwrap());
        let cancel = Cancellation::default();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let hold = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n{")
                .await
                .unwrap();
            ready_tx.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        let token = cancel.clone();
        let pending = tokio::spawn(async move {
            client
                .discover(true, None, &token, Duration::from_millis(500))
                .await
                .err()
        });
        ready_rx.await.unwrap();
        if cancelled {
            cancel.cancel();
        }
        let error = tokio::time::timeout(Duration::from_secs(2), pending)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            error,
            Some(if cancelled {
                Error::Cancelled
            } else {
                Error::Timeout
            })
        );
        hold.abort();
        assert!(hold.await.unwrap_err().is_cancelled());
    }
}
