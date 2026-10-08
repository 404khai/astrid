//! Keys become CLI actions before being passed to the editor widget.
use super::format::printable;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui_textarea::{CursorMove, TextArea, WrapMode};
use unicode_segmentation::UnicodeSegmentation;

const COMMANDS: [(&str, &str); 6] = [
    ("/model", "Switch the active model"),
    ("/help", "Show available commands"),
    ("/quit", "Exit Astrid"),
    ("/mode", "Switch permission mode"),
    ("/sessions", "Switch conversation"),
    ("/new", "Start a fresh session"),
];
const MAX_INPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Action {
    Continue,
    Submit(String),
    Dismiss,
    Cancel,
    Resize,
}

pub(super) struct Editor {
    pub textarea: TextArea<'static>,
    pub selected: usize,
    models: Option<Vec<String>>,
    pub notice: Option<&'static str>,
    pub choice_title: Option<String>,
}
impl Editor {
    pub(super) fn new(models: Option<&[String]>, current: &str) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text(if models.is_some() {
            "Search models…"
        } else {
            "Message Astrid… (/ for commands)"
        });
        textarea.set_wrap_mode(WrapMode::Glyph);
        Self {
            textarea,
            notice: None,
            choice_title: None,
            models: models.map(<[String]>::to_vec),
            selected: models
                .and_then(|m| m.iter().position(|v| v == current))
                .unwrap_or(0),
        }
    }
    pub(super) fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }
    pub(super) fn menu(&self) -> bool {
        self.models.is_some() || self.text().starts_with('/')
    }
    pub(super) fn model_menu(&self) -> bool {
        self.models.is_some()
    }
    pub(super) fn options(&self) -> Vec<(String, String)> {
        let text = self.text();
        if let Some(models) = &self.models {
            models
                .iter()
                .filter(|m| m.to_lowercase().contains(&text.to_lowercase()))
                .map(|m| (m.clone(), String::new()))
                .collect()
        } else if self.menu() {
            COMMANDS
                .iter()
                .filter(|(c, _)| c.starts_with(&text))
                .map(|(c, d)| (c.to_string(), d.to_string()))
                .collect()
        } else {
            Vec::new()
        }
    }
    fn replace(&mut self, text: &str) {
        self.textarea.select_all();
        self.textarea.insert_str(text);
        self.selected = 0;
    }
    pub(super) fn handle(&mut self, event: Event) -> Action {
        match event {
            Event::Resize(_, _) => return Action::Resize,
            Event::Paste(text) => {
                // Paste is data, never an Enter/command action. Normalize CRLF once.
                let text = printable(&text.replace("\r\n", "\n").replace('\r', "\n"));
                if self.text().len() + text.len() <= MAX_INPUT_BYTES {
                    self.textarea.insert_str(text);
                    self.notice = None;
                } else {
                    self.notice = Some("Input limit: 64 KiB; paste was not inserted.");
                }
                self.selected = 0;
                return Action::Continue;
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => return self.key(key),
            _ => {}
        }
        Action::Continue
    }
    fn key(&mut self, key: KeyEvent) -> Action {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let options = self.options();
        self.selected = self.selected.min(options.len().saturating_sub(1));
        match key.code {
            KeyCode::Char('c' | 'C') if ctrl => Action::Cancel,
            KeyCode::Char('d') if ctrl && self.text().is_empty() => {
                if self.model_menu() {
                    Action::Dismiss
                } else {
                    Action::Submit("/quit".into())
                }
            }
            KeyCode::Char('h' | 'd' | 'b' | 'f') if ctrl => {
                let code = match key.code {
                    KeyCode::Char('h') => KeyCode::Backspace,
                    KeyCode::Char('d') => KeyCode::Delete,
                    KeyCode::Char('b') => KeyCode::Left,
                    _ => KeyCode::Right,
                };
                self.grapheme_key(code);
                Action::Continue
            }
            KeyCode::Char('m' | 'M') if ctrl => {
                self.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            }
            KeyCode::Char('j' | 'J') if ctrl => {
                if self.text().len() < MAX_INPUT_BYTES {
                    self.textarea.insert_newline();
                } else {
                    self.notice = Some("Input limit: 64 KiB.");
                }
                Action::Continue
            }
            KeyCode::Enter => {
                if !self.model_menu() && self.text() == "/exit" {
                    return Action::Submit("/exit".into());
                }
                if self.menu() {
                    if let Some((value, _)) = options.get(self.selected) {
                        if !self.model_menu() && value == "/help" {
                            self.replace("/");
                            Action::Continue
                        } else {
                            Action::Submit(value.clone())
                        }
                    } else if !self.model_menu() {
                        Action::Submit(self.text())
                    } else {
                        Action::Continue
                    }
                } else {
                    Action::Submit(self.text())
                }
            }
            KeyCode::Esc => {
                if self.model_menu() {
                    Action::Dismiss
                } else {
                    self.replace("");
                    Action::Continue
                }
            }
            KeyCode::Tab if self.menu() => {
                if !self.model_menu()
                    && let Some((value, _)) = options.get(self.selected)
                {
                    self.replace(value);
                }
                Action::Continue
            }
            KeyCode::Up | KeyCode::Down if self.menu() && !options.is_empty() => {
                self.selected = if key.code == KeyCode::Up {
                    (self.selected + options.len() - 1) % options.len()
                } else {
                    (self.selected + 1) % options.len()
                };
                Action::Continue
            }
            KeyCode::Backspace | KeyCode::Delete | KeyCode::Left | KeyCode::Right
                if key.modifiers.is_empty() =>
            {
                self.grapheme_key(key.code);
                self.selected = 0;
                Action::Continue
            }
            _ => {
                let growth = match key.code {
                    KeyCode::Char(c)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        c.len_utf8()
                    }
                    KeyCode::Tab => usize::from(self.textarea.tab_length()),
                    KeyCode::Char('y') if ctrl => self.textarea.yank_text().len(),
                    _ => 0,
                };
                if self.text().len().saturating_add(growth) <= MAX_INPUT_BYTES {
                    self.textarea.input(key);
                    self.snap_cursor();
                    self.notice = None;
                } else {
                    self.notice = Some("Input limit: 64 KiB.");
                }
                self.selected = 0;
                Action::Continue
            }
        }
    }
    fn snap_cursor(&mut self) {
        let cursor = self.textarea.cursor();
        let line = &self.textarea.lines()[cursor.0];
        let byte = line
            .char_indices()
            .nth(cursor.1)
            .map_or(line.len(), |(i, _)| i);
        if let Some((start, g)) = line
            .grapheme_indices(true)
            .find(|(i, g)| *i < byte && *i + g.len() > byte)
        {
            let column = line[..start + g.len()].chars().count();
            self.textarea
                .move_cursor(CursorMove::Jump(cursor.0 as u16, column as u16));
        }
    }
    // Textarea's native positions count characters. Keep plain navigation/deletion
    // on grapheme boundaries so combining marks and emoji aren't split by Backspace.
    fn grapheme_key(&mut self, key: KeyCode) {
        if self.textarea.is_selecting() && matches!(key, KeyCode::Backspace | KeyCode::Delete) {
            self.textarea.delete_char();
            return;
        }
        self.snap_cursor();
        let cursor = self.textarea.cursor();
        let line = &self.textarea.lines()[cursor.0];
        let byte = line
            .char_indices()
            .nth(cursor.1)
            .map_or(line.len(), |(i, _)| i);
        let backwards = matches!(key, KeyCode::Backspace | KeyCode::Left);
        let count = if backwards {
            line[..byte]
                .graphemes(true)
                .next_back()
                .map_or(1, |g| g.chars().count())
        } else {
            line[byte..]
                .graphemes(true)
                .next()
                .map_or(1, |g| g.chars().count())
        };
        for _ in 0..count {
            match key {
                KeyCode::Backspace => {
                    self.textarea.delete_char();
                }
                KeyCode::Delete => {
                    self.textarea.delete_next_char();
                }
                KeyCode::Left => self.textarea.move_cursor(CursorMove::Back),
                _ => self.textarea.move_cursor(CursorMove::Forward),
            }
        }
    }
}

#[derive(Default)]
pub(super) struct ApprovalInput {
    pub text: String,
    pub armed: bool,
}
impl ApprovalInput {
    pub(super) fn arm(&mut self) {
        self.text.clear();
        self.armed = true;
    }
    pub(super) fn disarm(&mut self) {
        self.text.clear();
        self.armed = false;
    }
    pub(super) fn handle(&mut self, event: Event) -> Action {
        match event {
            Event::Resize(_, _) => Action::Resize,
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c' | 'C'))
                {
                    return Action::Cancel;
                }
                if !self.armed {
                    return Action::Continue;
                }
                match key.code {
                    KeyCode::Enter => {
                        self.armed = false;
                        Action::Submit(std::mem::take(&mut self.text))
                    }
                    KeyCode::Esc => {
                        self.disarm();
                        Action::Submit(String::new())
                    }
                    KeyCode::Backspace => {
                        self.text.pop();
                        Action::Continue
                    }
                    KeyCode::Char('d')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && self.text.is_empty() =>
                    {
                        self.disarm();
                        Action::Submit(String::new())
                    }
                    KeyCode::Char(c)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                            && self.text.len() < 256 =>
                    {
                        self.text.push(c);
                        Action::Continue
                    }
                    _ => Action::Continue,
                }
            }
            // Never allow pasted data to grant account-level execution authority.
            Event::Paste(_) => Action::Continue,
            _ => Action::Continue,
        }
    }
}
