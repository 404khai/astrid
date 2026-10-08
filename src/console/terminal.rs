//! Terminal ownership belongs to this CLI, never to the runtime.
use crossterm::{
    cursor::Show,
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{
    io::{self, IsTerminal, Write},
    sync::{
        Once,
        atomic::{AtomicBool, Ordering},
    },
};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static HOOK: Once = Once::new();

pub(super) fn capable() -> bool {
    supports_inline(
        io::stdin().is_terminal(),
        io::stdout().is_terminal(),
        io::stderr().is_terminal(),
        std::env::var("TERM").ok().as_deref(),
        size(),
    )
}
pub(super) fn supports_inline(
    stdin: bool,
    stdout: bool,
    stderr: bool,
    term: Option<&str>,
    size: Option<(usize, usize)>,
) -> bool {
    stdin
        && stdout
        && stderr
        && term.is_some_and(|t| !t.is_empty() && t != "dumb")
        && size.is_some_and(|(w, h)| w >= 8 && h >= 8)
}
pub(super) fn color() -> bool {
    io::stdout().is_terminal()
        && io::stderr().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").is_ok_and(|t| t != "dumb")
}
pub(super) fn size() -> Option<(usize, usize)> {
    crossterm::terminal::size()
        .ok()
        .map(|(w, h)| (usize::from(w), usize::from(h)))
}

pub(super) struct TerminalGuard;
impl TerminalGuard {
    pub(super) fn enter() -> io::Result<Self> {
        HOOK.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                restore();
                previous(info);
            }));
        });
        enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        let guard = Self;
        execute!(io::stderr(), EnableBracketedPaste)?;
        Ok(guard)
    }
}
fn restore() {
    if ACTIVE.swap(false, Ordering::SeqCst) {
        // Attempt each cleanup even when another operation fails (e.g. broken pipe).
        let _ = disable_raw_mode();
        let _ = execute!(io::stderr(), DisableBracketedPaste, Show);
        let _ = io::stderr().flush();
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}
