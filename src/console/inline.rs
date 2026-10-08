//! Scrollback receives each completed display row once; only the tail is live.
use super::{
    input::ApprovalInput,
    layout,
    state::{Piece, Presentation},
    terminal::TerminalGuard,
    theme::Ink,
    widgets,
};
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::{Backend, CrosstermBackend},
    text::Line,
    widgets::Widget,
};
use std::{collections::VecDeque, io};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const PENDING_BYTES: usize = 16 * 1024;
const TAIL_BYTES: usize = 8 * 1024;

#[derive(Default)]
pub(super) struct Transcript {
    lines: VecDeque<(String, Ink)>,
    pub tail: String,
    ink: Ink,
    bytes: usize,
}
impl Transcript {
    fn push(&mut self, text: &str, ink: Ink) {
        for part in text.split_inclusive('\n') {
            if self.tail.is_empty() {
                self.ink = ink;
            }
            self.tail.push_str(part.strip_suffix('\n').unwrap_or(part));
            if part.ends_with('\n') {
                self.bytes += self.tail.len() + 1;
                self.lines
                    .push_back((std::mem::take(&mut self.tail), self.ink));
            }
        }
    }
    fn pressure(&self) -> bool {
        self.bytes >= PENDING_BYTES || self.lines.len() >= 256 || self.tail.len() >= TAIL_BYTES
    }
    #[cfg(test)]
    pub(super) fn pending_bytes(&self) -> usize {
        self.bytes + self.tail.len()
    }
}

// Byte ranges preserve the original graphemes when the live tail is reflowed.
fn ranges(text: &str, width: usize) -> Vec<std::ops::Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut cells = 0;
    for (index, g) in text.grapheme_indices(true) {
        let size = g.width();
        if cells + size > width && index > start {
            rows.push(start..index);
            start = index;
            cells = 0;
        }
        cells += size;
    }
    rows.push(start..text.len());
    rows
}
fn visible_line(text: &str, ink: Ink, color: bool, width: usize) -> Line<'static> {
    // A wide grapheme cannot fit a one-cell terminal after an extreme resize.
    let text: String = text
        .graphemes(true)
        .map(|g| if g.width() > width { "�" } else { g })
        .collect();
    Line::styled(text, ink.style(color))
}

pub(super) struct Inline<B: Backend> {
    pub terminal: Terminal<B>,
    pub transcript: Transcript,
    color: bool,
}
impl<B: Backend> Inline<B> {
    pub(super) fn new(terminal: Terminal<B>, color: bool) -> Self {
        Self {
            terminal,
            transcript: Transcript::default(),
            color,
        }
    }
    pub(super) fn push(&mut self, piece: &Piece) -> Result<(), B::Error> {
        let text = piece.text.replace('\t', "    ");
        // Process bounded chunks even when a single runtime event is large.
        let mut start = 0;
        for (index, _) in text.char_indices().skip(1) {
            if index - start >= 1024 {
                self.transcript.push(&text[start..index], piece.ink);
                start = index;
                if self.transcript.pressure() {
                    self.insert_pending(false)?;
                }
            }
        }
        self.transcript.push(&text[start..], piece.ink);
        if self.transcript.pressure() {
            self.insert_pending(false)?;
        }
        Ok(())
    }
    fn insert_lines(&mut self, rows: Vec<Line<'static>>) -> Result<(), B::Error> {
        let width = self.terminal.get_frame().area().width.max(1) as usize;
        // Bound temporary cell buffers as well as queued transcript bytes.
        let batch = (65536 / width).clamp(1, 128);
        for rows in rows.chunks(batch) {
            self.terminal.insert_before(rows.len() as u16, |buffer| {
                for (index, line) in rows.iter().enumerate() {
                    line.render(widgets::line_area(buffer.area, index as u16), buffer);
                }
            })?;
        }
        Ok(())
    }
    pub(super) fn insert_pending(&mut self, force: bool) -> Result<(), B::Error> {
        self.terminal.autoresize()?;
        let area = self.terminal.get_frame().area();
        let width = area.width.max(1) as usize;
        while let Some((text, ink)) = self.transcript.lines.pop_front() {
            self.transcript.bytes -= text.len() + 1;
            // Large logical lines are streamed in batches, not one large cell buffer.
            let row_ranges = ranges(&text, width);
            for row_ranges in row_ranges.chunks(128) {
                self.insert_lines(
                    row_ranges
                        .iter()
                        .map(|r| visible_line(&text[r.clone()], ink, self.color, width))
                        .collect(),
                )?;
            }
        }
        if !self.transcript.tail.is_empty() {
            let rows = ranges(&self.transcript.tail, width);
            let keep = if force {
                0
            } else {
                area.height.saturating_sub(2).max(1) as usize
            };
            let count = rows.len().saturating_sub(keep);
            if count > 0 {
                let end = rows[count - 1].end;
                let prefix: String = self.transcript.tail.drain(..end).collect();
                self.insert_lines(
                    rows[..count]
                        .iter()
                        .map(|r| {
                            visible_line(&prefix[r.clone()], self.transcript.ink, self.color, width)
                        })
                        .collect(),
                )?;
            } else if self.transcript.tail.len() >= TAIL_BYTES {
                // Pathological single graphemes can contain arbitrarily many marks.
                // Flush a character-boundary prefix to bound memory, retaining all bytes.
                let end = self
                    .transcript
                    .tail
                    .char_indices()
                    .find(|(i, _)| *i >= TAIL_BYTES / 2)
                    .map_or(self.transcript.tail.len(), |(i, _)| i);
                let prefix: String = self.transcript.tail.drain(..end).collect();
                self.insert_lines(vec![visible_line(
                    &prefix,
                    self.transcript.ink,
                    self.color,
                    width,
                )])?;
            }
        }
        Ok(())
    }
    pub(super) fn draw(
        &mut self,
        state: &Presentation,
        approval: &ApprovalInput,
    ) -> Result<(), B::Error> {
        self.insert_pending(false)?;
        let tail = &self.transcript.tail;
        let ink = self.transcript.ink;
        let color = self.color;
        self.terminal.draw(|frame| {
            let (area, _, _) = layout::execution(frame.area());
            for (row, range) in ranges(tail, area.width as usize)
                .into_iter()
                .take(area.height as usize)
                .enumerate()
            {
                frame.render_widget(
                    visible_line(&tail[range], ink, color, area.width as usize),
                    widgets::line_area(area, row as u16),
                );
            }
            widgets::status(
                frame,
                &state.status(frame.area().width as usize),
                &approval.text,
                state.waiting,
                state.done,
                color,
            );
        })?;
        Ok(())
    }
    pub(super) fn close(&mut self) -> Result<(), B::Error> {
        self.insert_pending(true)?;
        let origin = self.terminal.get_frame().area().as_position();
        self.terminal.clear()?;
        self.terminal.set_cursor_position(origin)?;
        self.terminal.show_cursor()?;
        self.terminal.backend_mut().flush()
    }
}

pub(super) type NativeInline = Inline<CrosstermBackend<io::Stderr>>;
pub(super) struct Session {
    pub renderer: NativeInline,
    _guard: TerminalGuard,
    closed: bool,
}
impl Session {
    pub(super) fn new(color: bool) -> io::Result<Self> {
        Self::with_height(color, 5)
    }
    pub(super) fn with_height(color: bool, height: u16) -> io::Result<Self> {
        let guard = TerminalGuard::enter()?;
        let terminal = native_terminal(height)?;
        Ok(Self {
            renderer: Inline::new(terminal, color),
            _guard: guard,
            closed: false,
        })
    }
    pub(super) fn close(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        self.renderer.close()
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
pub(super) fn native_terminal(height: u16) -> io::Result<Terminal<CrosstermBackend<io::Stderr>>> {
    Terminal::with_options(
        CrosstermBackend::new(io::stderr()),
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )
}
