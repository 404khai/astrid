//! Versioned workspace session storage, separate from metadata-only traces.
//! Contains conversation/tool content and provider continuation. No auth credentials,
//! saved permissions, terminal I/O, running tasks, or replay of tool side effects.
//! Authentication state is excluded; user/tool content may itself contain secrets.
use crate::{
    observability::{private_directory, private_open},
    runtime::{RunOutcome, Session},
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::PathBuf,
};

pub const MAX_SESSIONS: usize = 32;
const VERSION: u32 = 1;
const MAX_STORE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_SESSION_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredSession {
    pub session: Session,
    pub runs: usize,
    pub outcome: Option<RunOutcome>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionSnapshot {
    version: u32,
    workspace: PathBuf,
    pub active: usize,
    pub entries: Vec<StoredSession>,
}
impl SessionSnapshot {
    pub fn new(workspace: &Workspace, active: usize, entries: Vec<StoredSession>) -> Self {
        Self {
            version: VERSION,
            workspace: workspace.root().to_path_buf(),
            active,
            entries,
        }
    }
    fn validate(&self, workspace: &Workspace) -> io::Result<()> {
        if self.version != VERSION {
            return Err(io::Error::other("unsupported session store version"));
        }
        if self.workspace != workspace.root() {
            return Err(io::Error::other("session store workspace mismatch"));
        }
        if self.entries.is_empty()
            || self.entries.len() > MAX_SESSIONS
            || self.active >= self.entries.len()
        {
            return Err(io::Error::other("invalid session list or active selection"));
        }
        let mut ids = BTreeSet::new();
        for entry in &self.entries {
            if !ids.insert(entry.session.id) {
                return Err(io::Error::other("duplicate stored session ID"));
            }
            if (entry.runs == 0) != entry.session.messages.is_empty()
                || (entry.runs == 0) != entry.outcome.is_none()
            {
                return Err(io::Error::other(
                    "stored run metadata differs from conversation",
                ));
            }
            if entry.runs
                != entry
                    .session
                    .messages
                    .iter()
                    .filter(|message| matches!(message, crate::model::Message::User(_)))
                    .count()
            {
                return Err(io::Error::other(
                    "stored run count differs from submitted messages",
                ));
            }
            entry
                .session
                .validate_persisted(workspace)
                .map_err(io::Error::other)?;
            serde_json::to_writer(Limited::new(io::sink(), MAX_SESSION_BYTES), entry)
                .map_err(io::Error::other)?;
        }
        Ok(())
    }
}

/// Owns the exclusive writer lock until dropped. Use one store per canonical
/// workspace, and save only idle snapshots. Validation/serialization failures
/// retain the old file; a sync failure after publication leaves durability uncertain.
pub struct SessionStore {
    directory: PathBuf,
    workspace: Workspace,
    _lock: fs::File,
}
impl SessionStore {
    pub fn open(user_directory: &std::path::Path, workspace: &Workspace) -> io::Result<Self> {
        let parent = user_directory.join("sessions");
        private_directory(&parent)?;
        let key = format!(
            "{:x}",
            Sha256::digest(workspace.root().as_os_str().as_bytes())
        );
        let directory = parent.join(key);
        private_directory(&directory)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(directory.join(".lock"))?;
        if !lock.metadata()?.is_file() {
            return Err(io::Error::other("session lock must be a regular file"));
        }
        lock.set_permissions(fs::Permissions::from_mode(0o600))?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "another Astrid process is using this workspace's session store",
            ));
        }
        Ok(Self {
            directory,
            workspace: workspace.clone(),
            _lock: lock,
        })
    }
    pub fn load(&self) -> io::Result<Option<SessionSnapshot>> {
        let file = match private_open(&self.directory.join("state.json"), false) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_STORE_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_STORE_BYTES {
            return Err(io::Error::other("session store exceeds 32 MiB"));
        }
        let snapshot: SessionSnapshot = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::other(format!("invalid session store: {error}")))?;
        snapshot.validate(&self.workspace)?;
        Ok(Some(snapshot))
    }
    pub fn save(&self, snapshot: &SessionSnapshot) -> io::Result<()> {
        snapshot.validate(&self.workspace)?;
        // Reject corrupt/unsupported existing stores rather than overwriting them.
        self.load()?;
        let mut file = tempfile::NamedTempFile::new_in(&self.directory)?;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
        serde_json::to_writer(Limited::new(&mut file, MAX_STORE_BYTES - 1), snapshot)
            .map_err(io::Error::other)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(self.directory.join("state.json"))
            .map_err(|error| error.error)?;
        fs::File::open(&self.directory)?.sync_all()
    }
}
// File close releases flock even if the process exits unexpectedly. Do not
// unlink the lock inode: another process may already have opened it.
struct Limited<W> {
    writer: W,
    remaining: u64,
}
impl<W> Limited<W> {
    fn new(writer: W, remaining: u64) -> Self {
        Self { writer, remaining }
    }
}
impl<W: Write> Write for Limited<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::other(
                "session snapshot exceeds storage byte limit",
            ));
        }
        let written = self.writer.write(bytes)?;
        self.remaining -= written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
