//! CLI presentation only. Runtime events remain the source of execution semantics.
use astrid::{
    events::{EventKind, ExecutionEvent, ToolCallId},
    model::{ToolCall, ToolOutcome},
    tools,
    workspace::Workspace,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    io::{self, IsTerminal, Write},
    os::fd::AsRawFd,
    path::Path,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::logo::LOGO;

pub fn printable(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect()
}
fn single_line(value: &str) -> String {
    printable(value).replace(['\n', '\t'], " ")
}
fn truncate(value: &str, width: usize) -> String {
    let value = single_line(value);
    if value.width() <= width {
        return value;
    }
    if width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut used = 0;
    for c in value.chars() {
        let cells = c.width().unwrap_or(0);
        if used + cells > width - 1 {
            break;
        }
        result.push(c);
        used += cells;
    }
    result.push('…');
    result
}
fn short_path(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME")
        && let Ok(relative) = path.strip_prefix(home)
    {
        return format!("~/{}", relative.display());
    }
    path.display().to_string()
}

#[derive(Clone, Copy, Default)]
enum Ink {
    #[default]
    Normal,
    Dim,
    Accent,
}
impl Ink {
    fn paint(self, text: &str, color: bool) -> String {
        if !color {
            return text.to_owned();
        }
        let code = match self {
            Self::Normal => "0",
            Self::Dim => "2",
            Self::Accent => "38;2;0;247;213",
        };
        format!("\x1b[{code}m{text}\x1b[0m")
    }
}
struct Identity {
    model: String,
    cwd: String,
    instructions: Option<String>,
    tools: Vec<String>,
}
impl Identity {
    fn rows(&self, width: usize) -> Vec<String> {
        let labels = width.saturating_sub(4).min(14);
        let row = |label: &str, value: &str| {
            let label = truncate(label, labels.saturating_sub(2));
            format!(
                "{label:labels$}{}",
                truncate(value, width.saturating_sub(labels))
            )
        };
        let mut rows = vec![
            row("model", &self.model),
            row("provider", "OpenAI / ChatGPT subscription"),
            row("cwd", &self.cwd),
        ];
        if let Some(instructions) = &self.instructions {
            rows.push(row("instructions", instructions));
        }
        rows.push(row(
            "tools",
            &tool_list(&self.tools, width.saturating_sub(labels)),
        ));
        rows
    }
}
fn tool_list(tools: &[String], width: usize) -> String {
    for shown in (0..=tools.len().min(5)).rev() {
        let mut value = tools[..shown].join(", ");
        let remaining = tools.len() - shown;
        if remaining > 0 {
            if shown > 0 {
                value.push_str("  ");
            }
            value.push_str(&format!("+{remaining} more"));
        }
        if value.width() <= width {
            return value;
        }
    }
    truncate(&format!("{} tools", tools.len()), width)
}

/// A small terminal-native viewport, without raw mode or an alternate screen.
/// Only the stream scrolls; header and footer are outside the scroll margins.
/// Canonical terminal input and Ctrl-C retain their existing OS behavior.
struct Screen {
    width: usize,
    height: usize,
    top: usize,
    bottom: usize,
    row: usize,
    column: usize,
    history: VecDeque<(String, Ink)>,
    color: bool,
}
impl Screen {
    fn new(width: usize, height: usize, color: bool) -> Self {
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
    fn content_width(&self) -> usize {
        self.width.saturating_sub(2).max(1)
    }
    fn header(&self, identity: &Identity) -> Vec<(String, Ink)> {
        let width = self.content_width();
        let brand = format!("astrid  {}", env!("CARGO_PKG_VERSION"));
        let metadata = identity.rows(width);
        let mut rows = vec![(String::new(), Ink::Normal)];
        if self.width >= 78 && self.height >= 20 {
            // At common laptop heights place identity alongside the unmodified logo.
            let mut right = vec![brand, String::new()];
            right.extend(identity.rows(width.saturating_sub(34)));
            for (index, line) in LOGO.lines().enumerate() {
                rows.push((
                    format!(
                        "{line:30}    {}",
                        right.get(index).map(String::as_str).unwrap_or("")
                    ),
                    if index == 0 { Ink::Accent } else { Ink::Normal },
                ));
            }
        } else if self.width >= 34 && self.height >= 30 {
            rows.extend(LOGO.lines().map(|line| (line.to_owned(), Ink::Accent)));
            rows.push((String::new(), Ink::Normal));
            rows.push((brand, Ink::Accent));
            rows.push((String::new(), Ink::Normal));
            rows.extend(metadata.into_iter().map(|line| (line, Ink::Dim)));
        } else {
            // Preserve useful output space in small terminals.
            rows.push((brand, Ink::Accent));
            let budget = self.height.saturating_sub(9);
            rows.extend(
                metadata
                    .into_iter()
                    .take(budget)
                    .map(|line| (line, Ink::Dim)),
            );
        }
        rows.push((String::new(), Ink::Normal));
        rows
    }
    fn draw(&mut self, out: &mut impl Write, identity: &Identity) -> io::Result<()> {
        let header = self.header(identity);
        self.top = (header.len() + 1).min(self.height.saturating_sub(4).max(1));
        self.bottom = self.height.saturating_sub(3).max(self.top);
        write!(out, "\x1b[r\x1b[2J\x1b[H\x1b[?25l")?;
        for (i, (text, ink)) in header.iter().take(self.top - 1).enumerate() {
            write!(out, "\x1b[{};2H", i + 1)?;
            let side = self.width >= 78 && self.height >= 20;
            let stacked = self.width >= 34 && self.height >= 30;
            if (side || stacked) && (1..=10).contains(&i) {
                let logo = LOGO.lines().nth(i - 1).unwrap_or("");
                write!(out, "{}", paint_logo(logo, i - 1, self.color))?;
                if side {
                    let tail: String = text.chars().skip(30).collect();
                    write!(out, "{}", " ".repeat(30 - logo.width()))?;
                    if i == 1 {
                        write!(out, "{}", Ink::Accent.paint(&tail, self.color))?;
                    } else {
                        let boundary = 4 + self.content_width().saturating_sub(38).min(14);
                        let label: String = tail.chars().take(boundary).collect();
                        let value: String = tail.chars().skip(boundary).collect();
                        write!(
                            out,
                            "{}{}",
                            Ink::Dim.paint(&label, self.color),
                            Ink::Normal.paint(&value, self.color)
                        )?;
                    }
                }
            } else if text.contains("  ") && !text.starts_with("astrid") {
                let boundary = self.content_width().saturating_sub(4).min(14);
                let label: String = text.chars().take(boundary).collect();
                let value: String = text.chars().skip(boundary).collect();
                write!(
                    out,
                    "{}{}",
                    Ink::Dim.paint(&label, self.color),
                    Ink::Normal.paint(&value, self.color)
                )?;
            } else {
                write!(
                    out,
                    "{}",
                    ink.paint(&truncate(text, self.content_width()), self.color)
                )?;
            }
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
    fn rewrap(&mut self) {
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
    fn append(&mut self, out: &mut impl Write, text: &str, ink: Ink) -> io::Result<()> {
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
    fn footer(
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
                if waiting { Ink::Accent } else { Ink::Dim },
            ),
            (self.height, help, Ink::Dim),
        ] {
            write!(
                out,
                "\x1b[{row};1H\x1b[2K\x1b[2G{}",
                ink.paint(&truncate(text, self.content_width()), self.color)
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
fn paint_logo(line: &str, row: usize, color: bool) -> String {
    if !color {
        return line.to_owned();
    }
    let mut result = String::new();
    for (column, pixel) in line.chars().enumerate() {
        if pixel == ' ' {
            result.push(pixel);
            continue;
        }
        let code = if (3..=5).contains(&row) && matches!(column, 10 | 11 | 16 | 17) {
            "38;2;0;247;213"
        } else {
            "38;2;68;89;249"
        };
        result.push_str(&format!("\x1b[{code}m{pixel}\x1b[0m"));
    }
    result
}

fn terminal_size() -> Option<(usize, usize)> {
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::ioctl(io::stderr().as_raw_fd(), libc::TIOCGWINSZ, &mut size) };
    (result == 0 && size.ws_col >= 8 && size.ws_row >= 8)
        .then_some((usize::from(size.ws_col), usize::from(size.ws_row)))
}

pub struct Console {
    identity: Identity,
    screen: Option<Screen>,
    calls: BTreeMap<ToolCallId, String>,
    run: String,
    turn: usize,
    model_calls: usize,
    state: &'static str,
    waiting: bool,
    done: bool,
    text_open: bool,
}
impl Console {
    pub fn new(model: &str, workspace: &Workspace) -> io::Result<Self> {
        let terminal = io::stdout().is_terminal() && io::stderr().is_terminal();
        let capable = terminal && std::env::var("TERM").is_ok_and(|term| term != "dumb");
        let identity = Identity {
            model: single_line(model),
            cwd: short_path(workspace.root()),
            // Presentation inspects availability; runtime still loads/validates instructions.
            instructions: workspace
                .instructions()
                .ok()
                .flatten()
                .map(|_| "AGENTS.md".into()),
            tools: tools::definitions()
                .iter()
                .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
                .collect(),
        };
        let mut console = Self {
            identity,
            screen: if capable {
                terminal_size()
                    .map(|(w, h)| Screen::new(w, h, std::env::var_os("NO_COLOR").is_none()))
            } else {
                None
            },
            calls: BTreeMap::new(),
            run: String::new(),
            turn: 0,
            model_calls: 0,
            state: "starting",
            waiting: false,
            done: false,
            text_open: false,
        };
        if console.screen.is_some() {
            console.redraw()?;
        } else if terminal {
            let mut out = io::stderr().lock();
            writeln!(out, "\n{}\n\nastrid  {}", LOGO, env!("CARGO_PKG_VERSION"))?;
            for row in console.identity.rows(78) {
                writeln!(out, " {row}")?;
            }
            writeln!(out)?;
        }
        Ok(console)
    }
    fn status(&self) -> String {
        if self.run.is_empty() {
            return self.state.into();
        }
        let width = self
            .screen
            .as_ref()
            .map_or(usize::MAX, Screen::content_width);
        if width < 30 {
            format!("RUN {} T{} {}", self.run, self.turn, self.state)
        } else if width < 76 {
            format!(
                "RUN {}   TURN {}   {}   MODEL {}",
                self.run, self.turn, self.state, self.identity.model
            )
        } else {
            format!(
                "RUN {}   TURN {}   MODEL {}   CALLS {}   {}",
                self.run, self.turn, self.identity.model, self.model_calls, self.state
            )
        }
    }
    fn redraw(&mut self) -> io::Result<()> {
        let status = self.status();
        if let Some(screen) = &mut self.screen {
            let mut out = io::stderr().lock();
            screen.draw(&mut out, &self.identity)?;
            screen.footer(&mut out, &status, self.waiting, self.done)?;
        }
        Ok(())
    }
    pub fn resize(&mut self) -> io::Result<()> {
        // Canonical input is echoed by the OS. Do not erase an answer being typed.
        if self.waiting {
            return Ok(());
        }
        if let Some((width, height)) = terminal_size()
            && let Some(screen) = &mut self.screen
            && (width, height) != (screen.width, screen.height)
        {
            screen.width = width;
            screen.height = height;
            screen.rewrap();
            self.redraw()?;
        }
        Ok(())
    }
    fn emit(&mut self, text: &str, ink: Ink, model: bool) -> io::Result<()> {
        if let Some(screen) = &mut self.screen {
            screen.append(&mut io::stderr().lock(), text, ink)
        } else if model {
            let mut out = io::stdout().lock();
            write!(out, "{}", printable(text))?;
            out.flush()
        } else {
            write!(io::stderr().lock(), "{}", printable(text))
        }
    }
    fn line(&mut self, text: &str, ink: Ink) -> io::Result<()> {
        if self.text_open {
            self.emit("\n", Ink::Normal, true)?;
            self.text_open = false;
        }
        self.emit(&format!("{}\n", single_line(text)), ink, false)
    }
    pub fn render(&mut self, event: &ExecutionEvent) -> io::Result<()> {
        match &event.kind {
            EventKind::RunStarted { task, .. } => {
                self.run = event.run_id.to_string()[..5].into();
                self.line(&format!("› {}", single_line(task)), Ink::Dim)?;
                self.line("", Ink::Normal)?;
                self.state = "running";
            }
            EventKind::TurnStarted { number } => self.turn = *number,
            EventKind::ModelCallStarted { number } => {
                self.model_calls = *number;
                self.state = "model";
            }
            EventKind::ModelTextDelta { text } => {
                self.emit(text, Ink::Normal, true)?;
                self.text_open = true;
            }
            EventKind::ModelCallCompleted { .. } => {
                if self.text_open {
                    self.emit("\n\n", Ink::Normal, true)?;
                    self.text_open = false;
                }
            }
            EventKind::ModelCallFailed { message } => self.line(
                &format!("× model        response interrupted: {message}"),
                Ink::Normal,
            )?,
            EventKind::ModelCallCancelled => {
                self.line("! model        response interrupted", Ink::Normal)?
            }
            EventKind::ToolCallRequested { call } => {
                if let Some(id) = event.tool_call_id {
                    self.calls.insert(id, single_line(&call.name));
                }
                self.line(
                    &format!("● {:13} {}", single_line(&call.name), target(call)),
                    Ink::Normal,
                )?;
            }
            EventKind::ToolCallStarted => self.state = "tool",
            EventKind::PermissionRequested { command, workspace } => {
                let approval = format!(
                    "? permission   shell approval required\n  cwd: {}\n  command: {}\n  Runs with your account's permissions.\n",
                    single_line(workspace),
                    printable(command)
                );
                if self.screen.as_ref().is_some_and(|screen| {
                    let needed: usize = approval
                        .lines()
                        .map(|line| line.width().div_ceil(screen.content_width()).max(1))
                        .sum();
                    needed + 1 > screen.bottom - screen.top + 1
                }) {
                    // A command must remain reviewable before yes is accepted.
                    // Oversized approvals use the terminal's ordinary scrollback.
                    self.restore()?;
                }
                self.line("? permission   shell approval required", Ink::Accent)?;
                // Show the complete command, not a truncated permission target.
                self.emit(
                    &format!(
                        "  cwd: {}\n  command: {}\n  Runs with your account's permissions.\n",
                        single_line(workspace),
                        printable(command)
                    ),
                    Ink::Normal,
                    false,
                )?;
                self.waiting = true;
                self.state = "permission";
                if self.screen.is_none() {
                    // Keep approvals separate from piped stdout/stderr.
                    if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
                        write!(
                            tty,
                            "\nShell command in {}:\n{}\nRuns with your account's permissions.\n› type yes to approve: ",
                            single_line(workspace),
                            printable(command)
                        )?;
                        tty.flush()?;
                    }
                }
            }
            EventKind::PermissionGranted
            | EventKind::PermissionDenied
            | EventKind::PermissionCancelled
            | EventKind::PermissionFailed { .. } => {
                self.waiting = false;
                self.state = "running";
                match &event.kind {
                    EventKind::PermissionGranted => {
                        self.line("✓ permission   granted", Ink::Dim)?
                    }
                    EventKind::PermissionDenied => {
                        self.line("! permission   denied", Ink::Normal)?
                    }
                    EventKind::PermissionFailed { message } => {
                        self.line(&format!("× permission   {message}"), Ink::Normal)?
                    }
                    _ => self.line("! permission   cancelled", Ink::Normal)?,
                }
            }
            EventKind::ToolCallCompleted { outcome }
            | EventKind::ToolCallFailed { outcome }
            | EventKind::ToolCallDenied { outcome }
            | EventKind::ToolCallTimedOut { outcome } => {
                let name = event
                    .tool_call_id
                    .and_then(|id| self.calls.remove(&id))
                    .unwrap_or_else(|| "tool".into());
                let (symbol, state) = match &event.kind {
                    EventKind::ToolCallCompleted { .. } => ("✓", "completed"),
                    EventKind::ToolCallDenied { .. } => ("!", "denied"),
                    EventKind::ToolCallTimedOut { .. } => ("×", "timed out"),
                    _ => ("×", "failed"),
                };
                self.line(
                    &format!("{symbol} {state:13} {name} · {}", summary(outcome)),
                    if symbol == "✓" {
                        Ink::Dim
                    } else {
                        Ink::Normal
                    },
                )?;
                if name == "shell" {
                    self.command_output(outcome)?;
                }
            }
            EventKind::ToolCallCancelled | EventKind::ToolCallSkipped { .. } => {
                let name = event
                    .tool_call_id
                    .and_then(|id| self.calls.remove(&id))
                    .unwrap_or_else(|| "tool".into());
                let detail = match &event.kind {
                    EventKind::ToolCallSkipped { reason } => format!("skipped · {reason}"),
                    _ => "cancelled".into(),
                };
                self.line(&format!("! {name:13} {detail}"), Ink::Normal)?;
            }
            EventKind::CancellationRequested => {
                self.state = "cancelling";
                self.line("! run          cancellation requested", Ink::Normal)?;
            }
            EventKind::RunCompleted { .. } => {
                self.state = "completed";
                self.done = true;
                self.line("✓ run          completed", Ink::Accent)?;
            }
            EventKind::RunCancelled => {
                self.state = "cancelled";
                self.done = true;
                self.waiting = false;
                self.line("! run          cancelled", Ink::Normal)?;
            }
            EventKind::RunFailed { code, message } => {
                self.state = "failed";
                self.done = true;
                self.line(&format!("× run          {code}: {message}"), Ink::Normal)?;
            }
            EventKind::ModelCallLimitReached { limit, .. } => {
                self.state = "limit reached";
                self.done = true;
                self.line(
                    &format!(
                        "! run          model-call limit {limit}; final tool results uninspected"
                    ),
                    Ink::Normal,
                )?;
            }
            _ => {}
        }
        self.resize()?;
        let status = self.status();
        if let Some(screen) = &self.screen {
            screen.footer(&mut io::stderr().lock(), &status, self.waiting, self.done)?;
        }
        Ok(())
    }
    fn command_output(&mut self, outcome: &ToolOutcome) -> io::Result<()> {
        if let ToolOutcome::Success { data } | ToolOutcome::TimedOut { data } = outcome {
            for stream in ["stdout", "stderr"] {
                if let Some(text) = data[stream].as_str().filter(|text| !text.is_empty()) {
                    self.line(&format!("  {stream}"), Ink::Dim)?;
                    self.emit(
                        &format!("{}\n", printable(text).trim_end()),
                        Ink::Normal,
                        false,
                    )?;
                }
            }
        }
        Ok(())
    }
    pub fn finish(&mut self, model_calls: usize, tool_calls: usize) -> io::Result<()> {
        self.line(
            &format!("  {model_calls} model calls · {tool_calls} tool outcomes"),
            Ink::Dim,
        )?;
        if let Some(screen) = &self.screen {
            screen.footer(&mut io::stderr().lock(), &self.status(), false, true)?;
        }
        self.restore()
    }
    fn restore(&mut self) -> io::Result<()> {
        if let Some(screen) = self.screen.take() {
            let mut out = io::stderr().lock();
            // Return to normal scroll margins and leave the shell below the UI.
            writeln!(out, "\x1b[r\x1b[?25h\x1b[{};1H", screen.height)?;
            out.flush()?;
        }
        Ok(())
    }
}
impl Drop for Console {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
fn target(call: &ToolCall) -> String {
    let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
    let parts: Vec<_> = ["command", "path", "pattern"]
        .iter()
        .filter_map(|key| args[key].as_str())
        .collect();
    if parts.is_empty() {
        "invalid or empty arguments".into()
    } else {
        parts.join(" · ")
    }
}
fn summary(outcome: &ToolOutcome) -> String {
    match outcome {
        ToolOutcome::Error { code, message } => format!("{code}: {message}"),
        ToolOutcome::Success { data } | ToolOutcome::TimedOut { data } => {
            if data["timed_out"] == true {
                return "output unavailable after timeout".into();
            }
            if let Some(text) = data["content"].as_str() {
                return format!("{} lines", text.lines().count());
            }
            if let Some(bytes) = data["bytes_written"].as_u64() {
                return format!("{bytes} bytes written");
            }
            for (key, label) in [
                ("matches", "matches"),
                ("files", "files"),
                ("entries", "entries"),
            ] {
                if let Some(items) = data[key].as_array() {
                    return format!("{} {label}", items.len());
                }
            }
            if let Some(code) = data["exit_code"].as_i64() {
                return format!("exit {code}");
            }
            if data.get("exit_code").is_some() {
                return "no exit code (signal)".into();
            }
            "result available".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn identity() -> Identity {
        Identity {
            model: "gpt-5.6-sol".into(),
            cwd: "~/Developer/astrid".into(),
            instructions: Some("AGENTS.md".into()),
            tools: tools::definitions()
                .iter()
                .map(|tool| tool["name"].as_str().unwrap().to_owned())
                .collect(),
        }
    }
    #[test]
    fn text_is_sanitized_and_truncated_by_terminal_cells() {
        assert_eq!(truncate("a界b", 3), "a…");
        assert_eq!(truncate("a界b", 4), "a界b");
        assert_eq!(truncate("\x1b\x07hello\nworld", 8), "hello w…");
        assert_eq!(truncate("hello", 0), "");
        assert_eq!(truncate("hello", 1), "…");
        for width in 0..80 {
            for row in identity().rows(width) {
                assert!(row.width() <= width);
            }
            assert!(tool_list(&identity().tools, width).width() <= width);
        }
        assert!(tool_list(&identity().tools, 78).contains("+2 more"));
    }
    #[test]
    fn compact_results_keep_runtime_completion_distinct_from_shell_exit() {
        assert_eq!(
            summary(&ToolOutcome::Success {
                data: json!({"content":"a\nb\n"})
            }),
            "2 lines"
        );
        assert_eq!(
            summary(&ToolOutcome::Success {
                data: json!({"exit_code":101})
            }),
            "exit 101"
        );
        assert_eq!(
            summary(&ToolOutcome::TimedOut {
                data: json!({"timed_out":true,"stdout":null})
            }),
            "output unavailable after timeout"
        );
    }
    #[test]
    fn viewport_reserves_header_footer_and_wraps_stream_at_all_sizes() {
        for (width, height) in [(120, 40), (80, 24), (40, 30), (24, 12), (8, 8)] {
            let mut screen = Screen::new(width, height, false);
            let mut bytes = Vec::new();
            screen.draw(&mut bytes, &identity()).unwrap();
            for _ in 0..100 {
                screen
                    .append(&mut bytes, "source 界 line\n", Ink::Normal)
                    .unwrap();
            }
            screen
                .footer(
                    &mut bytes,
                    "RUN 82ac1   TURN 4   MODEL gpt-5.6-sol",
                    true,
                    false,
                )
                .unwrap();
            assert!(screen.top > 1);
            assert!(screen.top < screen.bottom);
            assert_eq!(screen.bottom, height - 3);
            assert!(screen.row <= screen.bottom);
            assert!(
                screen
                    .history
                    .iter()
                    .all(|(line, _)| line.width() <= width - 2)
            );
            let text = String::from_utf8(bytes).unwrap();
            assert!(!text.contains("\x1b[38;2;"));
            assert!(!text.contains("\x1b[2m"));
            assert!(text.contains(&format!("\x1b[{};1H", height - 1)));
            assert!(text.ends_with("\x1b[?25h"));
        }
    }
    #[test]
    fn resize_rewraps_without_losing_retained_text() {
        let mut screen = Screen::new(80, 24, false);
        let mut output = Vec::new();
        screen.draw(&mut output, &identity()).unwrap();
        let text = "long source file path with unicode 界 and model output";
        screen.append(&mut output, text, Ink::Normal).unwrap();
        screen.width = 20;
        screen.rewrap();
        assert_eq!(
            screen
                .history
                .iter()
                .map(|(line, _)| line.as_str())
                .collect::<String>(),
            text
        );
        assert!(screen.history.iter().all(|(line, _)| line.width() <= 18));
    }
    #[test]
    fn render_terminal_samples() {
        // Optional artifacts for a terminal emulator and visual QA; no live model.
        let directory = std::env::var_os("ASTRID_UI_CAPTURE_DIR");
        for (width, height) in [(120, 36), (80, 24), (42, 32), (24, 12)] {
            let mut screen = Screen::new(width, height, true);
            let mut bytes = Vec::new();
            screen.draw(&mut bytes, &identity()).unwrap();
            screen
                .append(&mut bytes, "› find and fix the failing test\n\n", Ink::Dim)
                .unwrap();
            screen
                .append(
                    &mut bytes,
                    "I found the greeting test. I'll inspect the implementation.\n\n",
                    Ink::Normal,
                )
                .unwrap();
            screen.append(&mut bytes, "● read_file     src/lib.rs\n✓ completed     read_file · 214 lines\n\n● shell         cargo test\n? permission    shell approval required\n  cwd: ~/Developer/astrid\n  command: cargo test\n  Runs with your account's permissions.\n", Ink::Normal).unwrap();
            screen
                .footer(
                    &mut bytes,
                    "RUN 82ac1   TURN 4   MODEL gpt-5.6-sol   CALLS 4   permission",
                    true,
                    false,
                )
                .unwrap();
            if let Some(directory) = &directory {
                std::fs::create_dir_all(directory).unwrap();
                std::fs::write(
                    Path::new(directory).join(format!("{width}x{height}.ansi")),
                    bytes,
                )
                .unwrap();
            }
        }
    }
}
