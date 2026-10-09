//! CLI presentation adapters. No provider or tool implementation dependencies.
use astrid::events::{EventKind, ExecutionEvent};
use std::io::{self, Write};
use unicode_width::UnicodeWidthStr;

mod append;
mod composer;
mod format;
mod identity;
mod inline;
mod input;
mod layout;
mod legacy;
#[cfg(test)]
mod pty_tests;
mod state;
mod terminal;
#[cfg(test)]
mod tests;
mod theme;
mod widgets;

pub use composer::Composer;
pub use format::printable;
use format::single_line;
pub use identity::Identity;
use input::{Action, ApprovalInput};
use state::Presentation;

pub fn interactive_available() -> bool {
    terminal::capable()
}
pub fn welcome(identity: &Identity) -> io::Result<()> {
    identity::append_header(
        &mut io::stderr().lock(),
        identity,
        terminal::size().map_or(80, |(w, _)| w),
        terminal::color(),
    )
}

enum Renderer {
    Append,
    Inline(Box<inline::Session>),
    Legacy(legacy::Screen),
}
pub struct Console {
    pub show_context: bool,
    identity: Identity,
    state: Presentation,
    renderer: Renderer,
    approval: ApprovalInput,
    color: bool,
    dirty: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum InputAction {
    None,
    Cancel,
    Approval(bool),
}
impl Console {
    pub fn new(identity: Identity, show_header: bool) -> io::Result<Self> {
        let capable = terminal::capable();
        let color = terminal::color() && terminal::size().is_some_and(|(w, h)| w >= 8 && h >= 8);
        let fixed = capable && std::env::var_os("ASTRID_FIXED_VIEWPORT").is_some();
        if capable && show_header && !fixed {
            welcome(&identity)?;
        }
        let renderer = if fixed {
            let (w, h) = terminal::size().unwrap_or((80, 24));
            Renderer::Legacy(legacy::Screen::new(w, h, color))
        } else if capable {
            Renderer::Inline(Box::new(inline::Session::new(color)?))
        } else {
            Renderer::Append
        };
        let mut state = Presentation::new(identity.model.clone());
        state.mode = identity.mode.clone();
        let settings = astrid::observability::Settings::load(
            &astrid::auth::default_directory()
                .map_err(io::Error::other)?
                .join("settings.json"),
        )?;
        state.expanded_tool_calls =
            settings.expanded_tool_calls == astrid::observability::Switch::On;
        let mut console = Self {
            show_context: false,
            state,
            identity,
            renderer,
            approval: ApprovalInput::default(),
            color,
            dirty: true,
        };
        if let Renderer::Legacy(screen) = &mut console.renderer {
            screen.draw(&mut io::stderr().lock(), &console.identity)?;
        }
        console.tick()?;
        Ok(console)
    }
    pub fn inline(&self) -> bool {
        matches!(self.renderer, Renderer::Inline(_))
    }
    fn output(&mut self) -> io::Result<()> {
        for piece in self.state.take_output() {
            match &mut self.renderer {
                Renderer::Append => append::write_piece(
                    &mut io::stdout().lock(),
                    &mut io::stderr().lock(),
                    &piece,
                    self.color,
                )?,
                Renderer::Inline(session) => session.renderer.push(&piece)?,
                Renderer::Legacy(screen) => {
                    screen.append(&mut io::stderr().lock(), &piece.text, piece.ink)?
                }
            }
        }
        self.dirty = true;
        Ok(())
    }
    pub fn render(&mut self, event: &ExecutionEvent) -> io::Result<()> {
        // The compatibility viewport must never hide the command being approved.
        if let EventKind::PermissionRequested { command, workspace } = &event.kind
            && let Renderer::Legacy(screen) = &self.renderer
        {
            let needed: usize =
                format!("? permission\n  cwd: {workspace}\n  command: {command}\n  authority\n")
                    .lines()
                    .map(|line| line.width().div_ceil(screen.content_width()).max(1))
                    .sum();
            if needed + 1 > screen.bottom - screen.top + 1 {
                self.restore()?;
            }
        }
        self.state.show_context = self.show_context;
        self.state.apply(event);
        self.output()?;
        if let EventKind::PermissionRequested { command, workspace } = &event.kind {
            self.flush()?;
            if !self.inline()
                && matches!(self.renderer, Renderer::Append)
                && let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty")
            {
                // Exact authority was already derived from the correlated tool event.
                write!(
                    tty,
                    "\nOperation in {}:\n{}\n{}\n› type yes to approve: ",
                    single_line(workspace),
                    printable(command),
                    self.state.approval_authority
                )?;
                tty.flush()?;
            }
        } else if matches!(
            event.kind,
            EventKind::PermissionGranted
                | EventKind::PermissionDenied
                | EventKind::PermissionCancelled
                | EventKind::PermissionFailed { .. }
        ) {
            self.approval.disarm();
            self.flush()?;
        } else if self.state.done {
            self.flush()?;
        }
        Ok(())
    }
    pub fn tick(&mut self) -> io::Result<()> {
        match &mut self.renderer {
            Renderer::Inline(session) if self.dirty => {
                session.renderer.draw(&self.state, &self.approval)?;
                self.dirty = false;
            }
            Renderer::Legacy(screen) => {
                if !self.state.waiting
                    && let Some((w, h)) = terminal::size()
                    && w >= 8
                    && h >= 8
                    && (w, h) != (screen.width, screen.height)
                {
                    screen.width = w;
                    screen.height = h;
                    screen.rewrap();
                    screen.draw(&mut io::stderr().lock(), &self.identity)?;
                }
                screen.footer(
                    &mut io::stderr().lock(),
                    &self.state.status(screen.content_width()),
                    self.state.waiting,
                    self.state.done,
                )?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn flush(&mut self) -> io::Result<()> {
        self.dirty = true;
        self.tick()
    }
    /// Discard typeahead before arming an already rendered permission request.
    /// Bounded polling keeps cancellation responsive even under an input flood.
    pub fn arm_permission(&mut self) -> io::Result<(bool, InputAction)> {
        if !self.inline() {
            return Ok((false, InputAction::None));
        }
        for _ in 0..64 {
            match composer::poll()? {
                None => {
                    self.approval.arm();
                    self.flush()?;
                    return Ok((true, InputAction::None));
                }
                Some(event) => {
                    // Drain all typeahead without editing an armed answer.
                    self.approval.disarm();
                    if self.approval.handle(event) == Action::Cancel {
                        return Ok((false, InputAction::Cancel));
                    }
                }
            }
        }
        Ok((false, InputAction::None))
    }
    pub fn poll_input(&mut self) -> io::Result<InputAction> {
        if !self.inline() {
            return Ok(InputAction::None);
        }
        let Some(event) = composer::poll()? else {
            return Ok(InputAction::None);
        };
        let action = self.approval.handle(event);
        self.dirty = true;
        Ok(match action {
            Action::Cancel => {
                self.approval.disarm();
                InputAction::Cancel
            }
            Action::Submit(answer) => {
                InputAction::Approval(answer.trim().eq_ignore_ascii_case("yes"))
            }
            _ => InputAction::None,
        })
    }
    pub fn finish(&mut self, models: usize, tools: usize) -> io::Result<()> {
        self.state.finish(models, tools);
        self.output()?;
        self.flush()?;
        self.restore()
    }
    fn restore(&mut self) -> io::Result<()> {
        match std::mem::replace(&mut self.renderer, Renderer::Append) {
            Renderer::Inline(mut session) => session.close(),
            Renderer::Legacy(screen) => {
                let mut out = io::stderr().lock();
                writeln!(out, "\x1b[r\x1b[?25h\x1b[{};1H", screen.height)?;
                out.flush()
            }
            Renderer::Append => Ok(()),
        }
    }
}
impl Drop for Console {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
