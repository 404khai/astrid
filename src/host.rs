//! Application-lifetime client coordination, independent of Tauri and terminal I/O.
//! Runtime futures are host-owned. Visible data is an allowlist, not Session JSON.
pub mod view;
use crate::{
    cancellation::Cancellation,
    events::*,
    model::{Message, ModelProvider},
    runtime::{self, PermissionHandler, RunConfig, RunOutcome},
    session_store::{MAX_SESSIONS, SessionSnapshot, SessionStore, StoredSession},
    tools::{PermissionRequest, ToolExecutor, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};
use tokio::sync::{Notify, oneshot};
use uuid::Uuid;
use view::*;

const MAX_REQUESTS: usize = 32;
const ACK_DEADLINE: Duration = Duration::from_secs(10);
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct HostError(pub String);
type Result<T> = std::result::Result<T, HostError>;
fn error(message: impl Into<String>) -> HostError {
    HostError(message.into())
}
fn authorize(view: &str) -> Result<()> {
    if view == "main" {
        Ok(())
    } else {
        Err(error("unauthorized invoking view"))
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Submit {
    pub request_id: Uuid,
    pub task: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Receipt {
    pub request_id: Uuid,
    pub run_id: Option<RunId>,
    pub session_id: SessionId,
    pub status: String,
}
struct Record {
    digest: [u8; 32],
    receipt: Receipt,
}
struct Folder {
    workspace: Workspace,
    store: SessionStore,
    entries: Vec<StoredSession>,
    selected: usize,
    tools: Arc<dyn ToolExecutor>,
}
struct Pending {
    visible: PermissionView,
    reply: oneshot::Sender<bool>,
}
struct Active {
    receipt: Receipt,
    cancel: Cancellation,
    permission_context: Option<(RunId, ToolCallId)>,
    pending: Option<Pending>,
}
struct Lease {
    id: Uuid,
    cursor: u64,
    outstanding: Option<(u64, u64, Instant)>,
    next_batch: u64,
}
struct State {
    folder: Option<Folder>,
    workflow: Workflow,
    active: Option<Active>,
    last: Option<RunView>,
    requests: HashMap<Uuid, Record>,
    activity: VecDeque<Activity>,
    bytes: usize,
    ordinal: u64,
    lease: Option<Lease>,
    save_error: Option<String>,
    fatal: Option<String>,
    quitting: bool,
    decoders: [crate::output::TextDecoder; 2],
}
pub struct Host {
    provider: Arc<dyn ModelProvider>,
    user_directory: PathBuf,
    model: String,
    state: Mutex<State>,
    changed: Notify,
}
impl Host {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        user_directory: PathBuf,
        model: String,
    ) -> Result<Arc<Self>> {
        if model.trim().is_empty() || model.len() > 256 {
            return Err(error("invalid configured model"));
        }
        Ok(Arc::new(Self {
            provider,
            user_directory,
            model,
            state: Mutex::new(State {
                folder: None,
                workflow: Workflow::Notes,
                active: None,
                last: None,
                requests: HashMap::new(),
                activity: VecDeque::new(),
                bytes: 0,
                ordinal: 0,
                lease: None,
                save_error: None,
                fatal: None,
                quitting: false,
                decoders: Default::default(),
            }),
            changed: Notify::new(),
        }))
    }
    fn lock(&self) -> Result<MutexGuard<'_, State>> {
        self.state
            .lock()
            .map_err(|_| error("host state unavailable"))
    }
    fn idle(s: &State) -> Result<()> {
        if let Some(fatal) = &s.fatal {
            return Err(error(fatal));
        }
        if s.quitting {
            return Err(error("application is quitting"));
        }
        if s.active.is_some() {
            return Err(error("an active run owns the selected workspace"));
        }
        Ok(())
    }
    // Adapter must call blocking folder/store operations away from its UI loop.
    pub fn open_folder(&self, caller: &str, path: &Path) -> Result<()> {
        authorize(caller)?;
        let mut s = self.lock()?;
        Self::idle(&s)?;
        let workspace = Workspace::new(path).map_err(|e| error(e.to_string()))?;
        if workspace.root().to_string_lossy().len() > 2048 {
            return Err(error("workspace path exceeds visible bound"));
        }
        if s.folder
            .as_ref()
            .is_some_and(|f| f.workspace.root() == workspace.root())
        {
            return Ok(());
        }
        if s.folder.is_some() {
            Self::save(&mut s)?;
        }
        // Never drop the current lock or store until a replacement validates.
        let store = SessionStore::open(&self.user_directory, &workspace).map_err(store_error)?;
        let (entries, selected) = match store.load().map_err(store_error)? {
            Some(snapshot) => (snapshot.entries, snapshot.active),
            None => (
                vec![StoredSession {
                    session: runtime::Session::new(&workspace),
                    runs: 0,
                    outcome: None,
                }],
                0,
            ),
        };
        let tools = Arc::new(
            Tools::new(workspace.clone(), Duration::from_secs(60))
                .map_err(|e| error(e.to_string()))?,
        );
        let folder = Folder {
            workspace,
            store,
            entries,
            selected,
            tools,
        };
        Self::save_folder(&folder)?;
        s.folder = Some(folder);
        s.save_error = None;
        s.activity.clear();
        s.bytes = 0;
        s.lease = None;
        s.last = None;
        Ok(())
    }
    fn save_folder(f: &Folder) -> Result<()> {
        // Keep the existing snapshot schema and idle-only one-writer contract.
        let entries = f
            .entries
            .iter()
            .map(|e| StoredSession {
                session: e.session.clone(),
                runs: e.runs,
                outcome: e.outcome.clone(),
            })
            .collect();
        f.store
            .save(&SessionSnapshot::new(&f.workspace, f.selected, entries))
            .map_err(store_error)
    }
    fn save(s: &mut State) -> Result<()> {
        let result = Self::save_folder(
            s.folder
                .as_ref()
                .ok_or_else(|| error("select a folder first"))?,
        );
        s.save_error = result.as_ref().err().map(ToString::to_string);
        result
    }
    pub fn workflow(&self, caller: &str, workflow: Workflow) -> Result<()> {
        authorize(caller)?;
        let mut s = self.lock()?;
        Self::idle(&s)?;
        s.workflow = workflow;
        Ok(())
    }
    pub fn new_session(&self, caller: &str) -> Result<SessionId> {
        authorize(caller)?;
        let mut s = self.lock()?;
        Self::idle(&s)?;
        let f = s
            .folder
            .as_mut()
            .ok_or_else(|| error("select a folder first"))?;
        if f.entries.len() == MAX_SESSIONS {
            return Err(error("session limit reached"));
        }
        let session = runtime::Session::new(&f.workspace);
        let id = session.id;
        f.entries.push(StoredSession {
            session,
            runs: 0,
            outcome: None,
        });
        f.selected = f.entries.len() - 1;
        s.last = None;
        Self::save(&mut s)?;
        Ok(id)
    }
    pub fn select_session(&self, caller: &str, id: SessionId) -> Result<()> {
        authorize(caller)?;
        let mut s = self.lock()?;
        Self::idle(&s)?;
        let f = s
            .folder
            .as_mut()
            .ok_or_else(|| error("select a folder first"))?;
        f.selected = f
            .entries
            .iter()
            .position(|e| e.session.id == id)
            .ok_or_else(|| error("unknown session"))?;
        s.last = None;
        Self::save(&mut s)
    }
    pub fn snapshot(&self, caller: &str) -> Result<Snapshot> {
        authorize(caller)?;
        let s = self.lock()?;
        let sessions = s
            .folder
            .as_ref()
            .map(|f| {
                f.entries
                    .iter()
                    .map(|e| SessionView {
                        id: e.session.id,
                        label: label(&e.session),
                        runs: e.runs,
                        messages: e.session.messages.len(),
                        outcome: e.outcome.as_ref().map(outcome),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Snapshot {
            version: 1,
            workspace: s
                .folder
                .as_ref()
                .map(|f| Text::new(&f.workspace.root().to_string_lossy())),
            model: self.model.clone(),
            provider: self.provider.provider_name().unwrap_or("unavailable"),
            workflow: s.workflow,
            policy: s.workflow.policy(),
            selected_session: s.folder.as_ref().map(|f| f.entries[f.selected].session.id),
            sessions,
            run: s
                .active
                .as_ref()
                .map(active_view)
                .or_else(|| s.last.clone()),
            save_error: s.fatal.clone().or_else(|| s.save_error.clone()),
            quitting: s.quitting,
        })
    }
    pub fn history(&self, caller: &str, offset: usize) -> Result<HistoryPage> {
        authorize(caller)?;
        let s = self.lock()?;
        let f = s
            .folder
            .as_ref()
            .ok_or_else(|| error("select a folder first"))?;
        let messages = &f.entries[f.selected].session.messages;
        if offset > messages.len() {
            return Err(error("invalid history offset"));
        }
        let mut page = HistoryPage {
            version: 1,
            messages: Vec::new(),
            next: Some(offset),
        };
        for (index, message) in messages.iter().enumerate().skip(offset).take(16) {
            page.messages.push(MessageView::project(index, message));
            page.next = (index + 1 < messages.len()).then_some(index + 1);
            if serde_json::to_vec(&page)
                .map_err(|_| error("projection failed"))?
                .len()
                > PAGE_BYTES
            {
                page.messages.pop();
                page.next = Some(index);
                break;
            }
        }
        if offset == messages.len() {
            page.next = None;
        }
        Ok(page)
    }
    pub fn submit(self: &Arc<Self>, caller: &str, intent: Submit) -> Result<Receipt> {
        use sha2::{Digest, Sha256};
        authorize(caller)?;
        if intent.task.trim().is_empty() || intent.task.len() > 64 * 1024 {
            return Err(error("task must contain 1–65536 bytes"));
        }
        let digest: [u8; 32] = Sha256::digest(intent.task.as_bytes()).into();
        let mut s = self.lock()?;
        if let Some(record) = s.requests.get(&intent.request_id) {
            return if record.digest == digest {
                Ok(record.receipt.clone())
            } else {
                Err(error(
                    "request ID already belongs to different task content",
                ))
            };
        }
        Self::idle(&s)?;
        if s.requests.len() == MAX_REQUESTS {
            return Err(error(
                "application request limit reached; retained IDs are never evicted/replayed",
            ));
        }
        let f = s
            .folder
            .as_ref()
            .ok_or_else(|| error("select a folder first"))?;
        let session = f.entries[f.selected].session.clone();
        let tools = f.tools.clone();
        let receipt = Receipt {
            request_id: intent.request_id,
            run_id: None,
            session_id: session.id,
            status: "starting".into(),
        };
        let cancel = Cancellation::default();
        s.active = Some(Active {
            receipt: receipt.clone(),
            cancel: cancel.clone(),
            permission_context: None,
            pending: None,
        });
        s.requests.insert(
            intent.request_id,
            Record {
                digest,
                receipt: receipt.clone(),
            },
        );
        let config = RunConfig {
            observability: None,
            context_budget: Some(Default::default()),
            model: self.model.clone(),
            task: intent.task,
            max_model_calls: 20,
            permissions: s.workflow.policy(),
        };
        drop(s);
        let host = self.clone();
        let request = intent.request_id;
        // Supervisor and runtime are owned by the host, never by IPC call/component futures.
        tokio::spawn(async move {
            let worker_host = host.clone();
            let worker = tokio::spawn(async move {
                let (tx, mut rx) = tokio::sync::mpsc::channel(16);
                let mut approvals = Approvals {
                    host: worker_host.clone(),
                    request,
                };
                let drain = async {
                    while let Some(e) = rx.recv().await {
                        worker_host.record(e);
                    }
                };
                let execute = runtime::run_in_session(
                    worker_host.provider.as_ref(),
                    tools.as_ref(),
                    &mut approvals,
                    config,
                    cancel,
                    Some(tx),
                    session,
                );
                let (result, ()) = tokio::join!(execute, drain);
                result
            });
            match worker.await {
                Ok(result) => host.finish(request, result),
                Err(_) => {
                    if let Ok(mut s) = host.lock() {
                        s.fatal=Some("host worker failed; effects are uncertain; no continuation or recovery is available".into());
                        s.active = None;
                    }
                    host.changed.notify_waiters();
                }
            }
        });
        Ok(receipt)
    }
    fn record(&self, e: ExecutionEvent) {
        if let Ok(mut s) = self.lock() {
            s.ordinal += 1;
            let mut projected = Activity::project(&e, s.ordinal);
            if let EventKind::ToolOutput { output } = &e.kind {
                let stream = usize::from(output.stream == crate::output::OutputStream::Stderr);
                projected.text = Text::new(&s.decoders[stream].push(&output.bytes));
            }
            if matches!(e.kind, EventKind::ToolCallStarted) {
                s.decoders = Default::default();
            }
            if let Some(active) = s.active.as_mut() {
                active.receipt.run_id = Some(e.run_id);
                active.receipt.status = "running".into();
                if matches!(e.kind, EventKind::PermissionRequested { .. }) {
                    active.permission_context = e.tool_call_id.map(|id| (e.run_id, id));
                }
                let id = active.receipt.request_id;
                let receipt = active.receipt.clone();
                if let Some(record) = s.requests.get_mut(&id) {
                    record.receipt = receipt;
                }
            }
            let size = serde_json::to_vec(&projected).map(|b| b.len()).unwrap_or(0);
            s.bytes += size;
            s.activity.push_back(projected);
            while s.bytes > RING_BYTES {
                if let Some(old) = s.activity.pop_front() {
                    s.bytes -= serde_json::to_vec(&old).map(|b| b.len()).unwrap_or(0);
                } else {
                    break;
                }
            }
        }
        self.changed.notify_waiters();
    }
    fn finish(
        &self,
        request: Uuid,
        result: std::result::Result<runtime::RunResult, runtime::SessionStartError>,
    ) {
        if let Ok(mut s) = self.lock() {
            let Some(active) = s.active.take() else {
                return;
            };
            let mut visible = active_view(&active);
            visible.permission = None;
            if let Some(f) = s.folder.as_mut() {
                match result {
                    Ok(result) => {
                        visible.status = outcome(&result.outcome);
                        let entry = &mut f.entries[f.selected];
                        entry.session = result.session;
                        entry.runs += 1;
                        entry.outcome = Some(result.outcome);
                    }
                    Err(rejected) => {
                        visible.status =
                            "rejected: incomplete or invalid session; start a new conversation"
                                .into();
                        f.entries[f.selected].session = *rejected.session;
                    }
                }
            }
            if let Some(record) = s.requests.get_mut(&request) {
                record.receipt.status = visible.status.clone();
            }
            s.last = Some(visible);
            let _ = Self::save(&mut s);
        }
        self.changed.notify_waiters();
    }
    pub fn cancel(&self, caller: &str, request: Uuid) -> Result<()> {
        authorize(caller)?;
        let s = self.lock()?;
        let active = s
            .active
            .as_ref()
            .filter(|a| a.receipt.request_id == request)
            .ok_or_else(|| error("stale or wrong-run cancellation"))?;
        active.cancel.cancel();
        Ok(())
    }
    pub fn permission(
        &self,
        caller: &str,
        id: Uuid,
        run: RunId,
        tool: ToolCallId,
        allow: bool,
    ) -> Result<()> {
        authorize(caller)?;
        let mut s = self.lock()?;
        let active = s
            .active
            .as_mut()
            .ok_or_else(|| error("no active permission"))?;
        let pending = active
            .pending
            .as_ref()
            .filter(|p| {
                p.visible.id == id && p.visible.run_id == run && p.visible.tool_call_id == tool
            })
            .ok_or_else(|| error("stale or wrong-run permission reply"))?;
        if active.cancel.is_cancelled() || (allow && !pending.visible.can_approve) {
            return Err(error("permission cannot be granted"));
        }
        let pending = active
            .pending
            .take()
            .ok_or_else(|| error("permission already answered"))?;
        pending
            .reply
            .send(allow)
            .map_err(|_| error("permission no longer pending"))
    }
    pub fn attach(&self, caller: &str, after: u64) -> Result<Uuid> {
        authorize(caller)?;
        let mut s = self.lock()?;
        if after > s.ordinal {
            return Err(error("invalid activity cursor"));
        }
        let id = Uuid::new_v4();
        s.lease = Some(Lease {
            id,
            cursor: after,
            outstanding: None,
            next_batch: 1,
        });
        Ok(id)
    }
    pub fn batch(&self, caller: &str, id: Uuid, ack: Option<u64>) -> Result<Batch> {
        authorize(caller)?;
        let mut s = self.lock()?;
        let lease = s
            .lease
            .as_mut()
            .filter(|l| l.id == id)
            .ok_or_else(|| error("subscription detached; reattach with gap inspection"))?;
        if let Some((batch, end, sent)) = lease.outstanding {
            if sent.elapsed() > ACK_DEADLINE {
                s.lease = None;
                return Err(error("stalled subscription detached"));
            }
            if ack != Some(batch) {
                return Err(error(
                    "acknowledge the outstanding batch before requesting more data",
                ));
            }
            lease.cursor = end;
            lease.outstanding = None;
        } else if ack.is_some() {
            return Err(error("stale batch acknowledgement"));
        }
        let cursor = lease.cursor;
        let batch_id = lease.next_batch;
        let gap = s.activity.front().is_some_and(|e| e.ordinal > cursor + 1);
        let mut batch = Batch {
            version: 1,
            batch_id,
            events: Vec::new(),
            coverage_gap: gap,
        };
        for event in s.activity.iter().filter(|e| e.ordinal > cursor) {
            batch.events.push(event.clone());
            if serde_json::to_vec(&batch)
                .map_err(|_| error("projection failed"))?
                .len()
                > PAGE_BYTES
            {
                batch.events.pop();
                break;
            }
        }
        let end = batch.events.last().map(|e| e.ordinal).unwrap_or(cursor);
        if let Some(lease) = s.lease.as_mut() {
            lease.next_batch += 1;
            lease.outstanding = Some((batch_id, end, Instant::now()));
        }
        Ok(batch)
    }
    pub fn acknowledge(&self, caller: &str, id: Uuid, batch: u64) -> Result<()> {
        authorize(caller)?;
        let mut s = self.lock()?;
        let lease = s
            .lease
            .as_mut()
            .filter(|l| l.id == id)
            .ok_or_else(|| error("subscription detached"))?;
        let (expected, end, sent) = lease
            .outstanding
            .ok_or_else(|| error("no outstanding batch"))?;
        if sent.elapsed() > ACK_DEADLINE {
            s.lease = None;
            return Err(error("stalled subscription detached"));
        }
        if expected != batch {
            return Err(error("stale batch acknowledgement"));
        }
        lease.cursor = end;
        lease.outstanding = None;
        Ok(())
    }
    pub fn attached(&self, caller: &str, id: Uuid) -> Result<bool> {
        authorize(caller)?;
        let mut s = self.lock()?;
        let attached = s.lease.as_ref().is_some_and(|l| {
            l.id == id
                && l.outstanding
                    .is_none_or(|(_, _, sent)| sent.elapsed() <= ACK_DEADLINE)
        });
        if s.lease.as_ref().is_some_and(|l| l.id == id) && !attached {
            s.lease = None;
        }
        Ok(attached)
    }
    pub fn detach(&self, caller: &str, id: Uuid) -> Result<()> {
        authorize(caller)?;
        let mut s = self.lock()?;
        if s.lease.as_ref().is_some_and(|l| l.id == id) {
            s.lease = None;
        }
        Ok(())
    }
    pub async fn shutdown(&self) -> Result<()> {
        {
            let mut s = self.lock()?;
            s.quitting = true;
            if let Some(active) = &s.active {
                active.cancel.cancel();
            }
        }
        loop {
            let notified = self.changed.notified();
            {
                let mut s = self.lock()?;
                if s.active.is_none() {
                    if let Some(fatal) = &s.fatal {
                        return Err(error(fatal));
                    }
                    let cleanup_failed=s.folder.as_ref().is_some_and(|f|matches!(&f.entries[f.selected].outcome,Some(RunOutcome::Failed {code,..}) if code.ends_with("cleanup_failed")));
                    if cleanup_failed {
                        let _ = Self::save(&mut s);
                        return Err(error(
                            "run cleanup failed; external effects are uncertain; idle save was attempted",
                        ));
                    }
                    return if s.folder.is_some() {
                        Self::save(&mut s)
                    } else {
                        Ok(())
                    };
                }
            }
            notified.await;
        }
    }
}
struct Approvals {
    host: Arc<Host>,
    request: Uuid,
}
#[async_trait]
impl PermissionHandler for Approvals {
    async fn decide(&mut self, request: &PermissionRequest) -> io::Result<bool> {
        let rx = loop {
            let notified = self.host.changed.notified();
            {
                let mut s = self.host.lock().map_err(io::Error::other)?;
                let active = s
                    .active
                    .as_mut()
                    .filter(|a| a.receipt.request_id == self.request)
                    .ok_or_else(|| io::Error::other("run no longer active"))?;
                if let Some((run, tool)) = active.permission_context.take() {
                    let (tx, rx) = oneshot::channel();
                    let command = Text::new(&request.command);
                    let workspace = Text::new(&request.workspace.to_string_lossy());
                    let can_approve = command.omitted_bytes == 0 && workspace.omitted_bytes == 0;
                    active.pending = Some(Pending {
                        visible: PermissionView {
                            id: Uuid::new_v4(),
                            run_id: run,
                            tool_call_id: tool,
                            command,
                            workspace,
                            can_approve,
                            authority: "Native writes are workspace-scoped; shell execution has broad local-account authority",
                        },
                        reply: tx,
                    });
                    break rx;
                }
            }
            notified.await;
        };
        self.host.changed.notify_waiters();
        rx.await
            .map_err(|_| io::Error::other("approval owner disappeared"))
    }
}
fn active_view(active: &Active) -> RunView {
    RunView {
        request_id: active.receipt.request_id,
        run_id: active.receipt.run_id,
        session_id: active.receipt.session_id,
        status: if active.pending.is_some() {
            "waiting_permission".into()
        } else {
            active.receipt.status.clone()
        },
        cancellation_requested: active.cancel.is_cancelled(),
        permission: active.pending.as_ref().map(|p| p.visible.clone()),
    }
}
fn label(session: &runtime::Session) -> String {
    match session.messages.first() {
        Some(Message::User(task)) => task
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(80)
            .collect(),
        _ => "New conversation".into(),
    }
}
fn outcome(result: &RunOutcome) -> String {
    match result {
        RunOutcome::Completed => "completed",
        RunOutcome::Cancelled => "cancelled",
        RunOutcome::Failed { .. } => "failed",
        RunOutcome::ModelCallLimitReached { .. } => "model_call_limit",
    }
    .into()
}

fn store_error(e: io::Error) -> HostError {
    let detail = e.to_string();
    let category = if detail.contains("in use") || e.kind() == io::ErrorKind::WouldBlock {
        "in use by another writer"
    } else if detail.contains("version") {
        "unsupported version"
    } else if detail.contains("byte limit") || detail.contains("too large") {
        "size limit exceeded"
    } else if e.kind() == io::ErrorKind::PermissionDenied {
        "access denied"
    } else {
        "invalid or unavailable"
    };
    error(format!(
        "Session store {category}; existing snapshots are retained. Failed saves do not imply durability."
    ))
}
