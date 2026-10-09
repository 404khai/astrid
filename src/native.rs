//! Source-bounded native inspection. Limits describe coverage, never a clean search.
use crate::{cancellation::Cancellation, model::ToolCall, tools::ToolError, workspace::Workspace};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    io::{self, Read},
    path::Path,
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
struct ReadArgs {
    path: String,
    #[serde(default)]
    start_line: Option<usize>,
    #[serde(default)]
    end_line: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrepArgs {
    path: String,
    pattern: String,
    #[serde(default)]
    include_glob: Option<String>,
    #[serde(default)]
    exclude_glob: Option<String>,
    #[serde(default)]
    before_context: Option<usize>,
    #[serde(default)]
    after_context: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
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

/// Scan with a fixed buffer, retaining only the requested range. A distant range
/// still costs prefix IO, bounded separately from returned content.
fn read_file(
    workspace: &Workspace,
    args: ReadArgs,
    cancel: &Cancellation,
) -> Result<Value, ToolError> {
    let start = args.start_line.unwrap_or(1);
    if start == 0 || args.end_line.is_some_and(|end| end < start) {
        return Err(ToolError::InvalidArguments(
            "line range must be 1-based with end_line >= start_line".into(),
        ));
    }
    let path = workspace.resolve(&args.path)?;
    if !fs::symlink_metadata(&path)?.is_file() {
        return Err(ToolError::PathDenied(path.display().to_string()));
    }
    let mut file = fs::File::open(path)?;
    let mut chunk = [0; 8192];
    let mut bytes = Vec::new();
    let mut scanned = 0;
    let mut processed = 0;
    let mut line = 1;
    let mut selected_lines = 0;
    let mut observed = 0;
    let mut range_reached = false;
    let mut last_selected_line = None;
    let mut reached_eof = false;
    let mut range_finished = false;
    let mut reason = None;
    'scan: loop {
        check(cancel)?;
        // One extra byte distinguishes an exact scan ceiling from EOF.
        let remaining = TOTAL_SCAN_LIMIT + 1 - scanned;
        let count = file.read(&mut chunk[..remaining.min(8192)])?;
        scanned += count;
        if count == 0 {
            reached_eof = true;
            range_finished = args.end_line.is_some() && args.end_line == last_selected_line;
            break;
        }
        for &byte in &chunk[..count] {
            processed += 1;
            if processed > TOTAL_SCAN_LIMIT {
                reason = Some("scan_limit");
                break 'scan;
            }
            if line >= start {
                range_reached = true;
                last_selected_line = Some(line);
                observed += 1;
                if bytes.len() == RECORD_BUDGET || selected_lines == RECORD_LIMIT {
                    reason = Some("output_limit");
                    break 'scan;
                }
                bytes.push(byte);
            }
            if byte == b'\n' {
                if line >= start {
                    selected_lines += 1;
                }
                if args.end_line == Some(line) {
                    range_finished = true;
                    break 'scan;
                }
                line += 1;
            }
        }
        if scanned > TOTAL_SCAN_LIMIT {
            reason = Some("scan_limit");
            break;
        }
    }
    check(cancel)?;
    let mut end = bytes.len();
    let content = loop {
        match std::str::from_utf8(&bytes[..end]) {
            Ok(text) => break text.to_owned(),
            Err(error) if error.error_len().is_none() && reason.is_some() => {
                end = error.valid_up_to();
            }
            Err(error) => return Err(io::Error::new(io::ErrorKind::InvalidData, error).into()),
        }
    };
    let mut value = json!({
        "path":args.path, "content":content, "lines":[], "truncated":reason.is_some(),
        "range":{"start_line":start,"end_line":args.end_line},
        "coverage":{"complete":reason.is_none(),"range_start_reached":range_reached,
            "reached_eof":reached_eof,"requested_end_reached":range_finished,
            "scanned_bytes":scanned,"scan_limit_bytes":TOTAL_SCAN_LIMIT,
            "observed_bytes":observed,"retained_bytes":end,
            "omitted_observed_bytes":observed-end,"output_limit_bytes":OUTPUT_LIMIT,
            "truncation_reason":reason,"last_line_partial":false}
    });
    loop {
        let text = value["content"].as_str().unwrap();
        value["lines"] = json!(
            text.lines()
                .enumerate()
                .map(|(index, text)| { json!({"line":start + index,"text":text}) })
                .collect::<Vec<_>>()
        );
        let size = encoded(&value);
        if size <= OUTPUT_LIMIT {
            break;
        }
        let text = value["content"].as_str().unwrap();
        if text.is_empty() {
            return Err(ToolError::InvalidArguments(
                "path metadata exceeds native output limit".into(),
            ));
        }
        // Scale by actual serialized size: escaping can expand each byte sixfold
        // in both content and numbered lines. Subtracting the excess raw bytes
        // would discard the entire useful prefix for heavily escaped text.
        let mut keep = (text.len() * OUTPUT_LIMIT / size).min(text.len() - 1);
        while !text.is_char_boundary(keep) {
            keep -= 1;
        }
        value["content"] = json!(&text[..keep]);
        value["truncated"] = json!(true);
        value["coverage"]["complete"] = json!(false);
        value["coverage"]["truncation_reason"] = json!("output_limit");
        value["coverage"]["retained_bytes"] = json!(keep);
        value["coverage"]["omitted_observed_bytes"] = json!(observed - keep);
    }
    let text = value["content"].as_str().unwrap();
    value["coverage"]["last_line_partial"] =
        json!(value["truncated"] == true && !text.is_empty() && !text.ends_with('\n'));
    Ok(value)
}

fn file_filter(pattern: Option<&str>) -> Result<Option<globset::GlobMatcher>, ToolError> {
    pattern
        .map(|pattern| {
            if Path::new(pattern).is_absolute() || pattern.split('/').any(|part| part == "..") {
                return Err(ToolError::PathDenied(pattern.into()));
            }
            globset::GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .map(|glob| glob.compile_matcher())
                .map_err(|error| ToolError::InvalidArguments(error.to_string()))
        })
        .transpose()
}

/// Context has its own byte budget, so surrounding giant lines cannot inflate
/// one match without bound. Match text itself is never silently clipped.
fn context_lines<'a>(lines: impl Iterator<Item = (usize, &'a str)>) -> (Vec<Value>, bool) {
    let mut result = Vec::new();
    let mut bytes = 0;
    let mut truncated = false;
    for (index, text) in lines {
        if text.len() > LINE_LIMIT {
            truncated = true;
            continue;
        }
        let record = json!({"line":index + 1,"text":text});
        let size = encoded(&record) + 1;
        if bytes + size > 4096 {
            truncated = true;
            continue;
        }
        bytes += size;
        result.push(record);
    }
    (result, truncated)
}

fn grep(workspace: &Workspace, args: GrepArgs, cancel: &Cancellation) -> Result<Value, ToolError> {
    let before = args.before_context.unwrap_or(0);
    let after = args.after_context.unwrap_or(0);
    let offset = args.offset.unwrap_or(0);
    let limit = args.limit.unwrap_or(100);
    if before > 20 || after > 20 || limit == 0 || limit > RECORD_LIMIT {
        return Err(ToolError::InvalidArguments(
            "context must be 0..=20 and limit 1..=1024".into(),
        ));
    }
    let path = workspace.resolve(&args.path)?;
    let pattern = regex::Regex::new(&args.pattern)
        .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
    let include = file_filter(args.include_glob.as_deref())?;
    let exclude = file_filter(args.exclude_glob.as_deref())?;
    let (selection, mut complete) = inventory(workspace, &path, cancel, |path| {
        include.as_ref().is_none_or(|glob| glob.is_match(path))
            && !exclude.as_ref().is_some_and(|glob| glob.is_match(path))
    })?;
    let (files, _, omitted_files) = selection.finish();
    complete &= omitted_files == 0;
    let mut matches = Vec::new();
    let mut match_bytes = 0;
    let mut scanned = 0;
    let mut oversized_files = 0;
    let mut oversized_lines = 0;
    let mut oversized_matches = 0;
    let mut excluded_binary = 0;
    let mut remaining_files = 0;
    let mut matched = 0;
    let mut next_offset = None;
    let mut page_reason = None;
    'files: for (index, file) in files.iter().enumerate() {
        check(cancel)?;
        let relative = file.as_str().unwrap();
        let path = workspace.resolve(relative)?;
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
        let mut history = VecDeque::new();
        let mut lines = content.lines().enumerate();
        while let Some((line_index, line)) = lines.next() {
            check(cancel)?;
            if line.len() > LINE_LIMIT {
                oversized_lines += 1;
                complete = false;
            } else if pattern.is_match(line) {
                let current_offset = matched;
                matched += 1;
                if current_offset >= offset {
                    let mut record = json!({"path":relative,"line":line_index + 1,"text":line});
                    if before > 0 || after > 0 {
                        let (preceding, before_truncated) = context_lines(history.iter().copied());
                        let (following, after_truncated) = context_lines(lines.clone().take(after));
                        record["before"] = json!(preceding);
                        record["after"] = json!(following);
                        record["context_truncated"] = json!(before_truncated || after_truncated);
                    }
                    let size = encoded(&record) + 1;
                    if size > RECORD_BUDGET {
                        oversized_matches += 1;
                        complete = false;
                    } else if matches.len() == limit || match_bytes + size > RECORD_BUDGET {
                        complete = false;
                        remaining_files = files.len() - index;
                        next_offset = Some(current_offset);
                        page_reason = Some(if matches.len() == limit {
                            "match_limit"
                        } else {
                            "output_limit"
                        });
                        break 'files;
                    } else {
                        match_bytes += size;
                        matches.push(record);
                    }
                }
            }
            if before > 0 {
                history.push_back((line_index, line));
                if history.len() > before {
                    history.pop_front();
                }
            }
        }
    }
    check(cancel)?;
    Ok(json!({"matches":matches,"truncated":!complete,
        "page":{"offset":offset,"limit":limit,"next_offset":next_offset,"stop_reason":page_reason},
        "coverage":{"complete":complete,"omitted_inventory_files":omitted_files,
            "oversized_files":oversized_files,"oversized_lines":oversized_lines,
            "oversized_matches":oversized_matches,"excluded_binary_files":excluded_binary,
            "remaining_files":remaining_files,"scanned_bytes":scanned,
            "file_scan_limit_bytes":FILE_SCAN_LIMIT,"total_scan_limit_bytes":TOTAL_SCAN_LIMIT,
            "line_limit_bytes":LINE_LIMIT}}))
}

pub fn invoke(
    workspace: &Workspace,
    call: &ToolCall,
    cancel: &Cancellation,
) -> Result<Value, ToolError> {
    check(cancel)?;
    let result = match call.name.as_str() {
        "read_file" => read_file(workspace, parse(call)?, cancel)?,
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
        "grep" => grep(workspace, parse(call)?, cancel)?,
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
