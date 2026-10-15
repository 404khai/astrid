//! Actual CLI subprocess preflight without any provider calls or real credentials.
use std::process::Command;

#[test]
fn notes_help_and_invalid_folders_are_truthful() {
    let binary = env!("CARGO_BIN_EXE_astrid");
    let help = Command::new(binary)
        .args(["run", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("--notes"));
    assert!(text.contains("--folder"));
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("file.txt");
    std::fs::write(&file, "unchanged").unwrap();
    for folder in [root.path().join("missing"), file.clone()] {
        let result = Command::new(binary)
            .args([
                "run",
                "checklist",
                "--notes",
                "--model",
                "mock-m01",
                "--observability",
                "off",
                "--folder",
            ])
            .arg(folder)
            .output()
            .unwrap();
        assert!(!result.status.success());
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains("astrid:"));
        assert!(!error.contains("run          completed"));
        assert!(!error.contains("Trace:"));
    }
    assert_eq!(std::fs::read_to_string(file).unwrap(), "unchanged");
}
