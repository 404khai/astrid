//! Client selection and labels; conversations execute through the runtime API.
use astrid::{
    runtime::{RunOutcome, RunResult, Session},
    session_store::{MAX_SESSIONS, SessionSnapshot, SessionStore, StoredSession},
    workspace::Workspace,
};

struct Entry {
    session: Option<Session>,
    label: String,
    runs: usize,
    outcome: Option<RunOutcome>,
}
pub struct Sessions {
    entries: Vec<Entry>,
    active: usize,
    store: SessionStore,
    workspace: Workspace,
    dirty: bool,
}
impl Sessions {
    pub fn open(workspace: &Workspace, directory: &std::path::Path) -> std::io::Result<Self> {
        let store = SessionStore::open(directory, workspace)?;
        let snapshot = store.load()?;
        let mut sessions = Self {
            entries: Vec::new(),
            active: 0,
            store,
            workspace: workspace.clone(),
            dirty: false,
        };
        if let Some(snapshot) = snapshot {
            sessions.active = snapshot.active;
            sessions.entries = snapshot
                .entries
                .into_iter()
                .map(|entry| Entry {
                    label: session_name(&entry.session),
                    session: Some(entry.session),
                    runs: entry.runs,
                    outcome: entry.outcome,
                })
                .collect();
        } else {
            sessions.create(workspace).map_err(std::io::Error::other)?;
        }
        Ok(sessions)
    }
    pub fn persist(&mut self) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let entries =
            self.entries
                .iter()
                .map(|entry| {
                    let session = entry.session.as_ref().ok_or_else(|| {
                        std::io::Error::other("cannot save a session while running")
                    })?;
                    Ok(StoredSession {
                        session: session.clone(),
                        runs: entry.runs,
                        outcome: entry.outcome.clone(),
                    })
                })
                .collect::<std::io::Result<Vec<_>>>()?;
        self.store
            .save(&SessionSnapshot::new(&self.workspace, self.active, entries))?;
        self.dirty = false;
        Ok(())
    }
    pub fn create(&mut self, workspace: &Workspace) -> Result<(), String> {
        if self.entries.len() == MAX_SESSIONS {
            return Err("Session limit (32) reached; existing sessions remain available.".into());
        }
        self.entries.push(Entry {
            session: Some(Session::new(workspace)),
            label: "New conversation".into(),
            runs: 0,
            outcome: None,
        });
        self.active = self.entries.len() - 1;
        self.dirty = true;
        Ok(())
    }
    pub fn active_index(&self) -> usize {
        self.active
    }
    pub fn active_label(&self) -> String {
        let id = self.entries[self.active]
            .session
            .as_ref()
            .expect("idle session")
            .id
            .to_string();
        format!("session {}", &id[..8])
    }
    pub fn choices(&self) -> Vec<String> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let id = entry.session.as_ref().expect("idle session").id.to_string();
                let outcome = match &entry.outcome {
                    None => "new",
                    Some(RunOutcome::Completed) => "completed",
                    Some(RunOutcome::Cancelled) => "cancelled",
                    Some(RunOutcome::Failed { .. }) => "failed",
                    Some(RunOutcome::ModelCallLimitReached { .. }) => "call limit",
                };
                format!(
                    "{} {} · {} · {} runs · {outcome}",
                    if index == self.active { "*" } else { " " },
                    &id[..8],
                    entry.label,
                    entry.runs
                )
            })
            .collect()
    }
    pub fn select(&mut self, choice: &str) -> bool {
        if let Some(index) = self.choices().iter().position(|value| value == choice) {
            self.active = index;
            self.dirty = true;
            true
        } else {
            false
        }
    }
    pub fn take_active(&mut self) -> Session {
        self.entries[self.active]
            .session
            .take()
            .expect("one run per session")
    }
    pub fn restore(&mut self, session: Session) {
        self.entries[self.active].session = Some(session);
    }
    pub fn complete(&mut self, result: RunResult) {
        let entry = &mut self.entries[self.active];
        entry.label = session_name(&result.session);
        entry.runs += 1;
        entry.outcome = Some(result.outcome);
        entry.session = Some(result.session);
        self.dirty = true;
    }
}

fn session_name(session: &Session) -> String {
    match session.messages.first() {
        Some(astrid::model::Message::User(task)) => {
            let clean = crate::console::printable(task)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let mut name: String = clean.chars().take(80).collect();
            if clean.chars().count() > 80 {
                name.push('…');
            }
            if name.is_empty() {
                "New conversation".into()
            } else {
                name
            }
        }
        _ => "New conversation".into(),
    }
}
