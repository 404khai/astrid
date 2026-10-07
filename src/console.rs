//! CLI presentation only. Runtime events remain the source of execution semantics.
use astrid::{
    events::{EventKind, ExecutionEvent, ToolCallId},
    model::{ToolCall, ToolOutcome},
    output::{OutputStream, TextDecoder},
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
    Success,
    Warning,
    Error,
    Reply,
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
            Self::Success => "32",
            Self::Warning => "33",
            Self::Error => "31",
            Self::Reply => "36",
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
    color: bool,
    calls: BTreeMap<ToolCallId, String>,
    run: String,
    turn: usize,
    model_calls: usize,
    state: &'static str,
    waiting: bool,
    done: bool,
    text_open: bool,
    tool_output_open: bool,
    active_stream: Option<(ToolCallId, OutputStream)>,
    decoders: BTreeMap<(ToolCallId, OutputStream), TextDecoder>,
}
impl Console {
    pub fn new(model: &str, workspace: &Workspace) -> io::Result<Self> {
        let terminal = io::stdout().is_terminal() && io::stderr().is_terminal();
        let capable = std::env::var_os("ASTRID_FIXED_VIEWPORT").is_some()
            && terminal
            && std::env::var("TERM").is_ok_and(|term| term != "dumb");
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
            color: terminal
                && std::env::var_os("NO_COLOR").is_none()
                && std::env::var("TERM").is_ok_and(|term| term != "dumb"),
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
            tool_output_open: false,
            active_stream: None,
            decoders: BTreeMap::new(),
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
            write!(out, "{}", ink.paint(&printable(text), self.color))?;
            out.flush()
        } else {
            let mut out = io::stderr().lock();
            write!(out, "{}", ink.paint(&printable(text), self.color))?;
            out.flush()
        }
    }
    fn line(&mut self, text: &str, ink: Ink) -> io::Result<()> {
        if self.tool_output_open {
            self.emit("\n", Ink::Normal, false)?;
            self.tool_output_open = false;
            self.active_stream = None;
        }
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
                self.line(&format!("› {}", single_line(task)), Ink::Accent)?;
                self.line("", Ink::Normal)?;
                self.state = "running";
            }
            EventKind::TurnStarted { number } => self.turn = *number,
            EventKind::ModelCallStarted { number } => {
                self.model_calls = *number;
                self.state = "model";
            }
            EventKind::ModelTextDelta { text } => {
                self.emit(text, Ink::Reply, true)?;
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
            EventKind::PermissionsConfigured { policy } => {
                self.line(
                    &format!(
                        "  policy       read={:?} write={:?} shell={:?}",
                        policy.read, policy.write, policy.execute
                    ),
                    Ink::Dim,
                )?;
            }
            EventKind::WorkspaceBaseline { git, complete, .. } => {
                self.line(
                    &format!(
                        "  workspace    branch={} · {} initial dirty paths · evidence {}",
                        git.branch.as_deref().unwrap_or(if git.detached {
                            "detached HEAD"
                        } else {
                            "unavailable"
                        }),
                        git.dirty.len(),
                        if *complete {
                            "complete within scope"
                        } else {
                            "incomplete"
                        }
                    ),
                    Ink::Dim,
                )?;
                for path in git.dirty.iter().take(20) {
                    self.line(
                        &format!(
                            "    {} · staged={} unstaged={} untracked={}",
                            path.path, path.staged, path.unstaged, path.untracked
                        ),
                        Ink::Dim,
                    )?;
                }
                if git.dirty.len() > 20 {
                    self.line(
                        "    additional dirty paths retained in runtime evidence",
                        Ink::Dim,
                    )?;
                }
                for error in &git.errors {
                    self.line(&format!("    Git: {error}"), Ink::Dim)?;
                }
            }
            EventKind::WorkspaceChanges { report } => {
                self.line(
                    &format!(
                        "  changes      {} observed paths · evidence {}",
                        report.changes.len(),
                        if report.complete {
                            "complete within scope"
                        } else {
                            "incomplete"
                        }
                    ),
                    Ink::Dim,
                )?;
                self.line(&report.attribution, Ink::Dim)?;
                for change in &report.changes {
                    self.line(
                        &format!("    {} · {}", change.path, change.kind),
                        Ink::Normal,
                    )?;
                    if let Some(patch) = &change.patch {
                        self.emit(patch, Ink::Normal, false)?;
                    }
                    if let Some(reason) = &change.unavailable {
                        self.line(&format!("    evidence unavailable: {reason}"), Ink::Dim)?;
                    }
                }
                for error in &report.errors {
                    self.line(&format!("    evidence: {error}"), Ink::Dim)?;
                }
            }
            EventKind::NativeMutationRecorded { evidence } => {
                self.line(
                    &format!(
                        "  mutation     {} · {}",
                        evidence.path,
                        if evidence.complete {
                            "before/after recorded"
                        } else {
                            "evidence incomplete"
                        }
                    ),
                    Ink::Dim,
                )?;
                if let Some(change) = &evidence.change {
                    if let Some(patch) = &change.patch {
                        self.emit(patch, Ink::Normal, false)?;
                    }
                    if let Some(reason) = &change.unavailable {
                        self.line(&format!("    evidence unavailable: {reason}"), Ink::Dim)?;
                    }
                }
                for error in &evidence.errors {
                    self.line(&format!("    evidence: {error}"), Ink::Dim)?;
                }
            }
            EventKind::PermissionPolicyEvaluated {
                capability,
                action,
                reason,
            } => {
                self.line(
                    &format!("  permission   {capability:?} {action:?} · {reason}"),
                    Ink::Dim,
                )?;
            }
            EventKind::ToolOutput { output } => {
                if let Some(id) = event.tool_call_id {
                    let key = (id, output.stream);
                    let text = self.decoders.entry(key).or_default().push(&output.bytes);
                    if !text.is_empty() {
                        if self.active_stream != Some(key) {
                            self.line(
                                &format!(
                                    "  {}",
                                    if output.stream == OutputStream::Stdout {
                                        "stdout"
                                    } else {
                                        "stderr"
                                    }
                                ),
                                Ink::Dim,
                            )?;
                        }
                        self.emit(&text, Ink::Normal, false)?;
                        self.tool_output_open = true;
                        self.active_stream = Some(key);
                    }
                }
            }
            EventKind::ToolCallStarted => self.state = "tool",
            EventKind::PermissionRequested { command, workspace } => {
                let shell = event
                    .tool_call_id
                    .and_then(|id| self.calls.get(&id))
                    .is_some_and(|name| name == "shell");
                let authority = if shell {
                    "Runs with your account's permissions."
                } else {
                    "Workspace operation subject to validated paths."
                };
                let approval = format!(
                    "? permission   operation approval required\n  cwd: {}\n  command: {}\n  {authority}\n",
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
                self.line("? permission   operation approval required", Ink::Warning)?;
                // Show the complete command, not a truncated permission target.
                self.emit(
                    &format!(
                        "  cwd: {}\n  command: {}\n  {authority}\n",
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
                            "\nOperation in {}:\n{}\n{authority}\n› type yes to approve: ",
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
                        self.line("✓ permission   granted", Ink::Success)?
                    }
                    EventKind::PermissionDenied => {
                        self.line("! permission   denied", Ink::Warning)?
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
                        Ink::Success
                    } else {
                        Ink::Error
                    },
                )?;
                if name == "shell" {
                    self.command_output(outcome)?;
                }
            }
            EventKind::ToolCallCancelled { .. } | EventKind::ToolCallSkipped { .. } => {
                let name = event
                    .tool_call_id
                    .and_then(|id| self.calls.remove(&id))
                    .unwrap_or_else(|| "tool".into());
                let detail = match &event.kind {
                    EventKind::ToolCallSkipped { reason } => format!("skipped · {reason}"),
                    _ => "cancelled".into(),
                };
                self.line(&format!("! {name:13} {detail}"), Ink::Error)?;
                if let EventKind::ToolCallCancelled { output: Some(data) } = &event.kind {
                    self.command_output(&ToolOutcome::Success { data: data.clone() })?;
                }
            }
            EventKind::CancellationRequested => {
                self.state = "cancelling";
                self.line("! run          cancellation requested", Ink::Normal)?;
            }
            EventKind::RunCompleted { .. } => {
                self.state = "completed";
                self.done = true;
                self.line("✓ run          completed", Ink::Success)?;
            }
            EventKind::RunCancelled => {
                self.state = "cancelled";
                self.done = true;
                self.waiting = false;
                self.line("! run          cancelled", Ink::Warning)?;
            }
            EventKind::RunFailed { code, message } => {
                self.state = "failed";
                self.done = true;
                self.line(&format!("× run          {code}: {message}"), Ink::Error)?;
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
                let metadata = &data["output"][stream];
                let queued = metadata["live_queued_bytes"].as_u64().unwrap_or(0);
                if queued == 0
                    && let Some(text) = data[stream].as_str().filter(|text| !text.is_empty())
                {
                    self.line(&format!("  {stream}"), Ink::Dim)?;
                    self.emit(
                        &format!("{}\n", printable(text).trim_end()),
                        Ink::Normal,
                        false,
                    )?;
                }
                if metadata["truncated"] == true
                    || metadata["text_unavailable_suffix_bytes"]
                        .as_u64()
                        .unwrap_or(0)
                        > 0
                    || metadata["live_omitted_bytes"].as_u64().unwrap_or(0) > 0
                {
                    self.line(&format!("  {stream}: observed={} captured={} capture omitted={} live omitted={} text suffix unavailable={} complete={}",
                        metadata["observed_bytes"], metadata["captured_bytes"], metadata["capture_omitted_bytes"], metadata["live_omitted_bytes"], metadata["text_unavailable_suffix_bytes"], metadata["complete"]), Ink::Dim)?;
                }
            }
        }
        self.decoders.clear();
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
                return if data["stdout"].is_null() && data["stderr"].is_null() {
                    "output unavailable after timeout".into()
                } else {
                    "partial output retained; unread output unavailable".into()
                };
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
            screen.append(&mut bytes, "● read_file     src/lib.rs\n✓ completed     read_file · 214 lines\n\n● shell         cargo test\n? permission    shell approval required\n  cwd: ~/Developer/astrid\n  command: cargo test\n  {authority}\n", Ink::Normal).unwrap();
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
