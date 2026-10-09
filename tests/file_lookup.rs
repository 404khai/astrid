use astrid::file_lookup::find_files;
use std::{
    fs,
    path::{Path, PathBuf},
};
fn write(root: &Path, name: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}
fn find(root: &Path, base: &str, pattern: &str, max: usize) -> Vec<PathBuf> {
    find_files(root, Path::new(base), pattern, max).unwrap()
}
fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}
#[test]
fn names_globs_bases_and_duplicate_matchers() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for name in ["foo.rs", "src/deep/foo.rs", "src/bar.rs", "docs/foo.md"] {
        write(root, name);
    }
    assert_eq!(
        find(root, ".", "foo.rs", 20),
        paths(&["foo.rs", "src/deep/foo.rs"])
    );
    assert_eq!(
        find(root, ".", "src/**/*.rs", 20),
        paths(&["src/bar.rs", "src/deep/foo.rs"])
    );
    assert_eq!(
        find(root, "src/deep/..", "*.rs", 20),
        paths(&["src/bar.rs", "src/deep/foo.rs"])
    );
    assert_eq!(
        find(root, ".", "{foo,bar}.rs", 20),
        paths(&["foo.rs", "src/bar.rs", "src/deep/foo.rs"])
    );
}
#[test]
fn primary_matches_prevent_fallback_and_fallback_includes_hidden_and_ignored() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
    fs::write(root.join(".ignore"), "other/\n").unwrap();
    for name in [
        "visible/foo.rs",
        "ignored/foo.rs",
        "ignored/only.rs",
        ".hidden/secret.rs",
        "other/file.rs",
    ] {
        write(root, name);
    }
    assert_eq!(find(root, ".", "foo.rs", 20), paths(&["visible/foo.rs"]));
    assert_eq!(find(root, ".", "only.rs", 20), paths(&["ignored/only.rs"]));
    assert_eq!(
        find(root, ".", "secret.rs", 20),
        paths(&[".hidden/secret.rs"])
    );
    assert_eq!(find(root, ".", "file.rs", 20), paths(&["other/file.rs"]));
}
#[test]
fn parent_ignores_are_respected() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    fs::write(temp.path().join(".ignore"), "ignored.rs\n").unwrap();
    write(&root, "ignored.rs");
    write(&root, "visible.rs");
    assert_eq!(find(&root, ".", "*.rs", 20), paths(&["visible.rs"]));
}
#[test]
fn fallback_never_visits_generated_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for dir in [
        ".git",
        "node_modules",
        "target",
        "dist",
        "build",
        ".cache",
        ".next",
        ".turbo",
        "__pycache__",
        ".venv",
    ] {
        write(root, &format!("nested/{dir}/needle.rs"));
    }
    assert!(find(root, ".", "needle.rs", 20).is_empty());
    assert!(find(root, "nested/target", "needle.rs", 20).is_empty());
}
#[test]
fn sorting_truncation_are_independent_of_creation_order() {
    for names in [["z.rs", "a.rs", "m.rs"], ["m.rs", "a.rs", "z.rs"]] {
        let temp = tempfile::tempdir().unwrap();
        for name in names {
            write(temp.path(), name);
        }
        assert_eq!(find(temp.path(), ".", "*.rs", 2), paths(&["a.rs", "m.rs"]));
    }
    assert!(
        find_files(Path::new("/missing"), Path::new("/invalid"), "[", 0)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn invalid_patterns_and_bases_are_errors() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "file.rs");
    for (base, pattern) in [
        (".", "["),
        (".", ""),
        ("../", "*"),
        ("nested/../../", "*"),
        ("missing", "*"),
        ("file.rs", "*"),
    ] {
        assert!(
            find_files(root, Path::new(base), pattern, 10).is_err(),
            "{base} {pattern}"
        );
    }
    assert!(find_files(root, root, "*", 10).is_err());
}
#[cfg(unix)]
#[test]
fn symlinks_are_not_followed_or_returned() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "real/foo.rs");
    write(outside.path(), "escape.rs");
    symlink(outside.path(), root.join("linked")).unwrap();
    symlink(root.join("real/foo.rs"), root.join("alias.rs")).unwrap();
    assert_eq!(find(root, ".", "*.rs", 20), paths(&["real/foo.rs"]));
    assert!(find_files(root, Path::new("linked"), "*", 10).is_err());
}
#[test]
fn rendered_paths_use_portable_slashes() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "src/nested/foo.rs");
    let results = find(temp.path(), ".", "foo.rs", 10);
    assert_eq!(results[0].to_str().unwrap(), "src/nested/foo.rs");
}
#[test]
fn malformed_ignore_rules_are_reported() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "foo.rs");
    fs::write(temp.path().join(".ignore"), "{a,b\n").unwrap();
    assert!(find_files(temp.path(), Path::new("."), "*.rs", 10).is_err());
}
