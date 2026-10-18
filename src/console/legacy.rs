use super::{
    format::{printable, truncate},
    identity::Identity,
    theme::Ink,
};
use std::{
    collections::VecDeque,
    io::{self, Write},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// A small terminal-native viewport, without raw mode or an alternate screen.
/// Only the stream scrolls; header and footer are outside the scroll margins.
/// Canonical terminal input and Ctrl-C retain their existing OS behavior.
pub(super) struct Screen {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) top: usize,
    pub(super) bottom: usize,
    pub(super) row: usize,
    column: usize,
    pub(super) history: VecDeque<(String, Ink)>,
    pub(super) color: bool,
}
impl Screen {
    pub(super) fn new(width: usize, height: usize, color: bool) -> Self {
        Self {
            width,
            height,
            top: 1,
            bottom: 1,
            row: 1,
            column: 0,
            history: VecDeque::from([(String::new(), Ink::Normal)]),
            color,
        }
    }
    pub(super) fn content_width(&self) -> usize {
        self.width.saturating_sub(2).max(1)
    }
    pub(super) fn draw(&mut self, out: &mut impl Write, identity: &Identity) -> io::Result<()> {
        // Execution keeps a compact identity so the conversation gets most rows.
        let budget = self.height.saturating_sub(5).clamp(1, 6);
        let header =
            super::identity::header_lines(identity, self.content_width(), budget, self.color);
        self.top = (header.len() + 1).min(self.height.saturating_sub(4).max(1));
        self.bottom = self.height.saturating_sub(3).max(self.top);
        write!(out, "\x1b[r\x1b[2J\x1b[H\x1b[?25l")?;
        for (i, line) in header.iter().take(self.top - 1).enumerate() {
            write!(out, "\x1b[{};2H", i + 1)?;
            super::identity::write_line(out, line, self.color)?;
        }
        write!(out, "\x1b[{};{}r", self.top, self.bottom)?;
        let visible = self.bottom - self.top + 1;
        let start = self.history.len().saturating_sub(visible);
        self.row = self.top;
        for (index, (line, ink)) in self.history.iter().skip(start).enumerate() {
            self.row = self.top + index;
            write!(
                out,
                "\x1b[{};2H{}",
                self.row,
                ink.paint(&truncate(line, self.content_width()), self.color)
            )?;
        }
        self.column = self
            .history
            .back()
            .map(|(line, _)| line.width().min(self.content_width()))
            .unwrap_or(0);
        Ok(())
    }
    pub(super) fn rewrap(&mut self) {
        let mut lines = VecDeque::new();
        for (text, ink) in &self.history {
            let mut line = String::new();
            let mut used = 0;
            for c in text.chars() {
                let cells = c.width().unwrap_or(0);
                if used + cells > self.content_width() {
                    lines.push_back((std::mem::take(&mut line), *ink));
                    used = 0;
                }
                line.push(c);
                used += cells;
            }
            lines.push_back((line, *ink));
        }
        while lines.len() > 2000 {
            lines.pop_front();
        }
        self.history = lines;
    }
    fn newline(&mut self, out: &mut impl Write, ink: Ink) -> io::Result<()> {
        write!(out, "\r\n\x1b[2G")?;
        self.row = (self.row + 1).min(self.bottom);
        self.column = 0;
        self.history.push_back((String::new(), ink));
        if self.history.len() > 2000 {
            self.history.pop_front();
        }
        Ok(())
    }
    pub(super) fn append(&mut self, out: &mut impl Write, text: &str, ink: Ink) -> io::Result<()> {
        let mut bytes = Vec::new();
        write!(bytes, "\x1b[{};{}H", self.row, self.column + 2)?;
        for c in printable(text).replace('\t', "    ").chars() {
            if c == '\n' {
                self.newline(&mut bytes, ink)?;
                continue;
            }
            let cells = c.width().unwrap_or(0);
            if self.column + cells > self.content_width() {
                self.newline(&mut bytes, ink)?;
            }
            if cells > self.content_width() {
                continue;
            }
            write!(bytes, "{}", ink.paint(&c.to_string(), self.color))?;
            if let Some((line, style)) = self.history.back_mut() {
                line.push(c);
                *style = ink;
            }
            self.column += cells;
        }
        out.write_all(&bytes)
    }
    pub(super) fn footer(
        &self,
        out: &mut impl Write,
        status: &str,
        waiting: bool,
        done: bool,
    ) -> io::Result<()> {
        let status = truncate(status, self.content_width());
        let prompt = if waiting && self.width < 12 {
            "› "
        } else if waiting && self.width < 30 {
            "› yes: "
        } else if waiting {
            "› type yes to approve: "
        } else if done {
            "› run ended"
        } else {
            "› executing"
        };
        let help = if done {
            "astrid --help"
        } else {
            "ctrl+c cancel"
        };
        for (row, text, ink) in [
            (self.height - 2, status.as_str(), Ink::Dim),
            (
                self.height - 1,
                prompt,
                if waiting {
                    super::theme::title_ink(status.starts_with("unbound"))
                } else {
                    Ink::Dim
                },
            ),
            (self.height, help, Ink::Dim),
        ] {
            write!(
                out,
                "\x1b[{row};1H\x1b[2K\x1b[2G{}",
                if let Some(rest) = text.strip_prefix("unbound") {
                    format!(
                        "{}{}",
                        Ink::Unbound.paint(&truncate("unbound", self.content_width()), self.color),
                        ink.paint(
                            &truncate(rest, self.content_width().saturating_sub(7)),
                            self.color
                        )
                    )
                } else {
                    ink.paint(&truncate(text, self.content_width()), self.color)
                }
            )?;
        }
        if waiting {
            write!(
                out,
                "\x1b[{};{}H\x1b[?25h",
                self.height - 1,
                2 + prompt.width().min(self.content_width())
            )?;
        } else {
            write!(out, "\x1b[?25l")?;
        }
        out.flush()
    }
}
