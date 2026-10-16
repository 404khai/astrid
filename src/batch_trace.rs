//! Separate metadata-only correlation. Root `batch-<UUID>` names share the run
//! recorder's exact store lock, total byte/file quotas and atomic publication.
//! No transcript, labels, error text, paths, digests or private state is recorded.
use crate::{
    observability::{self, Options, RecordingStatus},
    scheduler::*,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, BufRead, Read},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildCoverage {
    Off,
    Complete,
    Incomplete,
    Unavailable,
}
impl From<&RecordingStatus> for ChildCoverage {
    fn from(value: &RecordingStatus) -> Self {
        match value {
            RecordingStatus::Off => Self::Off,
            RecordingStatus::Complete => Self::Complete,
            RecordingStatus::Incomplete => Self::Incomplete,
            RecordingStatus::Unavailable => Self::Unavailable,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildOutcome {
    Completed,
    Failed,
    Cancelled,
    CallLimit,
    Lost,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildObservation {
    pub outcome: ChildOutcome,
    pub model_calls: Option<usize>,
    pub tool_calls: Option<usize>,
    pub recording: ChildCoverage,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Batch,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "snake_case", deny_unknown_fields)]
pub enum Record {
    Header {
        family: Family,
        schema: u32,
        batch: BatchId,
        tasks: usize,
        edges: usize,
        concurrency: usize,
        reserved_calls: usize,
    },
    Lifecycle {
        envelope: BatchEvent,
    },
    Footer {
        sequence: usize,
        complete: bool,
    },
}
#[derive(Debug, Clone)]
pub struct Report {
    pub batch: BatchId,
    pub status: RecordingStatus,
    /// Invocation-only writer diagnostic. Never part of persisted batch records.
    pub diagnostic: Option<String>,
}
impl Report {
    pub fn off(batch: BatchId) -> Self {
        Self {
            batch,
            status: RecordingStatus::Off,
            diagnostic: None,
        }
    }
}

pub(crate) struct Recorder {
    batch: BatchId,
    sender: Option<mpsc::SyncSender<Record>>,
    stopped: Arc<AtomicBool>,
    created: Arc<AtomicBool>,
    done: tokio::sync::oneshot::Receiver<io::Result<()>>,
    timeout: std::time::Duration,
}
impl Recorder {
    pub(crate) fn start(options: Options, batch: BatchId, spec: &BatchSpec) -> Self {
        let (sender, receiver) = mpsc::sync_channel(options.queue_capacity);
        let (finished, done) = tokio::sync::oneshot::channel();
        let stopped = Arc::new(AtomicBool::new(false));
        let created = Arc::new(AtomicBool::new(false));
        let thread_stopped = stopped.clone();
        let thread_created = created.clone();
        let timeout = options.finalize_timeout;
        if std::thread::Builder::new()
            .name("astrid-batch-trace".into())
            .spawn(move || {
                let result = observability::write_family(
                    &options,
                    &format!("batch-{batch}"),
                    receiver,
                    &thread_stopped,
                    &thread_created,
                    |r| matches!(r, Record::Footer { complete: true, .. }),
                );
                if result.is_err() {
                    thread_stopped.store(true, Ordering::Release);
                }
                let _ = finished.send(result);
            })
            .is_err()
        {
            stopped.store(true, Ordering::Release);
        }
        let mut recorder = Self {
            batch,
            sender: Some(sender),
            stopped,
            created,
            done,
            timeout,
        };
        recorder.submit(Record::Header {
            family: Family::Batch,
            schema: 1,
            batch,
            tasks: spec.tasks.len(),
            edges: spec.tasks.iter().map(|t| t.depends_on.len()).sum(),
            concurrency: spec.concurrency,
            reserved_calls: spec.tasks.iter().map(|t| t.config.max_model_calls).sum(),
        });
        recorder
    }
    fn submit(&mut self, record: Record) {
        if self.stopped.load(Ordering::Acquire) {
            self.sender = None;
            return;
        }
        if self
            .sender
            .as_ref()
            .is_some_and(|sender| sender.try_send(record).is_err())
        {
            self.stopped.store(true, Ordering::Release);
            self.sender = None;
        }
    }
    pub(crate) fn event(&mut self, event: &BatchEvent) {
        self.submit(Record::Lifecycle {
            envelope: event.clone(),
        });
    }
    pub(crate) async fn finish(mut self, sequence: usize) -> Report {
        self.submit(Record::Footer {
            sequence,
            complete: true,
        });
        self.sender = None;
        let result = tokio::time::timeout(self.timeout, &mut self.done).await;
        let complete = matches!(result, Ok(Ok(Ok(())))) && !self.stopped.load(Ordering::Acquire);
        let diagnostic = match &result {
            Ok(Ok(Ok(()))) if complete => None,
            Ok(Ok(Err(error))) => Some(error.to_string()),
            Err(_) => Some("trace finalization deadline".into()),
            _ => Some("trace queue stopped or worker unavailable".into()),
        };
        if !complete {
            self.stopped.store(true, Ordering::Release);
        }
        Report {
            batch: self.batch,
            diagnostic,
            status: if complete {
                RecordingStatus::Complete
            } else if self.created.load(Ordering::Acquire) {
                RecordingStatus::Incomplete
            } else {
                RecordingStatus::Unavailable
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Trace {
    pub batch: BatchId,
    pub complete: bool,
    pub state: BatchState,
    pub records: Vec<Record>,
}
/// Inspection does not resume jobs or infer child coverage from batch completeness.
pub fn read(directory: &Path, id: &str) -> io::Result<Trace> {
    let uuid = uuid::Uuid::parse_str(id).map_err(|_| io::Error::other("expected batch UUID"))?;
    observability::check_directory(directory)?;
    let name = format!("batch-{uuid}");
    let (file, published) =
        match observability::private_open(&directory.join(format!("{name}.jsonl")), false) {
            Ok(file) => (file, true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (
                observability::private_open(&directory.join(format!("{name}.partial")), false)?,
                false,
            ),
            Err(e) => return Err(e),
        };
    if file.metadata()?.len() > 8 * 1024 * 1024 {
        return Err(io::Error::other("batch trace too large"));
    }
    let mut reader = io::BufReader::new(file.take(8 * 1024 * 1024 + 1));
    let mut records = vec![];
    let mut state = None;
    let mut complete = false;
    let mut header_tasks = 0;
    let mut header_edges = 0;
    let mut header_concurrency = 0;
    let mut header_calls = 0;
    loop {
        let mut bytes = vec![];
        let count = (&mut reader)
            .take(16 * 1024 + 1)
            .read_until(b'\n', &mut bytes)?;
        if count == 0 {
            break;
        }
        if complete
            || count > 16 * 1024
            || bytes.last() != Some(&b'\n')
            || records.len() >= MAX_EVENTS + 2
        {
            complete = false;
            break;
        }
        let Ok(record) = serde_json::from_slice::<Record>(&bytes) else {
            complete = false;
            break;
        };
        let valid = match &record {
            Record::Header {
                schema,
                batch,
                tasks,
                edges,
                concurrency,
                reserved_calls,
                ..
            } => {
                if *schema != 1 {
                    return Err(io::Error::other("unsupported batch schema"));
                }
                if !records.is_empty()
                    || batch.to_string() != uuid.to_string()
                    || !(1..=MAX_TASKS).contains(tasks)
                    || *edges > MAX_EDGES
                    || !(1..=8).contains(concurrency)
                    || !(1..=1024).contains(reserved_calls)
                {
                    false
                } else {
                    state = Some(BatchState::new(*batch));
                    header_tasks = *tasks;
                    header_edges = *edges;
                    header_concurrency = *concurrency;
                    header_calls = *reserved_calls;
                    true
                }
            }
            Record::Lifecycle { envelope } => state.as_mut().is_some_and(|s| {
                if matches!(
                    envelope.event,
                    BatchEventKind::Transition {
                        state: TaskState::Running,
                        ..
                    }
                ) && s
                    .tasks
                    .values()
                    .filter(|t| matches!(t.state, TaskState::Running | TaskState::Cancelling))
                    .count()
                    >= header_concurrency
                {
                    return false;
                }
                if matches!(envelope.event, BatchEventKind::Admitted { .. })
                    && s.tasks.len() >= header_tasks
                {
                    return false;
                }
                s.apply(envelope).is_ok()
            }),
            Record::Footer {
                sequence,
                complete: footer,
            } => state.as_ref().is_some_and(|s| {
                *sequence == s.sequence
                    && s.outcome.is_some()
                    && *footer
                    && s.tasks.len() == header_tasks
                    && s.tasks
                        .values()
                        .map(|t| t.dependencies.len())
                        .sum::<usize>()
                        == header_edges
                    && s.tasks
                        .values()
                        .filter_map(|t| t.child.as_ref().and_then(|c| c.model_calls))
                        .try_fold(0usize, |sum, n| sum.checked_add(n))
                        .is_some_and(|sum| sum <= header_calls)
            }),
        };
        if !valid {
            complete = false;
            break;
        }
        complete = matches!(record, Record::Footer { .. });
        records.push(record);
    }
    let state = state.ok_or_else(|| io::Error::other("missing valid batch header"))?;
    Ok(Trace {
        batch: state.id,
        complete: complete && published,
        state,
        records,
    })
}
pub fn list(directory: &Path) -> io::Result<Vec<PathBuf>> {
    match observability::check_directory(directory) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e),
    }
    let mut paths = vec![];
    for (index, entry) in fs::read_dir(directory)?.enumerate() {
        if index >= 4096 {
            return Err(io::Error::other("trace file count limit"));
        }
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|s| s == "jsonl" || s == "partial")
            && path
                .file_stem()
                .is_some_and(|s| s.to_string_lossy().starts_with("batch-"))
        {
            paths.push(path);
        }
    }
    paths.sort_by_key(|p| p.file_stem().map(|s| s.to_owned()));
    paths.dedup_by(|a, b| a.file_stem() == b.file_stem());
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn saturated_or_failed_writer_reports_coverage_without_blocking() {
        for failed in [false, true] {
            let id = BatchId::default();
            let (sender, _stalled) = mpsc::sync_channel(1);
            let (done_tx, done) = tokio::sync::oneshot::channel();
            let stopped = Arc::new(AtomicBool::new(false));
            let mut recorder = Recorder {
                batch: id,
                sender: Some(sender),
                stopped: stopped.clone(),
                created: Arc::new(AtomicBool::new(failed)),
                done,
                timeout: std::time::Duration::ZERO,
            };
            recorder.submit(Record::Footer {
                sequence: 1,
                complete: true,
            });
            recorder.submit(Record::Footer {
                sequence: 1,
                complete: true,
            });
            if failed {
                let _ = done_tx.send(Err(io::Error::other("synthetic disk failure")));
            }
            let report = recorder.finish(1).await;
            assert!(stopped.load(Ordering::Acquire));
            assert_eq!(
                report.status,
                if failed {
                    RecordingStatus::Incomplete
                } else {
                    RecordingStatus::Unavailable
                }
            );
        }
    }
    #[test]
    fn concurrent_families_share_the_file_count_ceiling() {
        let root = tempfile::tempdir().unwrap();
        for n in 0..4092 {
            fs::File::create(root.path().join(format!("existing-{n}.partial"))).unwrap();
        }
        let options = Options::new(root.path().to_owned());
        let completed = std::thread::scope(|scope| {
            let mut handles = vec![];
            for name in [
                format!("batch-{}", BatchId::default()),
                uuid::Uuid::new_v4().to_string(),
            ] {
                let options = &options;
                handles.push(scope.spawn(move || {
                    let (tx, rx) = mpsc::channel();
                    tx.send("header").unwrap();
                    tx.send("footer").unwrap();
                    drop(tx);
                    observability::write_family(
                        options,
                        &name,
                        rx,
                        &AtomicBool::new(false),
                        &AtomicBool::new(false),
                        |r| *r == "footer",
                    )
                    .is_ok()
                }));
            }
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .filter(|v| *v)
                .count()
        });
        assert_eq!(completed, 1);
        assert!(fs::read_dir(root.path()).unwrap().count() <= 4096);
    }
}
