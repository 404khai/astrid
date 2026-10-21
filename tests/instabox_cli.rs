//! Real CLI preflight only. No real keys or cloud requests.
use std::process::Command;

#[test]
fn discovery_and_execution_require_explicit_grants_before_credentials() {
    let root = tempfile::tempdir().unwrap();
    let invoke = |args: &[&str], key: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_astrid"));
        command
            .env_clear()
            .env("HOME", root.path())
            .current_dir(root.path())
            .args(args);
        if let Some(key) = key {
            command.env("INSTABOX_API_KEY", key);
        }
        command.output().unwrap()
    };
    let help = invoke(&["instabox", "discover", "--help"], None);
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("--allow-discovery")
    );
    for (args, key, expected) in [
        (
            vec!["instabox", "discover"],
            Some("ibk_SECRET"),
            "discovery denied",
        ),
        (
            vec!["instabox", "discover", "--allow-discovery"],
            None,
            "set INSTABOX_API_KEY",
        ),
        (
            vec![
                "instabox",
                "discover",
                "--allow-discovery",
                "--machine",
                "../usage",
            ],
            Some("ibk_SECRET"),
            "invalid machine ID",
        ),
        (
            vec![
                "instabox",
                "exec",
                "--machine",
                "m_0123456789abcdefghijklmnop",
                "--command",
                "printf test",
            ],
            None,
            "execution denied",
        ),
    ] {
        let result = invoke(&args, key);
        assert!(!result.status.success());
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("ibk_SECRET"));
        assert!(result.stdout.is_empty());
    }
    let bad_timeout = invoke(
        &[
            "instabox",
            "exec",
            "--machine",
            "m_0123456789abcdefghijklmnop",
            "--command",
            "printf test",
            "--timeout-seconds",
            "601",
        ],
        None,
    );
    assert!(!bad_timeout.status.success());
    assert!(
        String::from_utf8(bad_timeout.stderr)
            .unwrap()
            .contains("600")
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    // Invalid synthetic key proves .env is loaded without dispatching a cloud request.
    std::fs::write(
        root.path().join(".env"),
        "INSTABOX_API_KEY=synthetic-file-key\n",
    )
    .unwrap();
    let result = invoke(&["instabox", "discover", "--allow-discovery"], None);
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("invalid InstaBox API key header")
    );
    std::fs::write(root.path().join(".env"), "INSTABOX_API_KEY='SECRET\n").unwrap();
    let denied = invoke(&["instabox", "discover"], None);
    assert!(
        String::from_utf8(denied.stderr)
            .unwrap()
            .contains("discovery denied")
    );
    let overridden = invoke(
        &["instabox", "discover", "--allow-discovery"],
        Some("synthetic-env-key"),
    );
    let error = String::from_utf8(overridden.stderr).unwrap();
    assert!(error.contains("invalid InstaBox API key header"));
    assert!(!error.contains("SECRET"));
}

#[test]
fn foreground_target_is_invocation_only_validated_and_rejected_outside_agent_scope() {
    let root = tempfile::tempdir().unwrap();
    // A malformed synthetic file must not be consulted by denied/invalid scopes.
    std::fs::write(root.path().join(".env"), "INSTABOX_API_KEY='SECRET\n").unwrap();
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_astrid"))
            .env_clear()
            .env("HOME", root.path())
            .current_dir(root.path())
            .args(args)
            .output()
            .unwrap()
    };
    let id = "m_0123456789abcdefghijklmnop";
    let help = invoke(&["run", "--help"]);
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("--instabox-machine")
    );
    for args in [
        vec![
            "--instabox-machine",
            id,
            "run",
            "task",
            "--model",
            "mock",
            "--notes",
        ],
        vec!["--instabox-machine", id, "batch", "trace"],
        vec!["--instabox-machine", id, "models"],
        vec!["--instabox-machine", id, "instabox", "discover"],
    ] {
        let result = invoke(&args);
        assert!(!result.status.success());
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(
            error.contains("requires foreground startup or run without --notes"),
            "{error}"
        );
        assert!(!error.contains("SECRET"));
    }
    for invalid in ["my-machine", "https://example.com", "../usage"] {
        let result = invoke(&[
            "run",
            "task",
            "--model",
            "mock",
            "--instabox-machine",
            invalid,
        ]);
        assert!(!result.status.success());
        assert!(
            String::from_utf8(result.stderr)
                .unwrap()
                .contains("expected exact existing")
        );
    }
    for args in [
        vec!["--instabox-machine", id],
        vec!["run", "task", "--model", "mock", "--instabox-machine", id],
    ] {
        let selected = invoke(&args);
        assert!(!selected.status.success());
        let error = String::from_utf8(selected.stderr).unwrap();
        // Explicit selection reaches only the local key loader, not provider auth or HTTP.
        assert!(
            error.contains("cannot load InstaBox key from .env"),
            "{error}"
        );
        assert!(!error.contains("SECRET"));
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}
