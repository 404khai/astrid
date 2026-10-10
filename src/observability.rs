//! Optional metadata telemetry. No transcript or provider continuation is stored.
use crate::events::{EventKind, ExecutionEvent, ModelCallId, RunId};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, BufRead, Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Switch {
    On,
    #[default]
    Off,
}
impl std::fmt::Display for Switch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if *self == Self::On { "on" } else { "off" })
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub observability: Switch,
    #[serde(default)]
    pub expanded_tool_calls: Switch,
}
impl Settings {
    pub fn load(path: &Path) -> io::Result<Self> {
        Self::load_with_source(path).map(|(settings, _)| settings)
    }
    /// Capture presence and value from one open, so concurrent replacement cannot mislabel the source.
    pub fn load_with_source(path: &Path) -> io::Result<(Self, bool)> {
        match private_open(path, false) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(4097).read_to_end(&mut bytes)?;
                if bytes.len() > 4096 {
                    return Err(io::Error::other("settings exceeds 4096 bytes"));
                }
                let settings = serde_json::from_slice(&bytes)
                    .map_err(|_| io::Error::other("invalid observability settings"))?;
                Ok((settings, true))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok((Self::default(), false)),
            Err(e) => Err(e),
        }
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("settings needs a parent directory"))?;
        private_directory(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
        serde_json::to_writer(&mut file, self)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
    pub fn resolve(&self, override_value: Option<Switch>, exists: bool) -> (Switch, &'static str) {
        match override_value {
            Some(value) => (value, "run flag"),
            None => (
                self.observability,
                if exists {
                    "user setting"
                } else {
                    "built-in default"
                },
            ),
        }
    }
}
pub(crate) fn private_directory(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if !m.is_dir() || m.file_type().is_symlink() => {
            return Err(io::Error::other(
                "storage directory must be a real directory",
            ));
        }
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(path)?;
        }
        Err(e) => return Err(e),
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}
pub(crate) fn private_open(path: &Path, create: bool) -> io::Result<fs::File> {
    let mut opts = fs::OpenOptions::new();
    opts.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .mode(0o600);
    if create {
        opts.write(true).create_new(true);
    } else {
        opts.read(true);
    }
    let file = opts.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("storage must be a regular file"));
    }
    Ok(file)
}

#[derive(Debug, Clone)]
pub struct Options {
    pub directory: PathBuf,
    pub max_run_bytes: u64,
    pub max_store_bytes: u64,
    pub queue_capacity: usize,
    pub finalize_timeout: Duration,
}
impl Options {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            max_run_bytes: 8 * 1024 * 1024,
            max_store_bytes: 256 * 1024 * 1024,
            queue_capacity: 256,
            finalize_timeout: Duration::from_millis(250),
        }
    }
    pub fn valid(&self) -> bool {
        self.max_run_bytes > 0
            && self.max_run_bytes <= 8 * 1024 * 1024
            && self.max_store_bytes > 0
            && self.queue_capacity > 0
            && self.queue_capacity <= 4096
            && self.finalize_timeout <= Duration::from_secs(1)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub invalid: bool,
}
impl Usage {
    pub fn from_response(value: &serde_json::Value) -> Self {
        let Some(v) = value.get("usage").filter(|v| !v.is_null()) else {
            return Self::default();
        };
        if !v.is_object() {
            return Self {
                invalid: true,
                ..Self::default()
            };
        }
        let mut invalid = false;
        let mut count = |v: Option<&serde_json::Value>| match v {
            None | Some(serde_json::Value::Null) => None,
            Some(v) => match v.as_u64() {
                Some(n) => Some(n),
                None => {
                    invalid = true;
                    None
                }
            },
        };
        let input_tokens = count(v.get("input_tokens"));
        let output_tokens = count(v.get("output_tokens"));
        let cached_tokens = count(
            v.get("input_tokens_details")
                .and_then(|v| v.get("cached_tokens")),
        );
        let total = count(v.get("total_tokens"));
        if v.get("input_tokens_details")
            .is_some_and(|v| !v.is_null() && !v.is_object())
        {
            invalid = true;
        }
        if cached_tokens.zip(input_tokens).is_some_and(|(c, i)| c > i)
            || total
                .zip(input_tokens.zip(output_tokens))
                .is_some_and(|(t, (i, o))| i.checked_add(o) != Some(t))
        {
            invalid = true;
        }
        if invalid {
            Self {
                invalid: true,
                ..Self::default()
            }
        } else {
            Self {
                input_tokens,
                output_tokens,
                cached_tokens,
                invalid: false,
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Preparation,
    Authentication,
    Dispatch,
    FirstVisibleText,
    ProviderAttempt,
    DeliveryWait,
    ContextSelection,
    ToolExecution,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Telemetry {
    pub phase: Option<Phase>,
    pub elapsed_us: Option<u64>,
    pub usage: Option<Usage>,
}
impl Telemetry {
    pub fn timing(phase: Phase, elapsed: Duration) -> Self {
        Self {
            phase: Some(phase),
            elapsed_us: Some(micros(elapsed)),
            usage: None,
        }
    }
}
pub fn micros(elapsed: Duration) -> u64 {
    elapsed.as_micros().min(u64::MAX as u128) as u64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "snake_case", deny_unknown_fields)]
pub enum Record {
    Header {
        schema: u32,
        run_id: RunId,
        session_id: crate::events::SessionId,
        started_unix_ms: u64,
        provider: Option<String>,
    },
    Event {
        sequence: u64,
        elapsed_us: u64,
        turn_id: Option<crate::events::TurnId>,
        model_call_id: Option<ModelCallId>,
        tool_call_id: Option<crate::events::ToolCallId>,
        kind: String,
        metadata: serde_json::Value,
    },
    Telemetry {
        sequence: u64,
        elapsed_us: u64,
        model_call_id: Option<ModelCallId>,
        tool_call_id: Option<crate::events::ToolCallId>,
        telemetry: Telemetry,
    },
    Footer {
        sequence: u64,
        complete: bool,
    },
}
fn snapshot_metadata(snapshot: &crate::context::ContextSnapshot) -> serde_json::Value {
    serde_json::json!({"serialized_request_bytes":snapshot.serialized_request_bytes,"non_opaque_json_size_token_heuristic":snapshot.non_opaque_json_size_token_heuristic,"provider_input_tokens":snapshot.provider_input_tokens})
}
fn failure_code(code: &str) -> &str {
    match code {
        "cancelled"
        | "invalid_arguments"
        | "path_denied"
        | "edit_ambiguous"
        | "cleanup_failed"
        | "io_error"
        | "permission_denied"
        | "permission_error"
        | "context"
        | "instructions"
        | "model"
        | "protocol"
        | "cancellation_cleanup_failed"
        | "execution_cleanup_failed" => code,
        _ => "other",
    }
}
fn outcome_metadata(outcome: &crate::model::ToolOutcome) -> serde_json::Value {
    match outcome {
        crate::model::ToolOutcome::Error { code, .. } => {
            serde_json::json!({"error_code":failure_code(code)})
        }
        crate::model::ToolOutcome::Success { data }
        | crate::model::ToolOutcome::TimedOut { data } => {
            serde_json::json!({"exit_code":data.get("exit_code").and_then(|v|v.as_i64())})
        }
    }
}
fn project(event: &ExecutionEvent, elapsed_us: u64) -> Record {
    use EventKind::*;
    let mut metadata = serde_json::json!({});
    let kind = match &event.kind {
        RunStarted { model, .. } => {
            metadata = serde_json::json!({"model":model.chars().take(256).collect::<String>(), "model_label_truncated":model.chars().count()>256});
            "run_started"
        }
        ContextSelected { selection } => {
            metadata = serde_json::json!({"snapshot":snapshot_metadata(&selection.snapshot),"budget":selection.budget,"retained_items":selection.decisions.iter().filter(|d|d.retained).count(),"evicted_items":selection.decisions.iter().filter(|d|!d.retained).count(),"summary_bytes":selection.summary.as_ref().map(|s|s.text_bytes)});
            "context_selected"
        }
        ContextPrepared { snapshot } => {
            metadata = serde_json::json!({"snapshot":snapshot_metadata(snapshot)});
            "context_prepared"
        }
        ToolCallRequested { call } => {
            let name = match call.name.as_str() {
                "read_file" | "write_file" | "edit_file" | "list_directory" | "glob" | "grep"
                | "shell" => Some(call.name.as_str()),
                _ => None,
            };
            metadata = serde_json::json!({"tool":name});
            "tool_call_requested"
        }
        ContextInherited { .. } => "context_inherited",
        ContextItemAdded { .. } => "context_item_added",
        WorkspaceBaseline { .. } => "workspace_baseline",
        WorkspaceChanges { .. } => "workspace_changes",
        PermissionsConfigured { .. } => "permissions_configured",
        CancellationRequested => "cancellation_requested",
        TurnStarted { .. } => "turn_started",
        ModelCallStarted { .. } => "model_call_started",
        ModelFirstTextDelta => "model_first_text_delta",
        ModelTextDelta { .. } => "model_text_delta",
        ModelCallCompleted { .. } => "model_call_completed",
        ModelCallFailed { .. } => "model_call_failed",
        ModelCallCancelled => "model_call_cancelled",
        PermissionRequested { .. } => "permission_requested",
        PermissionGranted => "permission_granted",
        PermissionDenied => "permission_denied",
        PermissionCancelled => "permission_cancelled",
        PermissionFailed { .. } => "permission_failed",
        PermissionPolicyEvaluated { .. } => "permission_policy_evaluated",
        NativeMutationRecorded { .. } => "native_mutation_recorded",
        ToolCallStarted => "tool_call_started",
        ToolOutput { .. } => "tool_output",
        ToolCallCompleted { outcome } => {
            metadata = outcome_metadata(outcome);
            "tool_call_completed"
        }
        ToolCallFailed { outcome } => {
            metadata = outcome_metadata(outcome);
            "tool_call_failed"
        }
        ToolCallDenied { outcome } => {
            metadata = outcome_metadata(outcome);
            "tool_call_denied"
        }
        ToolCallTimedOut { outcome } => {
            metadata = outcome_metadata(outcome);
            "tool_call_timed_out"
        }
        ToolCallCancelled { .. } => "tool_call_cancelled",
        ToolCallSkipped { .. } => "tool_call_skipped",
        TurnCompleted => "turn_completed",
        TurnFailed { .. } => "turn_failed",
        TurnCancelled => "turn_cancelled",
        RunCompleted { .. } => "run_completed",
        RunFailed { code, .. } => {
            metadata = serde_json::json!({"error_code":failure_code(code)});
            "run_failed"
        }
        RunCancelled => "run_cancelled",
        ModelCallLimitReached { .. } => "model_call_limit_reached",
    };
    Record::Event {
        sequence: event.sequence,
        elapsed_us,
        turn_id: event.turn_id,
        model_call_id: event.model_call_id,
        tool_call_id: event.tool_call_id,
        kind: kind.into(),
        metadata,
    }
}
const MAX_RECORD: usize = 16 * 1024;
const MAX_FILES: usize = 4096;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordingStatus {
    Off,
    Complete,
    Incomplete,
    Unavailable,
}
#[derive(Debug, Clone)]
pub struct Report {
    /// Recording-only diagnostic; never contains task or provider error text.
    pub diagnostic: Option<String>,
    pub status: RecordingStatus,
    pub run_id: RunId,
}
/// Never awaits disk I/O on the runtime thread. A saturated queue stops recording.
pub(crate) struct Recorder {
    sender: Option<mpsc::SyncSender<Record>>,
    stopped: Arc<AtomicBool>,
    created: Arc<AtomicBool>,
    done: tokio::sync::oneshot::Receiver<io::Result<()>>,
    timeout: Duration,
    start: Instant,
    run_id: RunId,
}
impl Recorder {
    pub fn start(
        options: Options,
        run_id: RunId,
        session_id: crate::events::SessionId,
        provider: Option<&str>,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(options.queue_capacity);
        let (completed, done) = tokio::sync::oneshot::channel();
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_stopped = stopped.clone();
        let created = Arc::new(AtomicBool::new(false));
        let worker_created = created.clone();
        let timeout = options.finalize_timeout;
        let spawn = std::thread::Builder::new()
            .name("astrid-trace".into())
            .spawn(move || {
                let result =
                    write_trace(&options, run_id, receiver, &worker_stopped, &worker_created);
                if result.is_err() {
                    worker_stopped.store(true, Ordering::Release);
                }
                let _ = completed.send(result);
            });
        if spawn.is_err() {
            stopped.store(true, Ordering::Release);
        }
        let recorder = Self {
            sender: Some(sender),
            stopped,
            created,
            done,
            timeout,
            start: Instant::now(),
            run_id,
        };
        let started_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        recorder.submit(Record::Header {
            schema: 1,
            run_id,
            session_id,
            started_unix_ms,
            provider: provider.map(|s| s.chars().take(256).collect()),
        });
        recorder
    }
    fn submit(&self, record: Record) {
        if self.stopped.load(Ordering::Acquire) {
            return;
        }
        if self
            .sender
            .as_ref()
            .is_some_and(|s| s.try_send(record).is_err())
        {
            self.stopped.store(true, Ordering::Release);
        }
    }
    pub fn event(&self, event: &ExecutionEvent) {
        self.submit(project(event, micros(self.start.elapsed())));
    }
    pub fn telemetry(
        &self,
        sequence: u64,
        model_call_id: Option<ModelCallId>,
        tool_call_id: Option<crate::events::ToolCallId>,
        telemetry: Telemetry,
    ) {
        self.submit(Record::Telemetry {
            sequence,
            elapsed_us: micros(self.start.elapsed()),
            model_call_id,
            tool_call_id,
            telemetry,
        });
    }
    pub async fn finish(mut self, sequence: u64) -> Report {
        self.submit(Record::Footer {
            sequence,
            complete: true,
        });
        self.sender.take();
        let result = tokio::time::timeout(self.timeout, &mut self.done).await;
        let diagnostic = match &result {
            Ok(Ok(Ok(()))) if !self.stopped.load(Ordering::Acquire) => None,
            Ok(Ok(Err(e))) => Some(e.to_string()),
            Err(_) => Some("trace finalization deadline".into()),
            _ => Some("trace queue stopped or worker unavailable".into()),
        };
        let status = if matches!(result, Ok(Ok(Ok(())))) && !self.stopped.load(Ordering::Acquire) {
            RecordingStatus::Complete
        } else {
            self.stopped.store(true, Ordering::Release);
            if self.created.load(Ordering::Acquire) {
                RecordingStatus::Incomplete
            } else {
                RecordingStatus::Unavailable
            }
        };
        Report {
            diagnostic,
            status,
            run_id: self.run_id,
        }
    }
}
struct StoreLock(fs::File);
impl Drop for StoreLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
fn lock_store(directory: &Path) -> io::Result<StoreLock> {
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(directory.join(".lock"))?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("invalid store lock"));
    }
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(io::Error::other(format!(
            "trace store lock: {}",
            io::Error::last_os_error()
        )));
    }
    Ok(StoreLock(file))
}
fn store_size(directory: &Path) -> io::Result<u64> {
    let mut bytes = 0u64;
    for (index, entry) in fs::read_dir(directory)?.enumerate() {
        if index >= MAX_FILES {
            return Err(io::Error::other("trace file count limit"));
        }
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(io::Error::other("unexpected trace store entry"));
        }
        bytes = bytes
            .checked_add(metadata.len())
            .ok_or_else(|| io::Error::other("trace size overflow"))?;
    }
    Ok(bytes)
}
fn write_trace(
    options: &Options,
    run_id: RunId,
    receiver: mpsc::Receiver<Record>,
    stopped: &AtomicBool,
    created: &AtomicBool,
) -> io::Result<()> {
    private_directory(&options.directory)?;
    let partial = options.directory.join(format!("{run_id}.partial"));
    let mut file = {
        let _lock = lock_store(&options.directory)?;
        if store_size(&options.directory)? >= options.max_store_bytes {
            return Err(io::Error::other("trace store full"));
        }
        if fs::read_dir(&options.directory)?
            .take(MAX_FILES - 1)
            .count()
            >= MAX_FILES - 2
        {
            return Err(io::Error::other("trace file count limit"));
        }
        private_open(&partial, true)?
    };
    created.store(true, Ordering::Release);
    let mut run_bytes = 0u64;
    let mut footer = false;
    for record in receiver {
        if stopped.load(Ordering::Acquire) {
            return Err(io::Error::other("recording stopped"));
        }
        let mut bytes = serde_json::to_vec(&record)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_RECORD || run_bytes + bytes.len() as u64 > options.max_run_bytes {
            return Err(io::Error::other("trace record/run limit"));
        }
        let _lock = lock_store(&options.directory)?;
        if store_size(&options.directory)?.saturating_add(bytes.len() as u64)
            > options.max_store_bytes
        {
            return Err(io::Error::other("trace store limit"));
        }
        file.write_all(&bytes)?;
        run_bytes += bytes.len() as u64;
        footer = matches!(record, Record::Footer { complete: true, .. });
    }
    if !footer || stopped.load(Ordering::Acquire) {
        return Err(io::Error::other("trace incomplete"));
    }
    file.sync_all()?;
    if stopped.load(Ordering::Acquire) {
        return Err(io::Error::other("recording stopped"));
    }
    // Publish completion atomically; incomplete prefixes retain the .partial suffix.
    let _lock = lock_store(&options.directory)?;
    let final_path = options.directory.join(format!("{run_id}.jsonl"));
    // link creates the final name without replacing any existing file.
    fs::hard_link(&partial, &final_path)?;
    fs::remove_file(&partial)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Trace {
    pub run_id: RunId,
    pub complete: bool,
    pub records: Vec<Record>,
}
pub fn read_trace(directory: &Path, id: &str) -> io::Result<Trace> {
    let uuid = uuid::Uuid::parse_str(id).map_err(|_| io::Error::other("expected run UUID"))?;
    let path = directory.join(format!("{uuid}.jsonl"));
    check_directory(directory)?;
    let (file, published) = match private_open(&path, false) {
        Ok(file) => (file, true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (
            private_open(&directory.join(format!("{uuid}.partial")), false)?,
            false,
        ),
        Err(e) => return Err(e),
    };
    if file.metadata()?.len() > 8 * 1024 * 1024 {
        return Err(io::Error::other("trace too large to inspect"));
    }
    let mut reader = io::BufReader::new(file.take(8 * 1024 * 1024 + 1));
    let mut read_bytes = 0usize;
    let mut records = Vec::new();
    let mut sequence = 0u64;
    let mut terminal = false;
    let mut complete = false;
    let mut run_id = None;
    let mut last_elapsed = 0;
    loop {
        let mut bytes = Vec::new();
        let count = (&mut reader)
            .take((MAX_RECORD + 1) as u64)
            .read_until(b'\n', &mut bytes)?;
        if count == 0 {
            break;
        }
        read_bytes = read_bytes.saturating_add(count);
        if read_bytes > 8 * 1024 * 1024
            || count > MAX_RECORD
            || bytes.last() != Some(&b'\n')
            || complete
        {
            complete = false;
            break;
        }
        let Ok(record) = serde_json::from_slice::<Record>(&bytes) else {
            break;
        };
        let valid = match &record {
            Record::Header {
                schema, run_id: id, ..
            } => {
                if *schema != 1 {
                    return Err(io::Error::other("unsupported trace schema"));
                }
                if !records.is_empty() || id.to_string() != uuid.to_string() {
                    false
                } else {
                    run_id = Some(*id);
                    true
                }
            }
            Record::Event {
                sequence: s,
                elapsed_us,
                kind,
                ..
            } => {
                if run_id.is_none()
                    || sequence.checked_add(1) != Some(*s)
                    || *elapsed_us < last_elapsed
                    || terminal
                    || (sequence == 0 && kind != "run_started")
                    || (sequence > 0 && kind == "run_started")
                {
                    false
                } else {
                    sequence = *s;
                    last_elapsed = *elapsed_us;
                    terminal = matches!(
                        kind.as_str(),
                        "run_completed"
                            | "run_failed"
                            | "run_cancelled"
                            | "model_call_limit_reached"
                    );
                    true
                }
            }
            Record::Telemetry {
                sequence: s,
                elapsed_us,
                ..
            } => {
                if run_id.is_none() || *s != sequence || *elapsed_us < last_elapsed || terminal {
                    false
                } else {
                    last_elapsed = *elapsed_us;
                    true
                }
            }
            Record::Footer {
                sequence: s,
                complete: c,
            } => *s == sequence && terminal && *c,
        };
        if !valid {
            break;
        }
        complete = matches!(record, Record::Footer { .. });
        records.push(record);
    }
    let run_id = run_id.ok_or_else(|| io::Error::other("missing valid trace header"))?;
    Ok(Trace {
        run_id,
        complete: complete && published,
        records,
    })
}
fn check_directory(directory: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::other("trace store must be a real directory"));
    }
    Ok(())
}
pub fn list_traces(directory: &Path) -> io::Result<Vec<PathBuf>> {
    match check_directory(directory) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let entries = match fs::read_dir(directory) {
        Ok(v) => v,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut paths = Vec::new();
    for (index, e) in entries.enumerate() {
        if index >= MAX_FILES {
            return Err(io::Error::other("trace file count limit"));
        }
        let path = e?.path();
        if path
            .extension()
            .is_some_and(|v| v == "jsonl" || v == "partial")
        {
            paths.push(path);
        }
    }
    paths.sort_by_key(|p| p.file_stem().map(|s| s.to_owned()));
    paths.dedup_by(|a, b| a.file_stem() == b.file_stem());
    Ok(paths)
}

#[derive(Debug, Default, Serialize)]
pub struct CallSummary {
    pub outcome: Option<String>,
    pub error_code: Option<String>,
    pub exit_code: Option<i64>,
    pub inclusive_runtime_us: Option<u64>,
    pub permission_wait_us: Option<u64>,
    pub tool: Option<String>,
    pub usage: Option<Usage>,
    pub timings_us: std::collections::BTreeMap<String, u64>,
    #[serde(skip)]
    start: Option<u64>,
    #[serde(skip)]
    permission_start: Option<u64>,
}
#[derive(Debug, Serialize)]
pub struct Summary {
    pub run_id: RunId,
    pub complete: bool,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub outcome: Option<String>,
    pub run_elapsed_us: Option<u64>,
    pub error_code: Option<String>,
    pub models: std::collections::BTreeMap<ModelCallId, CallSummary>,
    pub tools: std::collections::BTreeMap<crate::events::ToolCallId, CallSummary>,
    pub contexts: Vec<serde_json::Value>,
    pub runtime_timings_us: std::collections::BTreeMap<String, u64>,
    pub cost: Option<f64>,
    pub true_ttft_us: Option<u64>,
    pub decode_tokens_per_second: Option<f64>,
}
impl Trace {
    pub fn summary(&self) -> Summary {
        let mut summary = Summary {
            run_id: self.run_id,
            complete: self.complete,
            model: None,
            provider: None,
            outcome: None,
            run_elapsed_us: None,
            error_code: None,
            models: Default::default(),
            tools: Default::default(),
            contexts: Vec::new(),
            runtime_timings_us: Default::default(),
            cost: None,
            true_ttft_us: None,
            decode_tokens_per_second: None,
        };
        for record in &self.records {
            match record {
                Record::Header { provider, .. } => {
                    summary.provider = provider.clone();
                }
                Record::Event {
                    elapsed_us,
                    model_call_id,
                    tool_call_id,
                    kind,
                    metadata,
                    ..
                } => {
                    if kind == "run_started" {
                        summary.model = metadata["model"].as_str().map(str::to_owned);
                    }
                    if matches!(
                        kind.as_str(),
                        "run_completed"
                            | "run_failed"
                            | "run_cancelled"
                            | "model_call_limit_reached"
                    ) {
                        summary.outcome = Some(kind.clone());
                        summary.error_code = metadata["error_code"].as_str().map(str::to_owned);
                        summary.run_elapsed_us = Some(*elapsed_us);
                    }
                    if kind == "context_selected" || kind == "context_prepared" {
                        summary.contexts.push(metadata.clone());
                    }
                    if let Some(id) = model_call_id {
                        let call = summary.models.entry(*id).or_default();
                        if kind == "model_call_started" {
                            call.start = Some(*elapsed_us);
                        }
                        if matches!(
                            kind.as_str(),
                            "model_call_completed" | "model_call_failed" | "model_call_cancelled"
                        ) {
                            call.outcome = Some(kind.clone());
                            call.error_code = metadata["error_code"].as_str().map(str::to_owned);
                            call.exit_code = metadata["exit_code"].as_i64();
                            call.inclusive_runtime_us =
                                call.start.and_then(|s| elapsed_us.checked_sub(s));
                        }
                    }
                    if let Some(id) = tool_call_id {
                        let call = summary.tools.entry(*id).or_default();
                        if kind == "tool_call_requested" {
                            call.tool = metadata["tool"].as_str().map(str::to_owned);
                        }
                        if kind == "tool_call_started" {
                            call.start = Some(*elapsed_us);
                        }
                        if kind == "permission_requested" {
                            call.permission_start = Some(*elapsed_us);
                        }
                        if matches!(
                            kind.as_str(),
                            "permission_granted"
                                | "permission_denied"
                                | "permission_failed"
                                | "permission_cancelled"
                        ) {
                            call.permission_wait_us = call
                                .permission_start
                                .and_then(|s| elapsed_us.checked_sub(s));
                        }
                        if matches!(
                            kind.as_str(),
                            "tool_call_completed"
                                | "tool_call_failed"
                                | "tool_call_timed_out"
                                | "tool_call_cancelled"
                                | "tool_call_denied"
                                | "tool_call_skipped"
                        ) {
                            call.outcome = Some(kind.clone());
                            call.error_code = metadata["error_code"].as_str().map(str::to_owned);
                            call.exit_code = metadata["exit_code"].as_i64();
                            call.inclusive_runtime_us =
                                call.start.and_then(|s| elapsed_us.checked_sub(s));
                        }
                    }
                }
                Record::Telemetry {
                    model_call_id,
                    tool_call_id,
                    telemetry,
                    ..
                } => {
                    if let (
                        Some(phase @ (Phase::ContextSelection | Phase::DeliveryWait)),
                        Some(elapsed),
                    ) = (telemetry.phase, telemetry.elapsed_us)
                    {
                        let key = serde_json::to_value(phase)
                            .expect("phase serialization")
                            .as_str()
                            .expect("phase string")
                            .to_owned();
                        let entry = summary.runtime_timings_us.entry(key).or_default();
                        *entry = entry.saturating_add(elapsed);
                    }
                    // Tool measurements attach to the tool; provider usage attaches to the model.
                    let call = if telemetry.phase == Some(Phase::ToolExecution) {
                        tool_call_id.map(|id| summary.tools.entry(id).or_default())
                    } else {
                        model_call_id.map(|id| summary.models.entry(id).or_default())
                    };
                    if let Some(call) = call {
                        if let Some(usage) = &telemetry.usage {
                            call.usage = Some(usage.clone());
                        }
                        if let (Some(phase), Some(elapsed)) =
                            (telemetry.phase, telemetry.elapsed_us)
                        {
                            let key = serde_json::to_value(phase)
                                .expect("phase serialization")
                                .as_str()
                                .expect("phase string")
                                .to_owned();
                            let entry = call.timings_us.entry(key).or_default();
                            *entry = entry.saturating_add(elapsed);
                        }
                    }
                }
                _ => {}
            }
        }
        summary
    }
}
#[derive(Debug, Default, Serialize)]
pub struct Stats {
    pub retained_traces: usize,
    pub complete_traces: usize,
    pub incomplete_traces: usize,
    pub unreadable_traces: usize,
    pub observed_model_calls: usize,
    pub input_reporting_calls: usize,
    pub output_reporting_calls: usize,
    pub cache_reporting_calls: usize,
    pub known_input_tokens: u128,
    pub known_output_tokens: u128,
    pub known_cached_tokens: u128,
    pub cost: Option<f64>,
    pub unrecorded_runs: Option<usize>,
}
impl Stats {
    pub fn add(&mut self, summary: &Summary) {
        self.retained_traces += 1;
        if summary.complete {
            self.complete_traces += 1;
        } else {
            self.incomplete_traces += 1;
        }
        self.observed_model_calls += summary.models.len();
        for call in summary.models.values() {
            if let Some(usage) = &call.usage {
                if let Some(v) = usage.input_tokens {
                    self.input_reporting_calls += 1;
                    self.known_input_tokens += v as u128;
                }
                if let Some(v) = usage.output_tokens {
                    self.output_reporting_calls += 1;
                    self.known_output_tokens += v as u128;
                }
                if let Some(v) = usage.cached_tokens {
                    self.cache_reporting_calls += 1;
                    self.known_cached_tokens += v as u128;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(
        sequence: u64,
        elapsed_us: u64,
        kind: &str,
        model: Option<ModelCallId>,
        tool: Option<crate::events::ToolCallId>,
    ) -> Record {
        Record::Event {
            sequence,
            elapsed_us,
            turn_id: None,
            model_call_id: model,
            tool_call_id: tool,
            kind: kind.into(),
            metadata: serde_json::json!({}),
        }
    }
    fn fixture(run: RunId) -> Vec<Record> {
        vec![
            Record::Header {
                schema: 1,
                run_id: run,
                session_id: Default::default(),
                started_unix_ms: 0,
                provider: None,
            },
            event(1, 0, "run_started", None, None),
            event(2, 10, "run_completed", None, None),
            Record::Footer {
                sequence: 2,
                complete: true,
            },
        ]
    }
    fn write_records(path: &Path, records: &[Record]) {
        let mut f = fs::File::create(path).unwrap();
        for record in records {
            serde_json::to_writer(&mut f, record).unwrap();
            f.write_all(b"\n").unwrap();
        }
    }
    #[test]
    fn reader_rejects_schema_gaps_suffixes_symlinks_and_unsafe_ids() {
        let root = tempfile::tempdir().unwrap();
        let id = RunId::default();
        let path = root.path().join(format!("{id}.jsonl"));
        let valid = fixture(id);
        write_records(&path, &valid);
        assert!(read_trace(root.path(), &id.to_string()).unwrap().complete);
        write_records(&path, &valid[..3]);
        assert!(!read_trace(root.path(), &id.to_string()).unwrap().complete);
        let mut gap = valid.clone();
        if let Record::Event { sequence, .. } = &mut gap[2] {
            *sequence = 3;
        }
        write_records(&path, &gap);
        let trace = read_trace(root.path(), &id.to_string()).unwrap();
        assert!(!trace.complete);
        assert_eq!(trace.records.len(), 2);
        write_records(&path, &valid);
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"garbage")
            .unwrap();
        assert!(!read_trace(root.path(), &id.to_string()).unwrap().complete);
        let mut future = valid.clone();
        if let Record::Header { schema, .. } = &mut future[0] {
            *schema = 99;
        }
        write_records(&path, &future);
        assert!(
            read_trace(root.path(), &id.to_string())
                .unwrap_err()
                .to_string()
                .contains("unsupported")
        );
        assert!(read_trace(root.path(), "../credentials").is_err());
        fs::remove_file(&path).unwrap();
        let outside = root.path().join("target");
        write_records(&outside, &valid);
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        assert!(read_trace(root.path(), &id.to_string()).is_err());
        fs::remove_file(&path).unwrap();
        let partial = root.path().join(format!("{id}.partial"));
        write_records(&partial, &valid);
        assert!(!read_trace(root.path(), &id.to_string()).unwrap().complete);
        let link = root.path().join("linked-store");
        std::os::unix::fs::symlink(root.path(), &link).unwrap();
        assert!(list_traces(&link).is_err());
    }
    #[test]
    fn fixed_timeline_separates_permission_execution_and_missing_usage() {
        let id = RunId::default();
        let model = ModelCallId::default();
        let tool = crate::events::ToolCallId::default();
        let records = vec![
            event(1, 0, "run_started", None, None),
            event(2, 10, "model_call_started", Some(model), None),
            event(3, 40, "model_call_completed", Some(model), None),
            event(4, 50, "tool_call_requested", Some(model), Some(tool)),
            event(5, 60, "permission_requested", Some(model), Some(tool)),
            event(6, 90, "permission_denied", Some(model), Some(tool)),
            event(7, 100, "tool_call_denied", Some(model), Some(tool)),
            event(8, 110, "run_completed", None, None),
        ];
        let s = Trace {
            run_id: id,
            complete: true,
            records,
        }
        .summary();
        let m = &s.models[&model];
        let t = &s.tools[&tool];
        assert_eq!(m.inclusive_runtime_us, Some(30));
        assert_eq!(t.permission_wait_us, Some(30));
        assert_eq!(t.inclusive_runtime_us, None);
        assert_eq!(m.usage, None);
        let mut stats = Stats::default();
        stats.add(&s);
        assert_eq!(stats.observed_model_calls, 1);
        assert_eq!(stats.input_reporting_calls, 0);
        assert_eq!(stats.cost, None);
    }
    #[tokio::test]
    async fn stalled_writer_and_queue_saturation_do_not_block_finalization() {
        let (sender, _receiver) = mpsc::sync_channel(1);
        let (_worker, done) = tokio::sync::oneshot::channel();
        let stopped = Arc::new(AtomicBool::new(false));
        let recorder = Recorder {
            sender: Some(sender),
            stopped: stopped.clone(),
            created: Arc::new(AtomicBool::new(true)),
            done,
            timeout: Duration::from_millis(10),
            start: Instant::now(),
            run_id: Default::default(),
        };
        recorder.submit(Record::Footer {
            sequence: 0,
            complete: true,
        });
        recorder.submit(Record::Footer {
            sequence: 0,
            complete: true,
        });
        assert!(stopped.load(Ordering::Acquire));
        let report = tokio::time::timeout(Duration::from_secs(1), recorder.finish(0))
            .await
            .unwrap();
        assert_eq!(report.status, RecordingStatus::Incomplete);
    }
    #[test]
    fn exclusive_store_guard_releases_and_full_store_is_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let first = lock_store(temp.path()).unwrap();
        assert!(lock_store(temp.path()).is_err());
        drop(first);
        assert!(lock_store(temp.path()).is_ok());
        let mut options = Options::new(temp.path().to_path_buf());
        options.max_store_bytes = 16;
        fs::write(temp.path().join("existing.partial"), [0; 16]).unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(sender);
        let result = write_trace(
            &options,
            Default::default(),
            receiver,
            &AtomicBool::new(false),
            &AtomicBool::new(false),
        );
        assert!(result.is_err());
        assert_eq!(store_size(temp.path()).unwrap(), 16);
    }
}
