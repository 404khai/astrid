//! Source-bounded native inspection. Limits describe coverage, never a clean search.
use crate::{cancellation::Cancellation, model::ToolCall, tools::ToolError, workspace::Workspace};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub const OUTPUT_LIMIT: usize = 64 * 1024;
const RECORD_BUDGET: usize = 32 * 1024;
const RECORD_LIMIT: usize = 1024;
const INVENTORY_RECORD_LIMIT: usize = 8192;
const FILE_SCAN_LIMIT: usize = 1024 * 1024;
const TOTAL_SCAN_LIMIT: usize = 16 * FILE_SCAN_LIMIT;
const LINE_LIMIT: usize = 16 * 1024;
const MAX_DEPTH: usize = 128;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobArgs {
    pattern: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrepArgs {
    path: String,
    pattern: String,
}
fn parse<T: serde::de::DeserializeOwned>(call: &ToolCall) -> Result<T, ToolError> {
    serde_json::from_str(&call.arguments).map_err(|e| ToolError::InvalidArguments(e.to_string()))
}
fn check(cancel: &Cancellation) -> Result<(), ToolError> {
    if cancel.is_cancelled() {
        Err(ToolError::Cancelled)
    } else {
        Ok(())
    }
}
fn encoded(value: &Value) -> usize {
    value.to_string().len()
}

/// Retain the smallest keys, bounded independently of total inventory size.
#[derive(Default)]
struct Selection {
    records: BTreeMap<String, Value>,
    seen: usize,
    excluded: usize,
}
impl Selection {
    fn add(&mut self, key: String, value: Value) {
        self.seen += 1;
        let size = encoded(&value) + key.len() + 1;
        if size > INVENTORY_RECORD_LIMIT {
            self.excluded += 1;
            return;
        }
        self.records.insert(key, value);
        while self.records.len() > RECORD_LIMIT {
            if self.records.pop_last().is_some() {
                self.excluded += 1;
            }
        }
    }
    fn finish(self) -> (Vec<Value>, usize, usize) {
        let mut values = Vec::new();
        let mut bytes = 0;
        let mut omitted = self.excluded;
        let mut full = false;
        for value in self.records.into_values() {
            let size = encoded(&value) + 1;
            if full || bytes + size > RECORD_BUDGET {
                full = true;
                omitted += 1;
            } else {
                bytes += size;
                values.push(value);
            }
        }
        (values, self.seen, omitted)
    }
}

fn read_prefix(path: &Path, limit: usize, cancel: &Cancellation) -> Result<Vec<u8>, ToolError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(ToolError::PathDenied(path.display().to_string()));
    }
    let mut file = fs::File::open(path)?;
    let mut result = Vec::with_capacity(limit.min(8192));
    let mut chunk = [0; 8192];
    while result.len() < limit {
        check(cancel)?;
        let remaining = (limit - result.len()).min(chunk.len());
        let count = file.read(&mut chunk[..remaining])?;
        if count == 0 {
            break;
        }
        result.extend_from_slice(&chunk[..count]);
    }
    Ok(result)
}

fn inventory(
    workspace: &Workspace,
    path: &Path,
    cancel: &Cancellation,
    predicate: impl Fn(&str) -> bool,
) -> Result<(Selection, bool), ToolError> {
    let metadata = fs::symlink_metadata(path)?;
    let mut selection = Selection::default();
    if metadata.is_file() {
        if path
            .strip_prefix(workspace.root())
            .unwrap_or(path)
            .to_str()
            .is_none()
        {
            selection.seen += 1;
            selection.excluded += 1;
            return Ok((selection, false));
        }
        let relative = workspace.relative(path);
        if predicate(&relative) {
            selection.add(relative.clone(), json!(relative));
        }
        return Ok((selection, true));
    }
    if !metadata.is_dir() {
        return Err(ToolError::PathDenied(path.display().to_string()));
    }
    let mut complete = true;
    // No sort_by: it eagerly collects entire directories. WalkDir keeps open
    // iterators bounded; depth is bounded too, with explicit incomplete coverage.
    for entry in WalkDir::new(path)
        .follow_links(false)
        .max_open(MAX_DEPTH)
        .max_depth(MAX_DEPTH)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != ".git")
    {
        check(cancel)?;
        let entry = entry.map_err(|e| ToolError::Io(io::Error::other(e)))?;
        if entry.depth() == MAX_DEPTH && entry.file_type().is_dir() {
            complete = false;
        }
        if entry.file_type().is_file() {
            if entry
                .path()
                .strip_prefix(workspace.root())
                .unwrap_or(entry.path())
                .to_str()
                .is_none()
            {
                selection.seen += 1;
                selection.excluded += 1;
                complete = false;
                continue;
            }
            let relative = workspace.relative(entry.path());
            workspace.resolve(&relative)?;
            if predicate(&relative) {
                selection.add(relative.clone(), json!(relative));
            }
        }
    }
    complete &= selection.excluded == 0;
    Ok((selection, complete))
}

pub fn invoke(
    workspace: &Workspace,
    call: &ToolCall,
    cancel: &Cancellation,
) -> Result<Value, ToolError> {
    check(cancel)?;
    let result = match call.name.as_str() {
        "read_file" => {
            let args: PathArgs = parse(call)?;
            let path = workspace.resolve(&args.path)?;
            let bytes = read_prefix(&path, OUTPUT_LIMIT + 1, cancel)?;
            let observed = bytes.len();
            let mut end = observed.min(OUTPUT_LIMIT);
            // An incomplete code point at the cap is omission, invalid interior
            // UTF-8 remains a recoverable IO error as in the original tool.
            let content = loop {
                match std::str::from_utf8(&bytes[..end]) {
                    Ok(text) => break text.to_owned(),
                    Err(e) if e.error_len().is_none() && observed > OUTPUT_LIMIT => {
                        end = e.valid_up_to()
                    }
                    Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e).into()),
                }
            };
            let mut value = json!({"path": args.path, "content":content,"truncated": observed > end,
                "coverage":{"complete":observed <= end,"observed_bytes":observed,"retained_bytes":end,"omitted_observed_bytes":observed-end,"output_limit_bytes":OUTPUT_LIMIT}});
            // JSON escaping can expand raw content sixfold. Fit the actual JSON.
            while encoded(&value) > OUTPUT_LIMIT {
                let text = value["content"].as_str().unwrap();
                if text.is_empty() {
                    return Err(ToolError::InvalidArguments(
                        "path metadata exceeds native output limit".into(),
                    ));
                }
                let mut keep = text
                    .len()
                    .saturating_sub((encoded(&value) - OUTPUT_LIMIT).max(1));
                while !text.is_char_boundary(keep) {
                    keep -= 1;
                }
                value["content"] = json!(&text[..keep]);
                value["truncated"] = json!(true);
                value["coverage"]["complete"] = json!(false);
                value["coverage"]["retained_bytes"] = json!(keep);
                value["coverage"]["omitted_observed_bytes"] = json!(observed - keep);
            }
            value
        }
        "list_directory" => {
            let args: PathArgs = parse(call)?;
            let path = workspace.resolve(&args.path)?;
            let mut selection = Selection::default();
            for entry in fs::read_dir(path)? {
                check(cancel)?;
                let entry = entry?;
                let kind = entry.file_type()?;
                let file_name = entry.file_name();
                let Some(name) = file_name.to_str() else {
                    selection.seen += 1;
                    selection.excluded += 1;
                    continue;
                };
                let name = name.to_owned();
                selection.add(name.clone(), json!({"name":name,"kind":if kind.is_symlink(){"symlink"}else if kind.is_dir(){"directory"}else if kind.is_file(){"file"}else{"other"}}));
            }
            let (entries, seen, omitted) = selection.finish();
            json!({"path":args.path,"entries":entries,"truncated":omitted>0,
                "coverage":{"complete":omitted==0,"observed_entries":seen,"omitted_entries":omitted}})
        }
        "glob" => {
            let args: GlobArgs = parse(call)?;
            if Path::new(&args.pattern).is_absolute()
                || args.pattern.split('/').any(|part| part == "..")
            {
                return Err(ToolError::PathDenied(args.pattern));
            }
            let pattern = globset::GlobBuilder::new(&args.pattern)
                .literal_separator(true)
                .build()
                .map_err(|e| ToolError::InvalidArguments(e.to_string()))?
                .compile_matcher();
            let (selection, complete) = inventory(workspace, workspace.root(), cancel, |path| {
                pattern.is_match(path)
            })?;
            let (files, seen, omitted) = selection.finish();
            let complete = complete && omitted == 0;
            json!({"files":files,"truncated":!complete,
                "coverage":{"complete":complete,"observed_files":seen,"omitted_files":omitted,"max_depth":MAX_DEPTH}})
        }
        "grep" => {
            let args: GrepArgs = parse(call)?;
            let path = workspace.resolve(&args.path)?;
            let pattern = regex::Regex::new(&args.pattern)
                .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
            let (selection, mut complete) = inventory(workspace, &path, cancel, |_| true)?;
            let (files, _, omitted_files) = selection.finish();
            complete &= omitted_files == 0;
            let mut matches = Vec::new();
            let mut match_bytes = 0;
            let mut scanned = 0;
            let mut oversized_files = 0;
            let mut oversized_lines = 0;
            let mut excluded_binary = 0;
            let mut remaining_files = 0;
            for (index, file) in files.iter().enumerate() {
                check(cancel)?;
                let relative = file.as_str().unwrap();
                let path: PathBuf = workspace.resolve(relative)?;
                let size = fs::symlink_metadata(&path)?.len();
                if size > FILE_SCAN_LIMIT as u64 {
                    oversized_files += 1;
                    complete = false;
                    continue;
                }
                if scanned + size as usize > TOTAL_SCAN_LIMIT {
                    remaining_files = files.len() - index;
                    complete = false;
                    break;
                }
                let read_limit = (FILE_SCAN_LIMIT + 1).min(TOTAL_SCAN_LIMIT - scanned);
                let bytes = read_prefix(&path, read_limit, cancel)?;
                scanned += bytes.len();
                if read_limit <= FILE_SCAN_LIMIT && bytes.len() == read_limit {
                    complete = false;
                    remaining_files = files.len() - index;
                    break;
                }
                if bytes.len() > FILE_SCAN_LIMIT {
                    oversized_files += 1;
                    complete = false;
                    continue;
                }
                if bytes.contains(&0) {
                    excluded_binary += 1;
                    continue;
                }
                let Ok(content) = std::str::from_utf8(&bytes) else {
                    excluded_binary += 1;
                    continue;
                };
                for (line_index, line) in content.lines().enumerate() {
                    check(cancel)?;
                    if line.len() > LINE_LIMIT {
                        oversized_lines += 1;
                        complete = false;
                        continue;
                    }
                    if pattern.is_match(line) {
                        let record = json!({"path":relative,"line":line_index+1,"text":line});
                        let size = encoded(&record) + 1;
                        if match_bytes + size > RECORD_BUDGET || matches.len() >= RECORD_LIMIT {
                            complete = false;
                            remaining_files = files.len() - index;
                            break;
                        }
                        match_bytes += size;
                        matches.push(record);
                    }
                }
                if remaining_files > 0 {
                    break;
                }
            }
            json!({"matches":matches,"truncated":!complete,"coverage":{"complete":complete,
                "omitted_inventory_files":omitted_files,"oversized_files":oversized_files,
                "oversized_lines":oversized_lines,"excluded_binary_files":excluded_binary,
                "remaining_files":remaining_files,"scanned_bytes":scanned,"file_scan_limit_bytes":FILE_SCAN_LIMIT,
                "total_scan_limit_bytes":TOTAL_SCAN_LIMIT,"line_limit_bytes":LINE_LIMIT}})
        }
        _ => {
            return Err(ToolError::InvalidArguments(format!(
                "not a native inspection tool: {}",
                call.name
            )));
        }
    };
    if encoded(&result) > OUTPUT_LIMIT {
        return Err(ToolError::InvalidArguments(
            "result metadata exceeds native output limit".into(),
        ));
    }
    Ok(result)
}
