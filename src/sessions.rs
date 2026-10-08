//! Client selection and labels; conversations execute through the runtime API.
use astrid::{
    runtime::{RunOutcome, RunResult, Session},
    workspace::Workspace,
};

const MAX_SESSIONS: usize = 32;
struct Entry {
    session: Option<Session>,
    label: String,
    runs: usize,
    outcome: Option<RunOutcome>,
}
pub struct Sessions {
    entries: Vec<Entry>,
    active: usize,
}
impl Sessions {
    pub fn new(workspace: &Workspace) -> Self {
        let mut sessions = Self {
            entries: Vec::new(),
            active: 0,
        };
        sessions.create(workspace).expect("initial session fits");
        sessions
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
        if entry.runs == 0
            && let Some(astrid::model::Message::User(task)) = result.session.messages.first()
        {
            entry.label = crate::console::printable(task)
                .replace(['\n', '\r', '\t'], " ")
                .chars()
                .take(80)
                .collect();
        }
        entry.runs += 1;
        entry.outcome = Some(result.outcome);
        entry.session = Some(result.session);
    }
}
