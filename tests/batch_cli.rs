//! Real CLI metadata preflight, with synthetic private storage and no auth.
use astrid::{
    batch_trace::{Family, Record},
    scheduler::BatchId,
};
use std::process::Command;
#[test]
fn batch_inspection_is_separate_and_partial_metadata_does_not_claim_completion() {
    let home = tempfile::tempdir().unwrap();
    let binary = env!("CARGO_BIN_EXE_astrid");
    let empty = Command::new(binary)
        .env("HOME", home.path())
        .args(["batch", "trace"])
        .output()
        .unwrap();
    assert!(empty.status.success());
    assert!(
        String::from_utf8(empty.stdout)
            .unwrap()
            .contains("No retained batch traces")
    );
    let store = home.path().join(".config/astrid/traces");
    std::fs::create_dir_all(&store).unwrap();
    let id = BatchId::default();
    let header = Record::Header {
        family: Family::Batch,
        schema: 1,
        batch: id,
        tasks: 1,
        edges: 0,
        concurrency: 1,
        reserved_calls: 1,
    };
    std::fs::write(
        store.join(format!("batch-{id}.partial")),
        format!("{}\n", serde_json::to_string(&header).unwrap()),
    )
    .unwrap();
    let inspect = Command::new(binary)
        .env("HOME", home.path())
        .args(["batch", "trace", &id.to_string()])
        .output()
        .unwrap();
    assert!(inspect.status.success());
    let value: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(value["complete"], false);
    assert_eq!(value["state"]["outcome"], serde_json::Value::Null);
    let normal = Command::new(binary)
        .env("HOME", home.path())
        .args(["trace"])
        .output()
        .unwrap();
    assert!(normal.status.success());
    assert!(
        String::from_utf8(normal.stdout)
            .unwrap()
            .contains("No retained traces")
    );
    assert!(!home.path().join(".config/astrid/credentials.json").exists());
    let unsafe_id = Command::new(binary)
        .env("HOME", home.path())
        .args(["batch", "trace", "../secret"])
        .output()
        .unwrap();
    assert!(!unsafe_id.status.success());
}

#[test]
fn manifests_are_bounded_versioned_snapshots_and_invalid_graphs_never_authenticate() {
    use astrid::batch_manifest::Manifest;
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let path = root.path().join("tasks.json");
    let valid = serde_json::json!({"schema":1,"tasks":[{"key":"A","instruction":"original","model":"fixture"}]});
    std::fs::write(&path, valid.to_string()).unwrap();
    let snapshot = Manifest::load(&path).unwrap();
    std::fs::write(&path, "changed after snapshot").unwrap();
    assert_eq!(snapshot.tasks[0].instruction, "original");
    assert!(Manifest::parse(&vec![b' '; astrid::scheduler::MAX_MANIFEST_BYTES + 1]).is_err());
    for invalid in [
        serde_json::json!({"schema":2,"tasks":[]}),
        serde_json::json!({"schema":1,"tasks":[],"extra":"forbidden"}),
        serde_json::json!({"schema":1,"tasks":[{"key":"A","instruction":"inspect","model":"fixture","depends_on":["A"]}]}),
        serde_json::json!({"schema":1,"tasks":[{"key":"A","instruction":"inspect","model":"fixture","permissions":{"read":"allow","write":"deny","execute":"deny","network":"allow"}}]}),
        serde_json::json!({"schema":1,"tasks":[{"key":"A","instruction":"inspect","model":"fixture","context_budget":{"extra":1}}]}),
    ] {
        std::fs::write(&path, invalid.to_string()).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_astrid"))
            .env("HOME", home.path())
            .args(["batch", "run"])
            .arg(&path)
            .arg("--folder")
            .arg(root.path())
            .args(["--observability", "off"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!home.path().join(".config/astrid/auth.lock").exists());
        assert!(!home.path().join(".config/astrid/sessions").exists());
    }
}
#[test]
fn failed_cli_batch_prints_terminal_source_bundle_without_creating_idle_sessions() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tasks.json");
    std::fs::write(&path,r#"{"schema":1,"tasks":[{"key":"A","instruction":"inspect","model":"fixture"},{"key":"C","instruction":"reconcile","model":"fixture","depends_on":["A"]}]}"#).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_astrid"))
        .env("HOME", home.path())
        .args(["batch", "run"])
        .arg(&path)
        .arg("--folder")
        .arg(root.path())
        .args(["--observability", "off", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["outcome"], "failed");
    assert_eq!(report["tasks"][0]["status"]["state"], "failed");
    assert_eq!(report["tasks"][1]["status"]["state"], "skipped");
    assert!(report["tasks"][1]["status"]["run"].is_null());
    assert!(report["measurements"]["usage"].is_null());
    assert!(!home.path().join(".config/astrid/sessions").exists());
}
#[test]
fn actual_sigint_cancels_two_real_auth_waiters_and_joins_before_exit() {
    use std::{
        fs::OpenOptions,
        time::{Duration, Instant},
    };
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let account = home.path().join(".config/astrid");
    std::fs::create_dir_all(&account).unwrap();
    let held = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(account.join("auth.lock"))
        .unwrap();
    held.lock().unwrap();
    let manifest = root.path().join("tasks.json");
    std::fs::write(&manifest,r#"{"schema":1,"tasks":[{"key":"A","instruction":"inspect","model":"fixture"},{"key":"B","instruction":"inspect","model":"fixture"},{"key":"queued","instruction":"inspect","model":"fixture"}]}"#).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_astrid"))
        .env("HOME", home.path())
        .args(["batch", "run"])
        .arg(&manifest)
        .arg("--folder")
        .arg(root.path())
        .args(["--observability", "on", "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let mut observed = false;
    while start.elapsed() < Duration::from_secs(8) {
        let count = std::fs::read_dir(account.join("traces"))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|e| !e.file_name().to_string_lossy().starts_with("batch-"))
            .filter(|e| {
                std::fs::read_to_string(e.path()).is_ok_and(|s| s.contains("model_call_started"))
            })
            .count();
        if count >= 2 {
            observed = true;
            break;
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if observed {
        assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    }
    let wait = Instant::now();
    let mut exited = false;
    while wait.elapsed() < Duration::from_secs(8) {
        if child.try_wait().unwrap().is_some() {
            exited = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if !exited {
        child.kill().unwrap();
    }
    let output = child.wait_with_output().unwrap();
    held.unlock().unwrap();
    assert!(
        observed,
        "two independently linked model/auth waits were not observed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(exited, "SIGINT did not join owned work");
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["outcome"], "cancelled");
    assert!(
        report["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["status"]["state"] == "cancelled")
    );
    assert!(report["tasks"][2]["status"]["run"].is_null());
    assert!(!account.join("credentials.json").exists());
    assert!(!account.join("sessions").exists());
}
