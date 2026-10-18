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
