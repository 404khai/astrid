use astrid::{cancellation::Cancellation, changes, output::TextDecoder, workspace::Workspace};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn git(root: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}: {:?}", arguments, output);
}

#[tokio::test]
async fn becoming_ignored_is_not_file_deletion() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("user.txt"), "existing user work\n").unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let before = changes::capture(&workspace, &Cancellation::default()).await;
    fs::write(root.path().join(".gitignore"), "user.txt\n").unwrap();
    let after = changes::capture(&workspace, &Cancellation::default()).await;
    let report = before.compare(&after);
    assert_eq!(
        fs::read_to_string(root.path().join("user.txt")).unwrap(),
        "existing user work\n"
    );
    assert!(
        !report
            .changes
            .iter()
            .any(|change| change.path == "utf8:user.txt" && change.kind == "deleted"),
        "inventory membership cannot prove filesystem deletion: {report:?}"
    );
}

#[tokio::test]
async fn becoming_unignored_is_not_file_creation() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("user.txt"), "existing ignored user work\n").unwrap();
    fs::write(root.path().join(".gitignore"), "user.txt\n").unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let before = changes::capture(&workspace, &Cancellation::default()).await;
    fs::write(root.path().join(".gitignore"), "").unwrap();
    let after = changes::capture(&workspace, &Cancellation::default()).await;
    let report = before.compare(&after);
    assert!(
        !report
            .changes
            .iter()
            .any(|change| change.path == "utf8:user.txt" && change.kind == "added"),
        "new inventory membership cannot prove file creation: {report:?}"
    );
}

#[tokio::test]
async fn observation_never_runs_fsmonitor_or_changes_index() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("tracked"), "staged original\n").unwrap();
    git(root.path(), &["add", "tracked"]);
    fs::write(root.path().join("tracked"), "unstaged user work\n").unwrap();
    let hook = root.path().join("monitor-hook");
    fs::write(&hook, "#!/bin/sh\ntouch monitor-was-executed\nexit 1\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    git(root.path(), &["config", "core.fsmonitor", "./monitor-hook"]);
    let index = fs::read(root.path().join(".git/index")).unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let snapshot = changes::capture(&workspace, &Cancellation::default()).await;
    assert!(snapshot.git.available);
    assert!(!root.path().join("monitor-was-executed").exists());
    assert_eq!(fs::read(root.path().join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read_to_string(root.path().join("tracked")).unwrap(),
        "unstaged user work\n"
    );
    assert!(
        snapshot
            .git
            .dirty
            .iter()
            .any(|path| { path.path == "utf8:tracked" && path.staged && path.unstaged })
    );
}

#[tokio::test]
async fn git_status_observation_never_runs_clean_filters() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "--quiet"]);
    fs::write(root.path().join("tracked"), "staged original\n").unwrap();
    git(root.path(), &["add", "tracked"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    );
    fs::write(root.path().join(".gitattributes"), "tracked filter=probe\n").unwrap();
    git(
        root.path(),
        &[
            "config",
            "filter.probe.clean",
            "touch clean-filter-executed; cat",
        ],
    );
    // Same byte length forces Git to inspect content instead of accepting a
    // changed size as sufficient proof of modification.
    fs::write(root.path().join("tracked"), "unstaged change\n").unwrap();
    let workspace = Workspace::new(root.path()).unwrap();
    let snapshot = changes::capture(&workspace, &Cancellation::default()).await;
    assert!(snapshot.git.available);
    assert!(
        !root.path().join("clean-filter-executed").exists(),
        "read-only baseline observation executed a repository clean filter"
    );
    git(
        root.path(),
        &[
            "config",
            "filter.probe.process",
            "touch process-filter-executed; exit 1",
        ],
    );
    let snapshot = changes::capture(&workspace, &Cancellation::default()).await;
    assert!(snapshot.git.available);
    assert!(
        !root.path().join("process-filter-executed").exists(),
        "read-only baseline observation executed a repository process filter"
    );
}

#[test]
fn raw_output_decoder_preserves_every_utf8_split_and_invalid_byte_order() {
    let expected = "no newline: 界🙂é";
    for split in 0..=expected.len() {
        let mut decoder = TextDecoder::default();
        let first = decoder.push(&expected.as_bytes()[..split]);
        let second = decoder.push(&expected.as_bytes()[split..]);
        assert_eq!(first + &second, expected);
    }
    let mut decoder = TextDecoder::default();
    assert_eq!(decoder.push(b"a\xffb\xf0"), "a�b");
    assert_eq!(decoder.push(&[0x9f, 0x99, 0x82, b'c']), "🙂c");
}
