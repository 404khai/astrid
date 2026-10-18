//! Real CLI preflight only. No real keys or cloud requests.
use std::process::Command;

#[test]
fn discovery_is_explicit_and_execution_is_unimplemented() {
    let root = tempfile::tempdir().unwrap();
    let invoke = |args: &[&str], key: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_astrid"));
        command.env_clear().env("HOME", root.path()).args(args);
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
        (vec!["instabox", "exec"], None, "unrecognized subcommand"),
    ] {
        let result = invoke(&args, key);
        assert!(!result.status.success());
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("ibk_SECRET"));
        assert!(result.stdout.is_empty());
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
