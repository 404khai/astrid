use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const ID: &str = "m_0123456789abcdefghijklmnop";
const USAGE: &str = r#"{"machines":5,"snapshots":10,"storedBytes":214748364800,"computedAt":null,"limits":{"maxMachines":5,"maxSnapshots":10,"maxStoredGiB":200},"private":"SECRET"}"#;

#[test]
fn dotenv_key_loading_is_local_literal_and_environment_takes_precedence() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(".env");
    assert_eq!(load_key(None, &path).err(), Some(Error::MissingKey));
    for assignment in [
        "INSTABOX_API_KEY=ibk_synthetic",
        "export INSTABOX_API_KEY = 'ibk_synthetic'",
        "INSTABOX_API_KEY=\"ibk_synthetic\"",
    ] {
        std::fs::write(&path, format!("# comment\nOTHER=ignored\n{assignment}\r\n")).unwrap();
        assert_eq!(load_key(None, &path).unwrap(), "ibk_synthetic");
        assert_eq!(
            load_key(Some(String::new()), &path).unwrap(),
            "ibk_synthetic"
        );
        assert_eq!(
            load_key(Some("ibk_environment".into()), &path).unwrap(),
            "ibk_environment"
        );
    }
    std::fs::write(&path, "INSTABOX_API_KEY=$(touch forbidden)\n").unwrap();
    assert_eq!(load_key(None, &path).unwrap(), "$(touch forbidden)");
    assert!(!root.path().join("forbidden").exists());
    assert_eq!(
        load_key(None, &root.path().join("child/.env")).err(),
        Some(Error::MissingKey)
    );
}

#[test]
fn dotenv_failures_do_not_echo_secrets_or_follow_symlinks() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(".env");
    for text in [
        "INSTABOX_API_KEY='SECRET",
        "INSTABOX_API_KEY=SECRET\nINSTABOX_API_KEY=duplicate",
        "INSTABOX_API_KEY SECRET",
    ] {
        std::fs::write(&path, text).unwrap();
        let error = load_key(None, &path).unwrap_err();
        assert_eq!(error, Error::InvalidEnvFile);
        assert!(!format!("{error:?} {error}").contains("SECRET"));
    }
    std::fs::write(&path, "x".repeat(16_385)).unwrap();
    assert_eq!(load_key(None, &path).err(), Some(Error::InvalidEnvFile));
    std::fs::write(&path, [0xff]).unwrap();
    assert_eq!(load_key(None, &path).err(), Some(Error::InvalidEnvFile));
    std::fs::write(&path, "INSTABOX_API_KEY=\n").unwrap();
    assert_eq!(load_key(None, &path).err(), Some(Error::MissingKey));
    let link = root.path().join("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert_eq!(load_key(None, &link).err(), Some(Error::InvalidEnvFile));
    assert_eq!(
        load_key(None, root.path()).err(),
        Some(Error::InvalidEnvFile)
    );
}

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

async fn request_server(
    replies: Vec<(u16, String)>,
) -> (DiscoveryClient, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = DiscoveryClient::new("ibk_test_only").unwrap();
    client.origin = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in replies {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).await.unwrap();
                bytes.push(byte[0]);
                assert!(bytes.len() < 8192);
            }
            let headers = String::from_utf8(bytes.clone()).unwrap();
            assert!(
                headers
                    .to_lowercase()
                    .contains("x-api-key: ibk_test_only\r\n")
            );
            assert!(!headers.to_lowercase().contains("authorization:"));
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(str::to_owned)
                })
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let mut payload = vec![0; length];
            stream.read_exact(&mut payload).await.unwrap();
            requests.push(format!(
                "{}\n{}",
                headers.lines().next().unwrap(),
                String::from_utf8(payload).unwrap()
            ));
            let response = format!(
                "HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
        requests
    });
    (client, task)
}

#[tokio::test]
async fn exec_requires_exact_grant_and_running_existing_machine_before_post() {
    let grant = ExecGrant {
        machine_id: ID.into(),
        command: "printf test".into(),
        cwd: "/data/home".into(),
        timeout_seconds: 30,
    };
    let (client, task) = request_server(vec![]).await;
    assert_eq!(
        client
            .execute(None, &grant, &Cancellation::default())
            .await
            .err(),
        Some(Error::ExecDenied)
    );
    assert!(task.await.unwrap().is_empty());

    let (client, task) = request_server(vec![(
        200,
        format!(r#"{{"id":"{ID}","state":"suspended"}}"#),
    )])
    .await;
    assert_eq!(
        client
            .execute(Some(&grant), &grant, &Cancellation::default())
            .await
            .err(),
        Some(Error::MachineNotRunning)
    );
    assert_eq!(task.await.unwrap().len(), 1);
}

#[tokio::test]
async fn exec_preserves_attribution_separate_output_exit_timeout_and_coverage_without_retry() {
    let grant = ExecGrant {
        machine_id: ID.into(),
        command: "sh -c 'echo out; echo err >&2; exit 7'".into(),
        cwd: "/work".into(),
        timeout_seconds: 45,
    };
    let response =
        r#"{"stdout":"out\n","stderr":"err\n","exitCode":7,"timedOut":false,"truncated":true}"#;
    let (client, task) = request_server(vec![
        (200, format!(r#"{{"id":"{ID}","state":"running"}}"#)),
        (200, response.into()),
    ])
    .await;
    let report = client
        .execute(Some(&grant), &grant, &Cancellation::default())
        .await
        .unwrap();
    assert_eq!(report.machine_id, ID);
    assert_eq!(report.stdout, "out\n");
    assert_eq!(report.stderr, "err\n");
    assert_eq!(report.exit_status, Some(7));
    assert!(!report.timed_out);
    assert!(report.output_truncated);
    assert_eq!(report.remote_outcome, RemoteOutcome::ObservedCompleted);
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].starts_with(&format!("POST /v1/machines/{ID}/exec HTTP/1.1")));
    let body: serde_json::Value =
        serde_json::from_str(requests[1].split_once('\n').unwrap().1).unwrap();
    assert_eq!(body["cmd"], grant.command);
    assert_eq!(body["cwd"], grant.cwd);
    assert_eq!(body["timeoutSeconds"], 45);
    assert_eq!(body.as_object().unwrap().len(), 3);
    assert!(body.get("envs").is_none());
    assert!(body.get("stdin").is_none());
}

#[tokio::test]
async fn provider_timeout_and_post_dispatch_cancellation_are_reported_honestly() {
    let grant = ExecGrant {
        machine_id: ID.into(),
        command: "sleep 90".into(),
        cwd: "/work".into(),
        timeout_seconds: 1,
    };
    let (client, task) = request_server(vec![
        (200, format!(r#"{{"id":"{ID}","state":"running"}}"#)),
        (
            200,
            r#"{"stdout":"partial","stderr":"","exitCode":124,"timedOut":true,"truncated":false}"#
                .into(),
        ),
    ])
    .await;
    let report = client
        .execute(Some(&grant), &grant, &Cancellation::default())
        .await
        .unwrap();
    assert!(report.timed_out);
    assert_eq!(report.exit_status, Some(124));
    assert_eq!(report.remote_outcome, RemoteOutcome::ObservedCompleted);
    assert_eq!(task.await.unwrap().len(), 2);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = DiscoveryClient::new("ibk_test_only").unwrap();
    client.origin = format!("http://{}", listener.local_addr().unwrap());
    let cancel = Cancellation::default();
    let signal = cancel.clone();
    let server = tokio::spawn(async move {
        for index in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).await.unwrap();
                bytes.push(byte[0]);
            }
            if index == 0 {
                let body = format!(r#"{{"id":"{ID}","state":"running"}}"#);
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            } else {
                signal.cancel();
                std::future::pending::<()>().await;
            }
        }
    });
    let report = client.execute(Some(&grant), &grant, &cancel).await.unwrap();
    assert_eq!(report.status, ExecStatus::Cancelled);
    assert_eq!(report.remote_outcome, RemoteOutcome::Unknown);
    assert_eq!(report.machine_id, ID);
    assert_eq!(report.exit_status, None);
    assert!(report.stdout.is_empty() && report.stderr.is_empty());
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
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

#[tokio::test]
async fn exec_rejects_changed_grants_invalid_requests_and_non_running_states() {
    let request = ExecGrant {
        machine_id: ID.into(),
        command: "printf test".into(),
        cwd: "/data/home".into(),
        timeout_seconds: 1,
    };
    let (client, task) = request_server(vec![]).await;
    let mut changed = request.clone();
    changed.command = "different command".into();
    assert_eq!(
        client
            .execute(Some(&changed), &request, &Cancellation::default())
            .await
            .unwrap_err(),
        Error::ExecDenied
    );
    for change in 0..5 {
        let mut invalid = request.clone();
        match change {
            0 => invalid.machine_id = "../usage".into(),
            1 => invalid.command = "  ".into(),
            2 => invalid.command = "bad\0command".into(),
            3 => invalid.cwd = "relative".into(),
            _ => invalid.timeout_seconds = 601,
        }
        assert!(matches!(
            client
                .execute(Some(&invalid), &invalid, &Cancellation::default())
                .await,
            Err(Error::InvalidId | Error::InvalidExecRequest)
        ));
    }
    let cancel = Cancellation::default();
    cancel.cancel();
    assert_eq!(
        client
            .execute(Some(&request), &request, &cancel)
            .await
            .unwrap_err(),
        Error::Cancelled
    );
    assert!(task.await.unwrap().is_empty());
    for state in [
        "creating",
        "starting",
        "stopped",
        "stopping",
        "suspending",
        "suspended",
        "resuming",
        "deleting",
        "error",
    ] {
        let (client, task) =
            request_server(vec![(200, format!(r#"{{"id":"{ID}","state":"{state}"}}"#))]).await;
        assert_eq!(
            client
                .execute(Some(&request), &request, &Cancellation::default())
                .await
                .unwrap_err(),
            Error::MachineNotRunning
        );
        assert_eq!(
            task.await.unwrap(),
            [format!("GET /v1/machines/{ID} HTTP/1.1\n")]
        );
    }
    let (client, task) = request_server(vec![(
        200,
        r#"{"id":"m_abcdefghijklmnopqrstuvwxyz","state":"running"}"#.into(),
    )])
    .await;
    assert_eq!(
        client
            .execute(Some(&request), &request, &Cancellation::default())
            .await
            .unwrap_err(),
        Error::InvalidResponse
    );
    assert_eq!(task.await.unwrap().len(), 1);
}

#[tokio::test]
async fn post_errors_keep_machine_and_unknown_outcome_without_echoing_errors_or_retry() {
    let grant = ExecGrant {
        machine_id: ID.into(),
        command: "printf test".into(),
        cwd: "/data/home".into(),
        timeout_seconds: 1,
    };
    for (status, category, body) in [
        (401, "authentication", "SECRET".into()),
        (403, "forbidden", "SECRET".into()),
        (404, "not_found", "SECRET".into()),
        (429, "quota", "SECRET".into()),
        (503, "unavailable", "SECRET".into()),
        (500, "transport", "SECRET".into()),
        (302, "transport", "SECRET".into()),
        (200, "invalid_response", "SECRET".into()),
        (200, "invalid_response", "x".repeat(MAX_BODY + 1)),
        (
            200,
            "invalid_response",
            r#"{"stdout":"","stderr":"","timedOut":false,"truncated":false}"#.into(),
        ),
    ] {
        let (client, task) = request_server(vec![
            (200, format!(r#"{{"id":"{ID}","state":"running"}}"#)),
            (status, body),
        ])
        .await;
        let report = client
            .execute(Some(&grant), &grant, &Cancellation::default())
            .await
            .unwrap();
        assert_eq!(report.status, ExecStatus::Failed);
        assert_eq!(report.machine_id, ID);
        assert_eq!(report.remote_outcome, RemoteOutcome::Unknown);
        assert_eq!(report.failure, Some(category));
        assert!(!report.output_available);
        assert_eq!(report.exit_status, None);
        let text = serde_json::to_string(&report).unwrap();
        assert!(!text.contains("SECRET") && !text.contains("ibk_test_only"));
        assert_eq!(task.await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn exec_deadlines_and_cancellation_cover_metadata_and_post_response_bodies() {
    for post_phase in [false, true] {
        for cancelled in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let mut client = DiscoveryClient::new("ibk_test_only").unwrap();
            client.origin = format!("http://{}", listener.local_addr().unwrap());
            let cancel = Cancellation::default();
            let signal = cancel.clone();
            let (ready, waiting) = tokio::sync::oneshot::channel();
            let server = tokio::spawn(async move {
                for phase in 0..=usize::from(post_phase) {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let mut bytes = Vec::new();
                    while !bytes.ends_with(b"\r\n\r\n") {
                        let mut byte = [0];
                        stream.read_exact(&mut byte).await.unwrap();
                        bytes.push(byte[0]);
                    }
                    if post_phase && phase == 0 {
                        let body = format!(r#"{{"id":"{ID}","state":"running"}}"#);
                        stream.write_all(format!("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
                    } else {
                        stream
                            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n{")
                            .await
                            .unwrap();
                        let _ = ready.send(());
                        std::future::pending::<()>().await;
                        break;
                    }
                }
            });
            let request = ExecGrant {
                machine_id: ID.into(),
                command: "printf test".into(),
                cwd: "/data/home".into(),
                timeout_seconds: 1,
            };
            let pending = tokio::spawn(async move {
                client
                    .execute_bounded(
                        Some(&request),
                        &request,
                        &cancel,
                        Duration::from_millis(500),
                        Duration::from_millis(500),
                    )
                    .await
            });
            tokio::time::timeout(Duration::from_secs(2), waiting)
                .await
                .unwrap()
                .unwrap();
            if cancelled {
                signal.cancel();
            }
            let result = tokio::time::timeout(Duration::from_secs(2), pending)
                .await
                .unwrap()
                .unwrap();
            if post_phase {
                let report = result.unwrap();
                assert_eq!(
                    report.status,
                    if cancelled {
                        ExecStatus::Cancelled
                    } else {
                        ExecStatus::DeadlineExceeded
                    }
                );
                assert_eq!(report.remote_outcome, RemoteOutcome::Unknown);
                assert!(!report.output_available);
                assert_eq!(report.machine_id, ID);
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    if cancelled {
                        Error::Cancelled
                    } else {
                        Error::Timeout
                    }
                );
            }
            server.abort();
            assert!(server.await.unwrap_err().is_cancelled());
        }
    }
}
