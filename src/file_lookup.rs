//! Filename-only lookup shared by clients; independent of terminal I/O and tools.
use crate::{tools::ToolError, workspace::Workspace};
use globset::{GlobBuilder, GlobMatcher};
use ignore::WalkBuilder;
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

// Lookup policy only: do not change native tool or workspace evidence coverage.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".cache",
    ".next",
    ".nuxt",
    ".turbo",
    "__pycache__",
    ".venv",
    "venv",
];

/// Find regular files under a repository-relative `base`, without reading contents.
/// Patterns use `/` separators and match repo-relative paths and basenames.
/// Separator-free patterns also match at any depth. Results use `/`, are unique,
/// lexicographically sorted, and limited *after* selection of the smallest paths.
///
/// The primary pass respects hidden and ignore rules (including parent files).
/// Only when it finds no matches, one fallback includes hidden and ignored files.
/// Generated/dependency directories remain excluded in both passes. Symlinks,
/// including base-path ancestors, are never followed. Traversal/ignore errors are
/// returned, rather than presenting an incomplete lookup as successful.
/// Memory for matches is O(max_hits); both passes may traverse the entire tree.
/// `max_hits == 0` returns immediately, without validation or filesystem access.
pub fn find_files(
    repo_root: &Path,
    base: &Path,
    pattern: &str,
    max_hits: usize,
) -> Result<Vec<PathBuf>, ToolError> {
    if max_hits == 0 {
        return Ok(Vec::new());
    }
    if base.is_absolute() {
        return Err(ToolError::PathDenied(
            "base must be repository-relative".into(),
        ));
    }
    let mut normalized = PathBuf::new();
    for component in base.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir if normalized.pop() => {}
            _ => return Err(ToolError::PathDenied("base escapes repository root".into())),
        }
    }
    let workspace = Workspace::new(repo_root)?;
    let relative = normalized
        .to_str()
        .ok_or_else(|| ToolError::InvalidArguments("base is not UTF-8".into()))?;
    let base = workspace.resolve(if relative.is_empty() { "." } else { relative })?;
    let metadata = std::fs::metadata(&base).map_err(|e| {
        std::io::Error::new(e.kind(), format!("lookup base {}: {e}", base.display()))
    })?;
    if !metadata.is_dir() {
        return Err(ToolError::InvalidArguments(format!(
            "lookup base {} is not a directory",
            base.display()
        )));
    }
    if pattern.is_empty() {
        return Err(ToolError::InvalidArguments("empty file pattern".into()));
    }
    let build = |p: &str| {
        GlobBuilder::new(p)
            .literal_separator(true)
            .build()
            .map(|g| g.compile_matcher())
            .map_err(|e| ToolError::InvalidArguments(format!("invalid file glob {p:?}: {e}")))
    };
    let mut matchers = vec![build(pattern)?];
    if !pattern.contains('/') {
        matchers.push(build(&format!("**/{pattern}"))?);
    }
    if normalized
        .components()
        .any(|c| SKIP_DIRS.iter().any(|s| c.as_os_str() == *s))
    {
        return Ok(Vec::new());
    }
    let primary = walk(workspace.root(), &base, &matchers, max_hits, true)?;
    if !primary.is_empty() {
        return Ok(primary);
    }
    walk(workspace.root(), &base, &matchers, max_hits, false)
}

fn walk(
    root: &Path,
    base: &Path,
    matchers: &[GlobMatcher],
    max: usize,
    primary: bool,
) -> Result<Vec<PathBuf>, ToolError> {
    let mut builder = WalkBuilder::new(base);
    builder
        .standard_filters(primary)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|e| {
            !e.file_type().is_some_and(|t| t.is_dir())
                || !SKIP_DIRS.iter().any(|s| e.file_name() == *s)
        });
    let mut hits = BTreeSet::new();
    for entry in builder.build() {
        let entry =
            entry.map_err(|e| std::io::Error::other(format!("file lookup traversal: {e}")))?;
        if let Some(error) = entry.error() {
            return Err(std::io::Error::other(format!("file lookup ignore rules: {error}")).into());
        }
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| ToolError::PathDenied(entry.path().display().to_string()))?;
        let rendered = render_path(relative)?;
        if matchers
            .iter()
            .any(|m| m.is_match(&rendered) || m.is_match(entry.file_name()))
        {
            hits.insert(rendered);
            // Keep scanning: a later candidate can sort before every retained hit.
            if hits.len() > max {
                hits.pop_last();
            }
        }
    }
    Ok(hits.into_iter().map(PathBuf::from).collect())
}

fn render_path(path: &Path) -> Result<String, ToolError> {
    path.components()
        .map(|c| {
            c.as_os_str()
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| ToolError::InvalidArguments("lookup path is not UTF-8".into()))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("/"))
}
