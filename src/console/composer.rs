use super::{
    inline::Session,
    input::{Action, Editor},
    terminal::{self},
    widgets,
};
use crossterm::event::{self, Event};
use std::{io, time::Duration};

#[derive(Default)]
pub struct Composer {
    pub notice: String,
    pub header: Option<super::Identity>,
    pub mode: astrid::permissions::PermissionMode,
    pub session_label: String,
    pub workspace_root: Option<std::path::PathBuf>,
}
impl Composer {
    pub fn compose(&mut self, model: &str) -> io::Result<String> {
        self.run(model, None, None).map(|s| s.unwrap_or_default())
    }
    pub fn select_model(&mut self, models: &[String], current: &str) -> io::Result<Option<String>> {
        self.run(current, Some(models), Some("Switch model — type to filter"))
    }
    pub fn choose(
        &mut self,
        title: &str,
        options: &[String],
        current: &str,
    ) -> io::Result<Option<String>> {
        self.run(current, Some(options), Some(title))
    }
    fn run(
        &mut self,
        model: &str,
        models: Option<&[String]>,
        title: Option<&str>,
    ) -> io::Result<Option<String>> {
        if !terminal::capable() {
            return Err(io::Error::other(
                "interactive startup requires usable terminal input and output; use astrid run \"task\" --model <model>",
            ));
        }
        let size = terminal::size().unwrap_or((80, 24));
        let header_height = self.header.as_ref().map_or(0, |id| {
            super::identity::header_lines(id, size.0, terminal::color()).len() as u16
        });
        let mut session =
            Session::with_height(terminal::color(), (header_height + 10).min(size.1 as u16))?;
        let terminal = &mut session.renderer.terminal;
        let mut editor = Editor::new(models, model);
        editor.choice_title = title.map(str::to_owned);
        if models.is_some() && title != Some("Switch model — type to filter") {
            editor.textarea.set_placeholder_text("Type to filter…");
        }
        // Filename traversal runs off the terminal thread. At most one pending
        // request and one result are queued; stale results never replace newer queries.
        let (requests, receiver) = std::sync::mpsc::sync_channel::<String>(1);
        let (results, completed) = std::sync::mpsc::sync_channel(1);
        let root = match &self.workspace_root {
            Some(root) => root.clone(),
            None => std::env::current_dir()?,
        };
        std::thread::spawn(move || {
            while let Ok(query) = receiver.recv() {
                let pattern = if query.is_empty() {
                    "*".into()
                } else {
                    format!("{{**/*{query}*,**/*{query}*/**}}")
                };
                let result =
                    astrid::file_lookup::find_files(&root, std::path::Path::new("."), &pattern, 10);
                if results.send((query, result)).is_err() {
                    break;
                }
            }
        });
        let mut requested: Option<String> = None;
        let status = format!("{} · {} · {model}", self.mode, self.session_label);
        let result = loop {
            let query = editor.mention().map(|(_, _, query)| query);
            if query != requested {
                editor.file_options.clear();
                editor.lookup_error = None;
                match &query {
                    Some(query) if requests.try_send(query.clone()).is_ok() => {
                        requested = Some(query.clone())
                    }
                    None => requested = None,
                    _ => {}
                }
            }
            while let Ok((searched, result)) = completed.try_recv() {
                if query.as_ref() == Some(&searched) {
                    match result {
                        Ok(paths) => {
                            editor.file_options = paths
                                .into_iter()
                                .map(|p| (p.to_string_lossy().into_owned(), String::new()))
                                .collect()
                        }
                        Err(error) => editor.lookup_error = Some(format!("File lookup: {error}")),
                    }
                }
            }
            terminal.draw(|frame| {
                let mut area = frame.area();
                if let Some(identity) = &self.header {
                    let lines = super::identity::header_lines(
                        identity,
                        area.width as usize,
                        terminal::color(),
                    );
                    let height = (lines.len() as u16).min(area.height.saturating_sub(5));
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(lines),
                        ratatui::layout::Rect::new(area.x, area.y, area.width, height),
                    );
                    area.y += height;
                    area.height -= height;
                }
                widgets::composer_in_area(
                    frame,
                    area,
                    &mut editor,
                    &status,
                    &self.notice,
                    terminal::color(),
                )
            })?;
            if !event::poll(Duration::from_millis(50))? {
                continue;
            }
            let event = match event::read() {
                Ok(e) => e,
                Err(e) => break Err(e),
            };
            match editor.handle(event) {
                Action::Submit(text) => break Ok(Some(text)),
                Action::Dismiss => break Ok(None),
                Action::Cancel => {
                    break Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
                }
                _ => {}
            }
        };
        session.close()?;
        result
    }
}

pub(super) fn poll() -> io::Result<Option<Event>> {
    if event::poll(Duration::ZERO)? {
        event::read().map(Some)
    } else {
        Ok(None)
    }
}
