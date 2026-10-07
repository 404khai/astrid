use astrid::{
    cancellation::Cancellation,
    changes::{capture, capture_path},
    workspace::Workspace,
};
use std::{fs, path::Path, process::Command};
fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(args)
            .status()
            .unwrap()
            .success()
    );
}
fn init(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "fixture@example.test"]);
    git(root, &["config", "user.name", "Fixture"]);
}
#[tokio::test]
async fn dirty_workspace_uses_invocation_baseline_preserves_index_and_reports_shell_edits() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init(root);
    fs::write(root.join("file"), "committed\n").unwrap();
    git(root, &["add", "file"]);
    git(root, &["commit", "-qm", "fixture"]);
    fs::write(root.join("file"), "staged user edit\n").unwrap();
    git(root, &["add", "file"]);
    fs::write(root.join("file"), "unstaged user edit\n").unwrap();
    fs::write(root.join("untracked"), "existing\n").unwrap();
    let index = fs::read(root.join(".git/index")).unwrap();
    let ws = Workspace::new(root).unwrap();
    let cancel = Cancellation::default();
    let before = capture(&ws, &cancel).await;
    assert!(before.git.dirty.iter().any(|p| p.staged && p.unstaged));
    assert!(before.git.dirty.iter().any(|p| p.untracked));
    fs::write(root.join("file"), "run edit\n").unwrap();
    assert!(
        Command::new("sh")
            .current_dir(root)
            .args(["-c", "printf 'shell edit\n' > untracked"])
            .status()
            .unwrap()
            .success()
    );
    let report = before.compare(&capture(&ws, &cancel).await);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    let patch = report
        .changes
        .iter()
        .find(|c| c.path == "utf8:file")
        .unwrap()
        .patch
        .as_ref()
        .unwrap();
    assert!(patch.contains("-unstaged user edit"));
    assert!(!patch.contains("-committed"));
    assert!(report.attribution.contains("authorship is not established"));
    assert_eq!(report.changes.len(), 2);
}
#[tokio::test]
async fn nested_workspace_excludes_parent_and_suppresses_fsmonitor_hooks() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init(root);
    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("local"), "a").unwrap();
    fs::write(root.join("parent"), "a").unwrap();
    let hook = root.join("monitor");
    fs::write(&hook, "#!/bin/sh\ntouch marker\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    git(root, &["config", "core.fsmonitor", hook.to_str().unwrap()]);
    let ws = Workspace::new(nested).unwrap();
    let c = Cancellation::default();
    let a = capture(&ws, &c).await;
    fs::write(root.join("parent"), "b").unwrap();
    fs::write(ws.root().join("local"), "b").unwrap();
    let report = a.compare(&capture(&ws, &c).await);
    assert_eq!(report.changes.len(), 1);
    assert_eq!(report.changes[0].path, "utf8:local");
    assert!(!root.join("marker").exists());
}
#[tokio::test]
async fn ignored_native_evidence_non_git_binary_deletion_and_modes() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let ws = Workspace::new(dir.path()).unwrap();
    let c = Cancellation::default();
    fs::write(dir.path().join("plain"), "old").unwrap();
    fs::write(dir.path().join("binary"), [0, 1]).unwrap();
    let a = capture(&ws, &c).await;
    assert!(!a.git.available);
    fs::set_permissions(dir.path().join("plain"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(dir.path().join("binary"), [0, 2]).unwrap();
    let report = a.compare(&capture(&ws, &c).await);
    assert!(report.changes.iter().any(|c| c.kind == "mode_changed"));
    assert!(
        report
            .changes
            .iter()
            .any(|c| c.unavailable.as_ref().is_some_and(|e| e.contains("Binary")))
    );
    init(dir.path());
    fs::write(dir.path().join(".gitignore"), "ignored\n").unwrap();
    fs::write(dir.path().join("ignored"), "old").unwrap();
    let native = capture_path(&ws, "ignored", &c).await;
    fs::remove_file(dir.path().join("ignored")).unwrap();
    let evidence = native.compare(&capture_path(&ws, "ignored", &c).await);
    assert_eq!(evidence.change.unwrap().kind, "deleted");
}
#[tokio::test]
async fn unusual_names_limits_and_cancelled_inventory_remain_explicit() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    let ws = Workspace::new(dir.path()).unwrap();
    let c = Cancellation::default();
    let non_utf8 = std::ffi::OsStr::from_bytes(b"invalid-\xff");
    let supports_non_utf8 = match fs::write(dir.path().join(non_utf8), "a") {
        Ok(()) => true,
        Err(error) if error.raw_os_error() == Some(libc::EILSEQ) => {
            eprintln!("Filesystem rejects non-UTF-8 names; hex identity is tested independently");
            false
        }
        Err(error) => panic!("{error}"),
    };
    let name = if supports_non_utf8 {
        non_utf8
    } else {
        std::ffi::OsStr::new("unusual\nname")
    };
    if !supports_non_utf8 {
        fs::write(dir.path().join(name), "a").unwrap();
    }
    fs::write(dir.path().join("oversized"), vec![b'a'; 1024 * 1024 + 1]).unwrap();
    let a = capture(&ws, &c).await;
    fs::write(dir.path().join(name), "b").unwrap();
    let report = a.compare(&capture(&ws, &c).await);
    if supports_non_utf8 {
        assert!(report.changes.iter().any(|c| c.path.starts_with("hex:")));
    } else {
        assert!(
            report
                .changes
                .iter()
                .any(|c| c.path.contains("unusual\nname"))
        );
    }
    assert!(!report.complete);
    c.cancel();
    let cancelled = capture(&ws, &c).await;
    assert!(!cancelled.complete);
    assert!(
        !a.compare(&cancelled)
            .changes
            .iter()
            .any(|c| c.kind == "deleted")
    );
}

#[tokio::test]
async fn inventory_cap_cannot_imply_a_deletion_or_clean_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let ws = Workspace::new(dir.path()).unwrap();
    let c = Cancellation::default();
    for n in 0..10_001 {
        fs::write(dir.path().join(format!("entry-{n:05}")), "").unwrap();
    }
    let before = capture(&ws, &c).await;
    assert!(!before.complete);
    fs::remove_file(dir.path().join("entry-00000")).unwrap();
    let after = capture(&ws, &c).await;
    assert!(!before.compare(&after).complete);
    // Missing baseline paths cannot be treated as additions when the baseline was incomplete.
    assert!(
        !before
            .compare(&after)
            .changes
            .iter()
            .any(|change| change.kind == "added")
    );
}

#[tokio::test]
async fn unborn_and_detached_head_remain_available() {
    let dir = tempfile::tempdir().unwrap();
    init(dir.path());
    let ws = Workspace::new(dir.path()).unwrap();
    let c = Cancellation::default();
    let unborn = capture(&ws, &c).await;
    assert!(unborn.git.available);
    assert!(unborn.git.unborn);
    assert!(unborn.git.head.is_none());
    assert!(unborn.git.branch.is_some());
    assert!(!unborn.git.detached);
    fs::write(dir.path().join("file"), "a").unwrap();
    git(dir.path(), &["add", "file"]);
    git(dir.path(), &["commit", "-qm", "fixture"]);
    git(dir.path(), &["checkout", "-q", "--detach"]);
    let detached = capture(&ws, &c).await;
    assert!(detached.git.available);
    assert!(detached.git.detached);
    assert!(!detached.git.unborn);
    assert!(detached.git.head.is_some());
    assert!(detached.git.branch.is_none());
}

#[tokio::test]
async fn git_ignore_membership_does_not_establish_creation_or_deletion() {
    let dir = tempfile::tempdir().unwrap();
    init(dir.path());
    let ws = Workspace::new(dir.path()).unwrap();
    let cancel = Cancellation::default();
    fs::write(dir.path().join("existing"), "user work").unwrap();
    let included = capture(&ws, &cancel).await;
    fs::write(dir.path().join(".gitignore"), "existing\n").unwrap();
    let ignored = capture(&ws, &cancel).await;
    let report = included.compare(&ignored);
    assert!(
        report
            .changes
            .iter()
            .any(|c| c.path == "utf8:existing" && c.kind == "coverage_changed")
    );
    assert!(
        !report
            .changes
            .iter()
            .any(|c| c.path == "utf8:existing" && c.kind == "deleted")
    );
    fs::write(dir.path().join(".gitignore"), "").unwrap();
    let unignored = capture(&ws, &cancel).await;
    let report = ignored.compare(&unignored);
    assert!(
        report
            .changes
            .iter()
            .any(|c| c.path == "utf8:existing" && c.kind == "coverage_changed")
    );
    assert!(
        !report
            .changes
            .iter()
            .any(|c| c.path == "utf8:existing" && c.kind == "added")
    );
}
