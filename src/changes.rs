//! Bounded working-tree evidence. Observations do not establish shell authorship.
use crate::{cancellation::Cancellation, workspace::Workspace};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::{io::AsyncReadExt, process::Command};

const FILE_LIMIT: usize = 1024 * 1024;
const TOTAL_LIMIT: usize = 16 * FILE_LIMIT;
const ENTRY_LIMIT: usize = 10_000;
const PATCH_LIMIT: usize = 64 * 1024;
const REPORT_LIMIT: usize = 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitPath {
    pub path: String,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
}
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GitState {
    pub available: bool,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub head: Option<String>,
    pub unborn: bool,
    pub dirty: Vec<GitPath>,
    pub incomplete: bool,
    pub errors: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileEvidence {
    pub path: String,
    pub size: u64,
    pub mode: u32,
    pub unavailable: Option<String>,
    #[serde(skip)]
    content: Option<Vec<u8>>,
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub git: GitState,
    pub complete: bool,
    pub errors: Vec<String>,
    files: BTreeMap<PathBuf, FileEvidence>,
    present: BTreeSet<PathBuf>,
    presence_complete: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub path: String,
    pub kind: String,
    pub before_mode: Option<u32>,
    pub after_mode: Option<u32>,
    pub patch: Option<String>,
    pub unavailable: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceReport {
    pub before_git: GitState,
    pub after_git: GitState,
    pub changes: Vec<Change>,
    pub complete: bool,
    pub errors: Vec<String>,
    pub attribution: String,
}
#[derive(Debug, Clone)]
pub struct PathSnapshot {
    pub path: String,
    pub complete: bool,
    pub error: Option<String>,
    file: Option<FileEvidence>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutationEvidence {
    pub path: String,
    pub change: Option<Change>,
    pub complete: bool,
    pub errors: Vec<String>,
}

// Hex identities preserve distinct non-UTF-8 names; ordinary names retain readable text.
fn label(path: &Path) -> String {
    match path.to_str() {
        Some(s) => format!("utf8:{s}"),
        None => {
            use std::os::unix::ffi::OsStrExt;
            format!(
                "hex:{}",
                path.as_os_str()
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            )
        }
    }
}
fn bytes_path(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}
async fn git(root: &Path, args: &[&str], cancellation: &Cancellation) -> Result<Vec<u8>, String> {
    let mut overrides = Vec::new();
    if args.first() == Some(&"status") {
        match git_raw(
            root,
            &[
                "config",
                "--null",
                "--name-only",
                "--get-regexp",
                "^filter\\..*\\.(clean|process|required)$",
            ],
            cancellation,
            &[],
        )
        .await
        {
            Ok(keys) => {
                if keys.len() > 64 * 1024 {
                    return Err(
                        "Filter configuration exceeds safe audit bound; status unavailable".into(),
                    );
                }
                for key in keys.split(|b| *b == 0).filter(|key| !key.is_empty()) {
                    let key = std::str::from_utf8(key)
                        .map_err(|_| "Non-UTF-8 filter configuration; status unavailable")?;
                    overrides.push(format!(
                        "{key}={}",
                        if key.ends_with(".required") {
                            "false"
                        } else {
                            ""
                        }
                    ));
                }
            }
            Err(error) if error == "Git exit code Some(1)" => {}
            Err(error) => return Err(format!("Cannot safely audit Git filters: {error}")),
        }
    }
    git_raw(root, args, cancellation, &overrides).await
}
async fn git_raw(
    root: &Path,
    args: &[&str],
    cancellation: &Cancellation,
    overrides: &[String],
) -> Result<Vec<u8>, String> {
    if cancellation.is_cancelled() {
        return Err("Git observation cancelled".into());
    }
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-c",
            "core.pager=cat",
            "--literal-pathspecs",
        ])
        .args(overrides.iter().flat_map(|value| ["-c", value.as_str()]))
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env_remove("GIT_CONFIG")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .process_group(0);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Git unavailable: {e}"))?;
    let pid = child.id();
    let mut stdout = child.stdout.take().expect("piped stdout");
    let work = async {
        let mut output = Vec::new();
        let mut chunk = [0; 8192];
        loop {
            let n = stdout.read(&mut chunk).await.map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            if output.len() + n > 2 * FILE_LIMIT {
                return Err("Git output exceeds 2 MiB".into());
            }
            output.extend_from_slice(&chunk[..n]);
        }
        let status = child.wait().await.map_err(|e| e.to_string())?;
        if status.success() {
            Ok(output)
        } else {
            Err(format!("Git exit code {:?}", status.code()))
        }
    };
    let result = tokio::select! {
        result = work => result,
        _ = cancellation.cancelled() => Err("Git observation cancelled".into()),
        _ = tokio::time::sleep(Duration::from_secs(1)) => Err("Git observation timed out".into()),
    };
    if result.is_err() {
        if let Some(pid) = pid {
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
        let _ = child.start_kill();
        if tokio::time::timeout(Duration::from_millis(250), child.wait())
            .await
            .is_err()
        {
            return Err("Git failed cleanup deadline; observation unavailable".into());
        }
    }
    result
}

pub async fn capture(workspace: &Workspace, cancellation: &Cancellation) -> Snapshot {
    if cancellation.is_cancelled() {
        return Snapshot {
            git: GitState::default(),
            complete: false,
            errors: vec!["Workspace observation cancelled".into()],
            files: BTreeMap::new(),
            present: BTreeSet::new(),
            presence_complete: false,
        };
    }
    let started = Instant::now();
    let root = workspace.root().to_path_buf();
    let mut state = GitState::default();
    let mut paths = None;
    match git(&root, &["rev-parse", "--show-toplevel"], cancellation).await {
        Ok(output) => {
            let repository = bytes_path(output.strip_suffix(b"\n").unwrap_or(&output));
            state.available = true;
            state.repository = Some(label(&repository));
            match git(
                &root,
                &["symbolic-ref", "--quiet", "--short", "HEAD"],
                cancellation,
            )
            .await
            {
                Ok(v) => {
                    let bytes = v.strip_suffix(b"\n").unwrap_or(&v);
                    state.branch = Some(match std::str::from_utf8(bytes) {
                        Ok(text) => text.to_owned(),
                        Err(_) => label(&bytes_path(bytes)),
                    });
                }
                // --quiet specifically returns 1 when HEAD is not a symbolic reference.
                Err(e) if e == "Git exit code Some(1)" => state.detached = true,
                Err(e) => {
                    state.incomplete = true;
                    state.errors.push(e);
                }
            }
            match git(
                &root,
                &["rev-parse", "--verify", "--quiet", "HEAD"],
                cancellation,
            )
            .await
            {
                Ok(v) => state.head = Some(String::from_utf8_lossy(&v).trim_end().into()),
                Err(e) if e == "Git exit code Some(1)" && state.branch.is_some() => {
                    state.unborn = true
                }
                Err(e) => {
                    state.incomplete = true;
                    state.errors.push(e);
                }
            }
            match git(
                &root,
                &[
                    "status",
                    "--ignore-submodules=all",
                    "--porcelain=v1",
                    "-z",
                    "--untracked-files=all",
                    "--",
                    ".",
                ],
                cancellation,
            )
            .await
            {
                Ok(v) => {
                    let mut records = v.split(|b| *b == 0).filter(|p| !p.is_empty());
                    while let Some(record) = records.next() {
                        if record.len() < 4 {
                            state.incomplete = true;
                            break;
                        }
                        let x = record[0];
                        let y = record[1];
                        let path = repository.join(bytes_path(&record[3..]));
                        if let Ok(relative) = path.strip_prefix(&root) {
                            if state.dirty.len() == ENTRY_LIMIT {
                                state.incomplete = true;
                                break;
                            }
                            state.dirty.push(GitPath {
                                path: label(relative),
                                staged: x != b' ' && x != b'?',
                                unstaged: y != b' ' && y != b'?',
                                untracked: x == b'?',
                            });
                        }
                        if x == b'R' || x == b'C' || y == b'R' || y == b'C' {
                            records.next();
                        }
                    }
                }
                Err(e) => {
                    state.incomplete = true;
                    state.errors.push(e);
                }
            }
            match git(
                &root,
                &[
                    "ls-files",
                    "-z",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                    "--",
                    ".",
                ],
                cancellation,
            )
            .await
            {
                Ok(v) => {
                    paths = Some(
                        v.split(|b| *b == 0)
                            .filter(|p| !p.is_empty())
                            .map(bytes_path)
                            .take(ENTRY_LIMIT + 1)
                            .collect::<Vec<_>>(),
                    )
                }
                Err(e) => {
                    state.incomplete = true;
                    state.errors.push(e);
                }
            }
        }
        Err(e) => state.errors.push(e),
    }
    let cancelled = cancellation.clone();
    let is_git = state.available;
    let collection = tokio::task::spawn_blocking(move || {
        let mut snapshot = Snapshot {
            git: GitState::default(),
            complete: true,
            errors: vec![],
            files: BTreeMap::new(),
            present: BTreeSet::new(),
            presence_complete: true,
        };
        let mut total = 0;
        let candidates: Box<dyn Iterator<Item = Result<PathBuf, String>>> = match paths {
            Some(paths) => Box::new(paths.into_iter().map(Ok)),
            None if is_git => {
                snapshot.complete = false;
                Box::new(std::iter::empty())
            }
            None => Box::new(
                walkdir::WalkDir::new(&root)
                    .max_open(ENTRY_LIMIT + 1)
                    .into_iter()
                    .filter_entry(|e| e.file_name() != ".git")
                    .map(|e| match e {
                        Ok(e) => e
                            .path()
                            .strip_prefix(&root)
                            .map(Path::to_path_buf)
                            .map_err(|e| e.to_string()),
                        Err(e) => Err(e.to_string()),
                    }),
            ),
        };
        for (count, candidate) in candidates.enumerate() {
            if count == ENTRY_LIMIT || started.elapsed() >= DEADLINE || cancelled.is_cancelled() {
                snapshot.complete = false;
                snapshot.errors.push(
                    "Inventory stopped at entry/time/cancellation limit; missing paths are unknown"
                        .into(),
                );
                break;
            }
            let relative = match candidate {
                Ok(p) => p,
                Err(e) => {
                    snapshot.complete = false;
                    if snapshot.errors.len() < 16 {
                        snapshot.errors.push(e);
                    }
                    continue;
                }
            };
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                snapshot.complete = false;
                continue;
            }
            if fs::symlink_metadata(root.join(&relative)).is_ok_and(|m| m.is_dir()) {
                continue;
            }
            let mut ancestor = root.clone();
            let unsafe_path = relative.components().any(|component| {
                ancestor.push(component);
                fs::symlink_metadata(&ancestor).is_ok_and(|m| m.file_type().is_symlink())
            });
            if unsafe_path {
                snapshot.files.insert(
                    relative.clone(),
                    FileEvidence {
                        path: label(&relative),
                        size: 0,
                        mode: 0,
                        unavailable: Some("Symlink path excluded".into()),
                        content: None,
                    },
                );
                continue;
            }
            match read_evidence(
                &root.join(&relative),
                label(&relative),
                TOTAL_LIMIT.saturating_sub(total),
            ) {
                Ok(Some(evidence)) => {
                    total += evidence.content.as_ref().map_or(0, Vec::len);
                    snapshot.files.insert(relative, evidence);
                }
                Ok(None) => {}
                Err(e) => {
                    snapshot.complete = false;
                    if snapshot.errors.len() < 16 {
                        snapshot.errors.push(e);
                    }
                }
            }
        }
        // Git inclusion is not filesystem existence: ignored paths can enter or leave
        // the content inventory while remaining present throughout the run.
        for (count, entry) in walkdir::WalkDir::new(&root)
            .max_open(ENTRY_LIMIT + 1)
            .into_iter()
            .filter_entry(|e| e.file_name() != ".git")
            .enumerate()
        {
            if count == ENTRY_LIMIT || started.elapsed() >= DEADLINE || cancelled.is_cancelled() {
                snapshot.presence_complete = false;
                snapshot
                    .errors
                    .push("Path-presence inventory incomplete; missing files are unknown".into());
                break;
            }
            match entry {
                Ok(entry) if !entry.file_type().is_dir() => {
                    if let Ok(relative) = entry.path().strip_prefix(&root) {
                        snapshot.present.insert(relative.to_path_buf());
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    snapshot.presence_complete = false;
                    if snapshot.errors.len() < 16 {
                        snapshot.errors.push(error.to_string());
                    }
                }
            }
        }
        snapshot.complete &= snapshot.presence_complete;
        snapshot
    })
    .await;
    let mut snapshot = collection.unwrap_or_else(|e| Snapshot {
        git: GitState::default(),
        complete: false,
        errors: vec![e.to_string()],
        files: BTreeMap::new(),
        present: BTreeSet::new(),
        presence_complete: false,
    });
    snapshot.complete &= !state.incomplete;
    if state.available {
        snapshot.errors.extend(state.errors.iter().cloned());
    }
    snapshot.git = state;
    snapshot
}
fn read_evidence(
    path: &Path,
    name: String,
    remaining: usize,
) -> Result<Option<FileEvidence>, String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut evidence = FileEvidence {
        path: name,
        size: metadata.len(),
        mode: metadata.mode(),
        unavailable: None,
        content: None,
    };
    if !metadata.is_file() {
        evidence.unavailable = Some("Nonregular file; content unavailable".into());
    } else if metadata.len() > FILE_LIMIT as u64 || metadata.len() > remaining as u64 {
        evidence.unavailable = Some("Content exceeds per-file or aggregate snapshot budget".into());
    } else {
        // O_NOFOLLOW plus nonblocking avoids following a raced symlink or blocking on a FIFO.
        use std::os::unix::fs::OpenOptionsExt;
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            evidence.unavailable = Some("File type changed during capture".into());
            return Ok(Some(evidence));
        }
        let limit = FILE_LIMIT.min(remaining);
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > limit {
            evidence.unavailable = Some("File grew beyond snapshot budget".into());
        } else {
            evidence.content = Some(bytes);
        }
    }
    Ok(Some(evidence))
}

pub async fn capture_path(
    workspace: &Workspace,
    path: &str,
    cancellation: &Cancellation,
) -> PathSnapshot {
    let name = label(Path::new(path));
    let resolved = match workspace.resolve(path) {
        Ok(p) => p,
        Err(e) => {
            return PathSnapshot {
                path: name,
                complete: false,
                error: Some(e.to_string()),
                file: None,
            };
        }
    };
    if cancellation.is_cancelled() {
        return PathSnapshot {
            path: name,
            complete: false,
            error: Some("Evidence cancelled".into()),
            file: None,
        };
    }
    let display = label(resolved.strip_prefix(workspace.root()).unwrap_or(&resolved));
    match tokio::task::spawn_blocking(move || read_evidence(&resolved, display, FILE_LIMIT)).await {
        Ok(Ok(file)) => PathSnapshot {
            path: name,
            complete: true,
            error: None,
            file,
        },
        result => PathSnapshot {
            path: name,
            complete: false,
            error: Some(format!("Evidence unavailable: {result:?}")),
            file: None,
        },
    }
}
impl PathSnapshot {
    pub fn compare(&self, end: &Self) -> MutationEvidence {
        let complete = self.complete && end.complete;
        let change = if complete {
            compare_file(&self.path, self.file.as_ref(), end.file.as_ref())
        } else {
            None
        };
        MutationEvidence {
            path: self.path.clone(),
            complete: complete && change.as_ref().is_none_or(|c| c.unavailable.is_none()),
            change,
            errors: self.error.iter().chain(end.error.iter()).cloned().collect(),
        }
    }
}
impl Snapshot {
    pub fn compare(&self, end: &Self) -> WorkspaceReport {
        let keys: BTreeSet<_> = self.files.keys().chain(end.files.keys()).collect();
        let mut report = WorkspaceReport { before_git: bounded_git(&self.git), after_git: bounded_git(&end.git), changes: vec![], complete: self.complete && end.complete, errors: self.errors.iter().chain(end.errors.iter()).cloned().collect(), attribution: "Workspace changes observed during the run; shell/external authorship is not established. Ignored files and paths outside the invocation workspace are excluded.".into() };
        report.complete &= !report.before_git.incomplete && !report.after_git.incomplete;
        let mut retained = serde_json::to_vec(&report).map_or(REPORT_LIMIT, |bytes| bytes.len());
        for key in keys {
            let before = self.files.get(key);
            let after = end.files.get(key);
            if (before.is_none() && !self.complete) || (after.is_none() && !end.complete) {
                continue;
            }
            let membership_changed = (before.is_none() && self.present.contains(key))
                || (after.is_none() && end.present.contains(key));
            let change = if membership_changed {
                Some(Change { path: label(key), kind: "coverage_changed".into(), before_mode: before.map(|f| f.mode), after_mode: after.map(|f| f.mode), patch: None, unavailable: Some("File exists outside one content inventory (for example, ignore rules changed); addition/deletion and content delta cannot be established".into()) })
            } else {
                compare_file(&label(key), before, after)
            };
            if let Some(change) = change {
                let size = serde_json::to_vec(&change).map_or(REPORT_LIMIT, |v| v.len());
                if retained + size + 1024 > REPORT_LIMIT {
                    report.complete = false;
                    report
                        .errors
                        .push("Workspace report reached 1 MiB total serialized budget".into());
                    break;
                }
                retained += size + 1;
                if change.unavailable.is_some() {
                    report.complete = false;
                }
                report.changes.push(change);
            }
        }
        report
    }
}
// Bound metadata separately before reserving the total serialized report budget.
fn bounded_git(state: &GitState) -> GitState {
    const METADATA_LIMIT: usize = 128 * 1024;
    let mut result = state.clone();
    result.dirty.clear();
    for value in [&mut result.repository, &mut result.branch, &mut result.head]
        .into_iter()
        .flatten()
    {
        if value.len() > 8192 {
            let mut end = 8192;
            while !value.is_char_boundary(end) {
                end -= 1;
            }
            value.truncate(end);
            result.incomplete = true;
        }
    }
    let mut retained = serde_json::to_vec(&result).map_or(METADATA_LIMIT, |bytes| bytes.len());
    for path in &state.dirty {
        let bytes = serde_json::to_vec(path).map_or(METADATA_LIMIT, |bytes| bytes.len()) + 1;
        if retained + bytes + 128 > METADATA_LIMIT {
            result.incomplete = true;
            result
                .errors
                .push("Git metadata omitted after 128 KiB report budget".into());
            break;
        }
        retained += bytes;
        result.dirty.push(path.clone());
    }
    result
}
fn compare_file(
    path: &str,
    before: Option<&FileEvidence>,
    after: Option<&FileEvidence>,
) -> Option<Change> {
    if let (Some(a), Some(b)) = (before, after)
        && a.content.is_some()
        && a.content == b.content
        && a.mode == b.mode
    {
        return None;
    }
    let kind = match (before, after) {
        (None, None) => return None,
        (None, _) => "added",
        (_, None) => "deleted",
        (Some(a), Some(b)) if a.content.is_some() && a.content == b.content => "mode_changed",
        _ => "modified_or_unavailable",
    };
    let mut change = Change {
        path: path.into(),
        kind: kind.into(),
        before_mode: before.map(|f| f.mode),
        after_mode: after.map(|f| f.mode),
        patch: None,
        unavailable: None,
    };
    if before
        .iter()
        .chain(after.iter())
        .any(|f| f.content.is_none())
    {
        let reasons = before
            .iter()
            .chain(after.iter())
            .filter_map(|f| f.unavailable.as_deref())
            .collect::<Vec<_>>()
            .join("; ");
        change.unavailable = Some(format!(
            "Before/after content unavailable; content change cannot be established: {reasons}"
        ));
        return Some(change);
    }
    let a = before
        .and_then(|f| f.content.as_deref())
        .unwrap_or_default();
    let b = after.and_then(|f| f.content.as_deref()).unwrap_or_default();
    if a == b {
        return Some(change);
    }
    if a.contains(&0) || b.contains(&0) {
        change.unavailable = Some("Binary content changed; text patch unavailable".into());
        return Some(change);
    }
    let (Ok(a), Ok(b)) = (std::str::from_utf8(a), std::str::from_utf8(b)) else {
        change.unavailable = Some("Non-UTF-8 content changed; text patch unavailable".into());
        return Some(change);
    };
    let mut patch = format!(
        "--- before/{path}\n+++ after/{path}\n@@ -{},{} +{},{} @@\n",
        usize::from(!a.is_empty()),
        a.lines().count(),
        usize::from(!b.is_empty()),
        b.lines().count()
    );
    for (prefix, text) in [("-", a), ("+", b)] {
        for line in text.split_inclusive('\n') {
            let addition = prefix.len()
                + line.len()
                + if line.ends_with('\n') {
                    0
                } else {
                    "\n\\ No newline at end of file\n".len()
                };
            if patch.len() + addition > PATCH_LIMIT {
                change.unavailable = Some(
                    "Whole-file replacement patch exceeds 64 KiB; retained prefix only".into(),
                );
                change.patch = Some(patch);
                return Some(change);
            }
            patch.push_str(prefix);
            patch.push_str(line);
            if !line.ends_with('\n') {
                patch.push_str("\n\\ No newline at end of file\n");
            }
        }
    }
    change.patch = Some(patch);
    Some(change)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serialized_report_includes_git_metadata_in_its_budget() {
        let snapshot = Snapshot {
            git: GitState {
                available: true,
                dirty: (0..10_000)
                    .map(|n| GitPath {
                        path: format!("utf8:{n:05}-{}", "a".repeat(200)),
                        staged: true,
                        unstaged: true,
                        untracked: false,
                    })
                    .collect(),
                ..GitState::default()
            },
            complete: true,
            errors: vec![],
            files: BTreeMap::new(),
            present: BTreeSet::new(),
            presence_complete: true,
        };
        let report = snapshot.compare(&snapshot);
        assert!(serde_json::to_vec(&report).unwrap().len() <= REPORT_LIMIT);
        assert!(!report.complete);
        assert!(report.before_git.incomplete);
    }
    #[test]
    fn non_utf8_identities_cannot_collide_with_literal_hex_names() {
        assert_eq!(label(&bytes_path(b"\xff")), "hex:ff");
        assert_ne!(label(&bytes_path(b"\xff")), label(Path::new("hex:ff")));
        assert_ne!(label(&bytes_path(b"\xff")), label(&bytes_path(b"\xfe")));
    }
}
