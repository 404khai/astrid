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
    pub mode: astrid::permissions::PermissionMode,
    pub session_label: String,
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
        let mut session = Session::with_height(terminal::color(), 10)?;
        let terminal = &mut session.renderer.terminal;
        let mut editor = Editor::new(models, model);
        editor.choice_title = title.map(str::to_owned);
        if models.is_some() && title != Some("Switch model — type to filter") {
            editor.textarea.set_placeholder_text("Type to filter…");
        }
        let status = format!("{} · {} · {model}", self.mode, self.session_label);
        let result = loop {
            terminal.draw(|frame| {
                widgets::composer(frame, &mut editor, &status, &self.notice, terminal::color())
            })?;
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
