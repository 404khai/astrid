use super::format::truncate;
use super::*;
use super::{
    identity::{append_header, tool_list},
    legacy::Screen,
    state::summary,
    theme::Ink,
};
use astrid::{model::ToolOutcome, tools};
use serde_json::json;
use std::path::Path;
use unicode_width::UnicodeWidthStr;

#[test]
fn designed_header_preserves_logo_metadata_and_scrollback() {
    for width in [40, 80, 120] {
        let mut bytes = Vec::new();
        append_header(&mut bytes, &identity(), width, true).unwrap();
        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains("astrid"));
        assert!(output.contains("model"));
        assert!(output.contains("cwd"));
        assert!(output.contains("38;2;68;89;249"));
        assert!(output.contains("38;2;0;247;213"));
        assert!(!output.contains("\x1b[2J"));
        assert!(!output.contains("\x1b[r"));
    }
    let mut bytes = Vec::new();
    append_header(&mut bytes, &identity(), 80, false).unwrap();
    assert!(!bytes.contains(&0x1b));
}
fn identity() -> Identity {
    Identity {
        mode: "auto".into(),
        model: "gpt-5.6-sol".into(),
        provider: "OpenAI / Codex subscription".into(),
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

use super::{
    inline::Inline,
    input::{Action, ApprovalInput, Editor},
    state::{Piece, Presentation},
};
use astrid::{
    events::{EventKind, ExecutionEvent},
    output::{OutputStream, ToolOutput},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, TerminalOptions, Viewport, backend::TestBackend, buffer::Buffer};

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn ctrl(c: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
}
pub(super) fn event(kind: EventKind) -> ExecutionEvent {
    ExecutionEvent {
        session_id: Default::default(),
        run_id: Default::default(),
        sequence: 1,
        turn_id: None,
        model_call_id: None,
        tool_call_id: None,
        kind,
    }
}
fn text(buffer: &Buffer) -> String {
    buffer
        .content
        .chunks(buffer.area.width.max(1) as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn test_inline(w: u16, h: u16) -> Inline<TestBackend> {
    Inline::new(
        Terminal::with_options(
            TestBackend::new(w, h),
            TerminalOptions {
                viewport: Viewport::Inline(5),
            },
        )
        .unwrap(),
        false,
    )
}

#[test]
fn input_preserves_actions_and_paste_is_data() {
    let mut editor = Editor::new(None, "test");
    assert_eq!(
        editor.handle(Event::Paste("first\r\nsecond".into())),
        Action::Continue
    );
    assert_eq!(editor.text(), "first\nsecond");
    editor.handle(ctrl('j'));
    editor.handle(key(KeyCode::Char('界')));
    assert_eq!(
        editor.handle(key(KeyCode::Enter)),
        Action::Submit("first\nsecond\n界".into())
    );
    assert_eq!(editor.handle(ctrl('c')), Action::Cancel);
    editor.handle(key(KeyCode::Esc));
    assert_eq!(editor.handle(ctrl('d')), Action::Submit("/quit".into()));
    editor.handle(key(KeyCode::Char('/')));
    editor.handle(key(KeyCode::Down));
    assert_eq!(editor.handle(key(KeyCode::Enter)), Action::Continue); // /help expands commands
    editor.handle(key(KeyCode::Down));
    editor.handle(key(KeyCode::Down));
    assert_eq!(
        editor.handle(key(KeyCode::Enter)),
        Action::Submit("/quit".into())
    );
    editor.handle(key(KeyCode::Esc));
    editor.handle(Event::Paste("/m".into()));
    editor.handle(key(KeyCode::Tab));
    assert_eq!(editor.text(), "/model");
    assert_eq!(
        editor.handle(key(KeyCode::Enter)),
        Action::Submit("/model".into())
    );
    let models = vec!["small".into(), "strong".into()];
    let mut picker = Editor::new(Some(&models), "strong");
    assert_eq!(
        picker.handle(key(KeyCode::Enter)),
        Action::Submit("strong".into())
    );
    assert_eq!(picker.handle(key(KeyCode::Esc)), Action::Dismiss);
    picker.handle(Event::Paste("sm".into()));
    assert_eq!(
        picker.handle(key(KeyCode::Enter)),
        Action::Submit("small".into())
    );
}
#[test]
fn editor_navigation_and_deletion_preserve_graphemes() {
    let mut editor = Editor::new(None, "test");
    editor.handle(Event::Paste("a界e\u{301}👩‍💻".into()));
    editor.handle(key(KeyCode::Left));
    assert_eq!(editor.textarea.cursor().1, 4); // a, 界, e, accent
    editor.handle(key(KeyCode::Delete));
    assert_eq!(editor.text(), "a界e\u{301}");
    editor.handle(key(KeyCode::Backspace));
    assert_eq!(editor.text(), "a界");
    editor.handle(key(KeyCode::Backspace));
    assert_eq!(editor.text(), "a");
    editor.handle(Event::Resize(8, 8));
    assert_eq!(editor.text(), "a");
}
#[test]
fn approvals_require_arming_and_never_accept_paste() {
    let mut input = ApprovalInput::default();
    for c in "yes".chars() {
        input.handle(key(KeyCode::Char(c)));
    }
    assert_eq!(input.handle(key(KeyCode::Enter)), Action::Continue);
    input.arm();
    assert_eq!(input.handle(Event::Paste("yes\n".into())), Action::Continue);
    assert_eq!(
        input.handle(key(KeyCode::Enter)),
        Action::Submit(String::new())
    );
    input.arm();
    for c in "YeS".chars() {
        input.handle(key(KeyCode::Char(c)));
    }
    assert_eq!(
        input.handle(key(KeyCode::Enter)),
        Action::Submit("YeS".into())
    );
    assert_eq!(input.handle(ctrl('c')), Action::Cancel);
    input.arm();
    assert_eq!(
        input.handle(key(KeyCode::Esc)),
        Action::Submit(String::new())
    );
}
#[test]
fn event_projection_keeps_streams_interruption_and_final_text_distinct() {
    let mut state = Presentation::new("test".into());
    let id = Default::default();
    let mut pieces = Vec::new();
    for kind in [
        EventKind::ModelTextDelta {
            text: "partial".into(),
        },
        EventKind::ModelCallCancelled,
        EventKind::ToolCallRequested {
            call: astrid::model::ToolCall {
                call_id: "external".into(),
                name: "shell".into(),
                arguments: "{\"command\":\"printf\"}".into(),
            },
        },
        EventKind::ToolOutput {
            output: ToolOutput {
                stream: OutputStream::Stdout,
                bytes: vec![0xe7, 0x95],
            },
        },
        EventKind::ToolOutput {
            output: ToolOutput {
                stream: OutputStream::Stdout,
                bytes: vec![0x8c, b'!'],
            },
        },
        EventKind::ToolOutput {
            output: ToolOutput {
                stream: OutputStream::Stderr,
                bytes: b"error".to_vec(),
            },
        },
        EventKind::ToolCallCompleted {
            outcome: ToolOutcome::Success {
                data: json!({"exit_code":101,"stdout":"界!","stderr":"error","output":{"stdout":{"live_queued_bytes":4},"stderr":{"live_queued_bytes":5,"live_omitted_bytes":2,"truncated":true}}}),
            },
        },
        EventKind::RunCompleted {
            final_text: "partial".into(),
        },
    ] {
        let mut e = event(kind);
        e.tool_call_id = Some(id);
        state.apply(&e);
        pieces.extend(state.take_output());
    }
    let output = pieces.iter().map(|p| p.text.as_str()).collect::<String>();
    assert_eq!(output.matches("partial").count(), 1);
    assert!(output.contains("response interrupted"));
    assert_eq!(output.matches("界!").count(), 1);
    assert_eq!(output.matches("error").count(), 1);
    assert!(output.contains("stdout\n界!\n  stderr\nerror\n"));
    assert!(output.contains("exit 101"));
    assert!(output.contains("live omitted=2"));
    assert!(output.contains("run          completed"));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    for piece in pieces {
        append::write_piece(&mut stdout, &mut stderr, &piece, false).unwrap();
    }
    assert_eq!(String::from_utf8(stdout).unwrap(), "partial\n");
    assert!(!stderr.contains(&27));
}
#[test]
fn inline_tail_reflows_then_commits_once_without_newline() {
    let mut ui = test_inline(40, 12);
    let state = Presentation::new("test".into());
    let approval = ApprovalInput::default();
    let original = "unique e\u{301} 界 👩‍💻";
    ui.push(&Piece {
        text: original.into(),
        ink: Ink::Reply,
        model: true,
    })
    .unwrap();
    ui.draw(&state, &approval).unwrap();
    assert!(text(ui.terminal.backend().buffer()).contains("unique"));
    ui.terminal.backend_mut().resize(8, 8);
    ui.draw(&state, &approval).unwrap();
    assert_eq!(ui.transcript.tail, original); // three wrapped rows still fit
    ui.terminal.backend_mut().resize(80, 12);
    ui.draw(&state, &approval).unwrap();
    assert_eq!(ui.transcript.tail, original);
    ui.push(&Piece {
        text: "\n".into(),
        ink: Ink::Normal,
        model: true,
    })
    .unwrap();
    ui.draw(&state, &approval).unwrap();
    ui.close().unwrap();
    let all = format!(
        "{}\n{}",
        text(ui.terminal.backend().scrollback()),
        text(ui.terminal.backend().buffer())
    );
    assert_eq!(all.matches("unique").count(), 1);
}
#[test]
fn inline_bursts_bound_pending_work_and_retain_every_row() {
    let mut ui = test_inline(80, 24);
    let state = Presentation::new("test".into());
    let approval = ApprovalInput::default();
    let start = std::time::Instant::now();
    for index in 0..600 {
        ui.push(&Piece {
            text: format!("row {index:04} {}\n", "界".repeat(10)),
            ink: Ink::Normal,
            model: false,
        })
        .unwrap();
        assert!(ui.transcript.pending_bytes() < 25000);
    }
    ui.push(&Piece {
        text: "z".repeat(50000),
        ink: Ink::Reply,
        model: true,
    })
    .unwrap();
    assert!(ui.transcript.pending_bytes() < 25000);
    ui.draw(&state, &approval).unwrap();
    ui.close().unwrap();
    let all = format!(
        "{}\n{}",
        text(ui.terminal.backend().scrollback()),
        text(ui.terminal.backend().buffer())
    );
    for index in 0..600 {
        assert_eq!(all.matches(&format!("row {index:04}")).count(), 1);
    }
    assert_eq!(all.chars().filter(|c| *c == 'z').count(), 50000);
    eprintln!(
        "inline fixture: 600 rows + 50k no-newline bytes rendered in {:?}",
        start.elapsed()
    );
}
#[test]
fn layouts_survive_tiny_sizes_and_color_disabled() {
    for (w, h) in [(1, 1), (2, 2), (8, 8), (24, 12), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        let mut editor = Editor::new(None, "test");
        editor.handle(Event::Paste("界e\u{301}\n👩‍💻".into()));
        terminal
            .draw(|f| widgets::composer(f, &mut editor, "test", "", false))
            .unwrap();
        terminal
            .draw(|f| widgets::status(f, "running", "yes", true, false, false))
            .unwrap();
        assert!(!text(terminal.backend().buffer()).contains('\x1b'));
    }
    use super::terminal::supports_inline;
    assert!(supports_inline(
        true,
        true,
        true,
        Some("xterm"),
        Some((80, 24))
    ));
    for flags in [
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ] {
        assert!(!supports_inline(
            flags.0,
            flags.1,
            flags.2,
            Some("xterm"),
            Some((80, 24))
        ));
    }
    assert!(!supports_inline(
        true,
        true,
        true,
        Some("dumb"),
        Some((80, 24))
    ));
    assert!(!supports_inline(true, true, true, Some("xterm"), None));
}

#[test]
fn append_output_surfaces_broken_pipe() {
    struct Closed;
    impl std::io::Write for Closed {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        append::write_piece(
            &mut Closed,
            &mut Vec::new(),
            &Piece {
                text: "reply".into(),
                ink: Ink::Reply,
                model: true
            },
            false
        )
        .unwrap_err()
        .kind(),
        std::io::ErrorKind::BrokenPipe
    );
}

#[test]
fn theme_and_input_limits_remain_explicit() {
    let mut editor = Editor::new(None, "test");
    editor.handle(Event::Paste("x".repeat(65537)));
    assert!(editor.text().is_empty());
    assert!(editor.notice.is_some());
    editor.handle(Event::Paste("/exit".into()));
    assert_eq!(
        editor.handle(key(KeyCode::Enter)),
        Action::Submit("/exit".into())
    );
    assert_eq!(
        super::theme::composer_style(true).bg,
        Some(ratatui::style::Color::Indexed(235))
    );
    assert_eq!(
        super::theme::selection_style(true).fg,
        Some(ratatui::style::Color::Indexed(208))
    );
    assert_eq!(
        super::theme::composer_style(false),
        ratatui::style::Style::default()
    );
}

#[test]
fn context_uncertainty_and_workspace_patches_survive_projection() {
    let mut state = Presentation::new("test".into());
    state.show_context = true;
    let snapshot = astrid::context::ContextSnapshot {
        measurements: vec![],
        serialized_request_bytes: 1234,
        non_opaque_json_size_token_heuristic: 309,
        provider_input_tokens: None,
    };
    state.apply(&event(EventKind::ContextPrepared { snapshot }));
    state.apply(&event(EventKind::WorkspaceChanges {
        report: astrid::changes::WorkspaceReport {
            before_git: Default::default(),
            after_git: Default::default(),
            complete: false,
            errors: vec!["bounded evidence".into()],
            attribution: "observed changes".into(),
            changes: vec![astrid::changes::Change {
                path: "utf8:src/lib.rs".into(),
                kind: "modified".into(),
                before_mode: None,
                after_mode: None,
                patch: Some("--- src/lib.rs\n+++ src/lib.rs\n-old\n+new\n".into()),
                unavailable: None,
            }],
        },
    }));
    let output = state
        .take_output()
        .into_iter()
        .map(|p| p.text)
        .collect::<String>();
    assert!(output.contains("1234 serialized request bytes; provider input tokens unavailable"));
    assert!(output.contains("not a tokenizer count or full context total"));
    assert!(output.contains("evidence incomplete"));
    assert!(output.contains("-old\n+new\n"));
    assert!(output.contains("bounded evidence"));
}

#[test]
fn full_input_still_allows_navigation_and_deletion() {
    let mut editor = Editor::new(None, "test");
    editor.handle(Event::Paste("x".repeat(65535)));
    editor.handle(key(KeyCode::Char('界'))); // three bytes cannot fit
    assert_eq!(editor.text().len(), 65535);
    editor.handle(key(KeyCode::Char('y')));
    assert_eq!(editor.text().len(), 65536);
    editor.handle(ctrl('j'));
    assert_eq!(editor.text().len(), 65536);
    editor.handle(key(KeyCode::Home));
    assert_eq!(editor.textarea.cursor().1, 0);
    editor.handle(key(KeyCode::Delete));
    assert_eq!(editor.text().len(), 65535);
}

#[test]
fn unbound_palette_and_mode_visibility_use_effective_authority() {
    let mut id = identity();
    id.mode = "unbound".into();
    let mut bytes = Vec::new();
    append_header(&mut bytes, &id, 80, true).unwrap();
    let output = String::from_utf8(bytes).unwrap();
    assert!(output.contains("unbound"));
    assert!(output.contains("\x1b[38;2;249;68;71mastrid"));
    assert!(output.contains("38;2;249;68;71"));
    assert!(output.contains("38;2;236;26;29"));
    assert!(output.contains("38;2;247;198;0"));
    let upper = theme::paint_logo("█", 0, true, true);
    let middle = theme::paint_logo("█", 6, true, true);
    let lower = theme::paint_logo("█", 9, true, true);
    assert_eq!(upper, lower);
    assert_ne!(upper, middle);
    let mut bytes = Vec::new();
    append_header(&mut bytes, &id, 80, false).unwrap();
    assert!(!bytes.contains(&27));
    let mut state = Presentation::new("test".into());
    state.apply(&event(EventKind::PermissionsConfigured {
        policy: astrid::permissions::PermissionMode::Unbound.policy(),
    }));
    for width in [8, 24, 80] {
        assert!(state.status(width).starts_with("unbound"));
    }
    state.apply(&event(EventKind::PermissionsConfigured {
        policy: astrid::permissions::PermissionPolicy {
            read: astrid::permissions::PermissionAction::Deny,
            ..Default::default()
        },
    }));
    assert!(state.status(80).starts_with("custom"));
}

#[test]
fn slash_mode_arguments_submit_as_commands_and_unknown_commands_reach_dispatch() {
    for command in ["/mode ask", "/mode auto", "/mode unbound", "/unknown"] {
        let mut editor = Editor::new(None, "test");
        editor.handle(Event::Paste(command.into()));
        assert_eq!(
            editor.handle(key(KeyCode::Enter)),
            Action::Submit(command.into())
        );
    }
    let editor = Editor::new(None, "test");
    let mut editor = editor;
    editor.handle(Event::Paste("/".into()));
    let names = editor
        .options()
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    for command in ["/mode", "/sessions", "/new"] {
        assert!(names.iter().any(|name| name == command));
    }
}

#[test]
fn expanded_tool_results_are_optional_and_bounded() {
    for expanded in [false, true] {
        let mut state = Presentation::new("test".into());
        state.expanded_tool_calls = expanded;
        state.apply(&event(EventKind::ToolCallCompleted {
            outcome: ToolOutcome::Success { data: json!({"content": (0..20).map(|n| format!("preview-{n}\n")).collect::<String>()}) },
        }));
        let output: String = state.take_output().into_iter().map(|p| p.text).collect();
        assert_eq!(output.contains("preview-0"), expanded);
        assert!(!output.contains("preview-12"));
        assert_eq!(output.contains("additional output omitted"), expanded);
    }
}

#[test]
fn composer_background_starts_at_two_rows_and_grows() {
    use ratatui::style::Color;
    let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    let mut editor = Editor::new(None, "test");
    for (input, expected) in [("", 2), ("first\nsecond", 2), (&"a".repeat(80), 3)] {
        editor.textarea.select_all();
        editor.textarea.insert_str(input);
        terminal
            .draw(|f| widgets::composer(f, &mut editor, "test", "", true))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let rows = (0..12)
            .filter(|y| buffer[(0, *y)].bg == Color::Indexed(235))
            .count();
        assert_eq!(rows, expected);
    }
}

#[test]
fn unbound_theme_recolors_transcript_prompt_and_mode_label() {
    use ratatui::style::Color;
    let red = Color::Rgb(249, 68, 71);
    let mut state = Presentation::new("test".into());
    state.mode = "unbound".into();
    state.apply(&event(EventKind::ModelTextDelta {
        text: "reply".into(),
    }));
    assert_eq!(state.take_output()[0].ink, Ink::Unbound);
    state.mode = "auto".into();
    state.apply(&event(EventKind::ModelTextDelta {
        text: "reply".into(),
    }));
    assert_eq!(state.take_output()[0].ink, Ink::Reply);
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    let mut editor = Editor::new(None, "test");
    terminal
        .draw(|f| widgets::composer(f, &mut editor, "unbound · session · test", "", true))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 8)].fg, red); // prompt chevron
    for x in 0..7 {
        assert_eq!(buffer[(x, 11)].fg, red);
    }
    terminal
        .draw(|f| widgets::status(f, "unbound · running", "", true, false, true))
        .unwrap();
    let buffer = terminal.backend().buffer();
    for x in 0..7 {
        assert_eq!(buffer[(x, 10)].fg, red);
    }
    assert_eq!(buffer[(0, 11)].fg, red);
}

#[test]
fn file_mentions_complete_at_the_cursor_without_submitting_or_losing_text() {
    let mut editor = Editor::new(None, "test");
    editor.handle(Event::Paste("inspect 界 @doc then fix".into()));
    editor
        .textarea
        .move_cursor(ratatui_textarea::CursorMove::Jump(0, 14));
    assert_eq!(editor.mention().unwrap().2, "doc");
    editor.file_options = vec![("docs/architecture.md".into(), String::new())];
    assert_eq!(editor.handle(key(KeyCode::Enter)), Action::Continue);
    assert_eq!(editor.text(), "inspect 界 @docs/architecture.md  then fix");
    assert!(editor.mention().is_none());
    assert_eq!(
        editor.handle(key(KeyCode::Enter)),
        Action::Submit(editor.text())
    );
    let mut editor = Editor::new(None, "test");
    editor.handle(Event::Paste("a@doc".into()));
    assert!(editor.mention().is_none());
}

#[test]
fn file_mention_navigation_and_tab_keep_multiline_input() {
    let mut editor = Editor::new(None, "test");
    editor.handle(Event::Paste("first\n@doc".into()));
    editor.file_options = vec![
        ("docs/a.md".into(), String::new()),
        ("docs/b.md".into(), String::new()),
    ];
    editor.handle(key(KeyCode::Down));
    assert_eq!(editor.handle(key(KeyCode::Tab)), Action::Continue);
    assert_eq!(editor.text(), "first\n@docs/b.md ");
}

#[test]
fn targeted_inspection_previews_show_line_numbers_and_limits() {
    let mut state = Presentation::new("test".into());
    state.expanded_tool_calls = true;
    state.apply(&event(EventKind::ToolCallCompleted {
        outcome: ToolOutcome::Success {
            data: json!({
                "content":"needle\n", "lines":[{"line":600,"text":"needle"}],
                "truncated":true,"coverage":{"truncation_reason":"output_limit"}
            }),
        },
    }));
    let output: String = state
        .take_output()
        .into_iter()
        .map(|piece| piece.text)
        .collect();
    assert!(output.contains("600: needle"));
    assert!(output.contains("truncated (output_limit)"));
    assert_eq!(
        summary(&ToolOutcome::Success {
            data: json!({
                "matches":[{"context_truncated":true}],"truncated":true,"page":{"next_offset":20}
            })
        }),
        "1 matches · more results, next offset 20 · context truncated"
    );
    assert_eq!(
        summary(&ToolOutcome::Success {
            data: json!({
                "matches":[],"truncated":true,"page":{"next_offset":null}
            })
        }),
        "0 matches · incomplete coverage"
    );
}
