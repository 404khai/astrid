//! Real child-process terminals; no credentials, API calls, or interactive developer tty.
use super::*;
use std::{
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Pty {
    child: Child,
    master: File,
    slave: File,
    original: libc::termios,
    parser: vt100::Parser,
    pub output: Vec<u8>,
    queries: usize,
    record: std::path::PathBuf,
}
impl Pty {
    fn new(scenario: &str, root: &std::path::Path, term: &str) -> Self {
        let mut master = 0;
        let mut slave = 0;
        let mut size = libc::winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut size,
                )
            },
            0
        );
        let master = unsafe { File::from_raw_fd(master) };
        let slave = unsafe { File::from_raw_fd(slave) };
        let mut original = std::mem::MaybeUninit::uninit();
        assert_eq!(
            unsafe { libc::tcgetattr(slave.as_raw_fd(), original.as_mut_ptr()) },
            0
        );
        let original = unsafe { original.assume_init() };
        let flags = unsafe { libc::fcntl(master.as_raw_fd(), libc::F_GETFL) };
        assert_ne!(
            unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) },
            -1
        );
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "console::pty_tests::pty_child", "--nocapture"])
            .env("ASTRID_PTY_SCENARIO", scenario)
            .env("ASTRID_PTY_ROOT", root)
            .env("TERM", term)
            .env("NO_COLOR", "1")
            .env_remove("ASTRID_FIXED_VIEWPORT")
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()));
        if scenario == "redirected_stdout"
            || scenario == "redirected_approval"
            || scenario == "broken_pipe"
        {
            command.stdout(Stdio::piped());
        }
        if scenario.starts_with("redirected_")
            || scenario == "fallback"
            || scenario == "broken_pipe"
        {
            command.env_remove("NO_COLOR");
        }
        if scenario == "conversations" {
            command.env_remove("NO_COLOR");
        }
        if scenario == "redirected_stderr" {
            command.stderr(Stdio::piped());
        }
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().unwrap();
        Self {
            child,
            master,
            slave,
            original,
            parser: vt100::Parser::new(24, 80, 2000),
            output: Vec::new(),
            queries: 0,
            record: root.join("termios.json"),
        }
    }
    fn pump(&mut self) {
        let mut buf = [0u8; 8192];
        while let Ok(n) = self.master.read(&mut buf) {
            if n == 0 {
                break;
            }
            self.parser.process(&buf[..n]);
            self.output.extend_from_slice(&buf[..n]);
        }
        let count = self.output.windows(4).filter(|b| *b == b"\x1b[6n").count();
        for _ in self.queries..count {
            let (row, col) = self.parser.screen().cursor_position();
            self.master
                .write_all(format!("\x1b[{};{}R", row + 1, col + 1).as_bytes())
                .unwrap();
        }
        self.queries = count;
    }
    fn wait_for(&mut self, needle: &str) {
        let start = Instant::now();
        loop {
            self.pump();
            if String::from_utf8_lossy(&self.output).contains(needle)
                || self.parser.screen().contents().contains(needle)
            {
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(8),
                "did not see {needle:?}; screen: {}\noutput: {}",
                self.parser.screen().contents(),
                String::from_utf8_lossy(&self.output)
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn wait_for_screen(&mut self, needle: &str) {
        let start = Instant::now();
        loop {
            self.pump();
            if self.parser.screen().contents().contains(needle) {
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(8),
                "did not see current screen {needle:?}: {}",
                self.parser.screen().contents()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn send(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).unwrap();
    }
    fn resize(&mut self, w: u16, h: u16) {
        let size = libc::winsize {
            ws_row: h,
            ws_col: w,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        assert_eq!(
            unsafe { libc::ioctl(self.slave.as_raw_fd(), libc::TIOCSWINSZ, &size) },
            0
        );
        self.parser.screen_mut().set_size(h, w);
    }
    fn finish(&mut self) -> std::process::ExitStatus {
        let start = Instant::now();
        let status = loop {
            self.pump();
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(
                start.elapsed() < Duration::from_secs(8),
                "child hung: {}",
                String::from_utf8_lossy(&self.output)
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        self.pump();
        for mut pipe in [
            self.child
                .stdout
                .take()
                .map(|p| Box::new(p) as Box<dyn Read>),
            self.child
                .stderr
                .take()
                .map(|p| Box::new(p) as Box<dyn Read>),
        ]
        .into_iter()
        .flatten()
        {
            pipe.read_to_end(&mut self.output).unwrap();
        }
        // macOS invalidates slave ioctls when its controlling session exits.
        // Read the child's final checkpoint, taken after UI guards unwind and
        // before exiting, rather than mistaking ENXIO for a cleanup failure.
        let after: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&self.record).expect("child termios checkpoint"))
                .unwrap();
        // PENDIN is kernel-managed pending-input bookkeeping, not a mode leak.
        assert_eq!(
            after["local"],
            serde_json::json!(self.original.c_lflag & !libc::PENDIN),
            "terminal local flags leaked"
        );
        assert_eq!(
            after["input"],
            serde_json::json!(self.original.c_iflag),
            "terminal input flags leaked"
        );
        assert_eq!(
            after["output"],
            serde_json::json!(self.original.c_oflag),
            "terminal output flags leaked"
        );
        assert_eq!(
            after["control"],
            serde_json::json!(self.original.c_cc),
            "terminal control characters leaked"
        );
        assert!(!self.parser.screen().alternate_screen());
        assert!(!self.parser.screen().hide_cursor());
        status
    }
}
impl Drop for Pty {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn pty_child() {
    let Ok(scenario) = std::env::var("ASTRID_PTY_SCENARIO") else {
        return;
    };
    if scenario == "absent_tty" {
        assert!(
            !tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(crate::terminal_answer())
                .unwrap()
        );
        return;
    }
    let root = std::path::PathBuf::from(std::env::var_os("ASTRID_PTY_ROOT").unwrap());
    struct Checkpoint(std::path::PathBuf);
    impl Drop for Checkpoint {
        fn drop(&mut self) {
            let mut state = std::mem::MaybeUninit::uninit();
            if unsafe { libc::tcgetattr(0, state.as_mut_ptr()) } == 0 {
                let state = unsafe { state.assume_init() };
                let _=std::fs::write(&self.0,serde_json::json!({"local":state.c_lflag & !libc::PENDIN,"input":state.c_iflag,"output":state.c_oflag,"control":state.c_cc}).to_string());
            }
        }
    }
    let _checkpoint = Checkpoint(root.join("termios.json"));
    if scenario == "conversations" {
        use astrid::model::{ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink};
        struct NoAuth;
        #[async_trait::async_trait]
        impl astrid::auth::Authentication for NoAuth {
            async fn bearer_token(
                &self,
            ) -> Result<astrid::auth::BearerToken, astrid::auth::AuthError> {
                panic!("PTY conversation fixture must not authenticate")
            }
        }
        struct Provider;
        #[async_trait::async_trait]
        impl ModelProvider for Provider {
            fn measure_request(
                &self,
                request: &ModelRequest<'_>,
            ) -> Result<Option<astrid::context::ContextSnapshot>, ModelError> {
                astrid::openai::OpenAiProvider::new(std::sync::Arc::new(NoAuth))
                    .unwrap()
                    .measure_request(request)
            }
            async fn generate(
                &self,
                request: &ModelRequest<'_>,
                sink: &mut dyn TextSink,
            ) -> Result<ModelResponse, ModelError> {
                let users = request
                    .messages
                    .iter()
                    .filter(|m| matches!(m, astrid::model::Message::User(_)))
                    .count();
                let task = request
                    .messages
                    .iter()
                    .rev()
                    .find_map(|message| match message {
                        astrid::model::Message::User(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .unwrap();
                let text = format!("REPLY:{task}:COUNT:{users}");
                sink.delta(&text).await?;
                astrid::openai::completed_response(
                    serde_json::json!({"status":"completed", "output":[{"type":"message", "role":"assistant", "content":[{"type":"output_text", "text":text}]}]}),
                )
            }
        }
        let tools = astrid::tools::Tools::new(
            astrid::workspace::Workspace::new(&root).unwrap(),
            Duration::from_secs(5),
        )
        .unwrap();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(crate::conversation_loop(
                &Provider,
                &tools,
                "fixture".into(),
                &root.join("model-settings"),
            ))
            .unwrap();
        assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
        return;
    }
    let id = Identity {
        mode: "auto".into(),
        model: "fixture".into(),
        provider: "fixture".into(),
        cwd: root.display().to_string(),
        instructions: None,
        tools: vec!["shell".into()],
    };
    if scenario == "composer" || scenario == "composer_cancel" {
        welcome(&id).unwrap();
        let result = Composer::default().compose("fixture");
        if scenario == "composer_cancel" {
            assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Interrupted);
        } else {
            assert_eq!(result.unwrap(), "first\n界e\u{301}");
        }
        assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
        return;
    }
    if scenario == "panic" {
        let _console = Console::new(id, true).unwrap();
        panic!("intentional terminal panic fixture");
    }
    if scenario == "setup_failure" {
        // A failing backend setup occurs after modes were enabled. Unwinding the
        // constructor must restore them even without an established viewport.
        let _result = (|| -> std::io::Result<()> {
            let _guard = terminal::TerminalGuard::enter()?;
            Err(std::io::Error::other("injected backend setup failure"))
        })();
        assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
        return;
    }
    if scenario == "fallback" || scenario == "redirected_stdout" || scenario == "redirected_stderr"
    {
        let mut console = Console::new(id, true).unwrap();
        assert!(!console.inline());
        console
            .render(&tests::event(astrid::events::EventKind::ModelTextDelta {
                text: "plain output".into(),
            }))
            .unwrap();
        console.finish(1, 0).unwrap();
        return;
    }
    // Use the same CLI drive loop with a deterministic provider and real Tools.
    use astrid::model::{ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink};
    struct Provider {
        scenario: String,
    }
    #[async_trait::async_trait]
    impl ModelProvider for Provider {
        async fn generate(
            &self,
            request: &ModelRequest<'_>,
            sink: &mut dyn TextSink,
        ) -> Result<ModelResponse, ModelError> {
            if self.scenario == "stream_cancel" {
                loop {
                    sink.delta("streaming tail 界 e\u{301} ").await?;
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
            let output = if request.messages.len() == 1 {
                let command = if self.scenario == "shell_cancel" {
                    "printf '%s' $$ > shell.pid; printf 'shell active\\n'; exec sleep 30"
                } else {
                    "printf 'tool stdout\\n'; printf 'tool stderr\\n' >&2"
                };
                vec![
                    serde_json::json!({"type":"function_call","call_id":"fixture-call","name":"shell","arguments":serde_json::json!({"command":command}).to_string()}),
                ]
            } else {
                sink.delta("final fixture response").await?;
                vec![
                    serde_json::json!({"type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"final fixture response"}]}),
                ]
            };
            astrid::openai::completed_response(
                serde_json::json!({"status":"completed","output":output}),
            )
        }
    }
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let mut console = Console::new(id, true).unwrap();
        let cancel = astrid::cancellation::Cancellation::default();
        let tools = astrid::tools::Tools::new(
            astrid::workspace::Workspace::new(&root).unwrap(),
            Duration::from_secs(30),
        )
        .unwrap();
        let provider = Provider {
            scenario: scenario.clone(),
        };
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let (permission_tx, permission_rx) = tokio::sync::mpsc::channel(1);
        let mut permissions = crate::TerminalPermission {
            requests: permission_tx,
        };
        let run = astrid::runtime::run(
            &provider,
            &tools,
            &mut permissions,
            astrid::runtime::RunConfig {
                observability: None,
                model: "fixture".into(),
                task: "PTY fixture".into(),
                max_model_calls: 3,
                context_budget: None,
                permissions: astrid::permissions::PermissionPolicy::default(),
            },
            cancel.clone(),
            Some(tx),
        );
        let result = crate::drive_run(run, cancel, rx, permission_rx, &mut console).await;
        if scenario == "broken_pipe" {
            assert_eq!(
                result
                    .unwrap_err()
                    .downcast_ref::<std::io::Error>()
                    .unwrap()
                    .kind(),
                std::io::ErrorKind::BrokenPipe
            );
            return;
        }
        let result = result.unwrap();
        match scenario.as_str() {
            "stream_cancel" | "permission_cancel" | "shell_cancel" => {
                assert_eq!(result.outcome, astrid::runtime::RunOutcome::Cancelled)
            }
            _ => assert_eq!(result.outcome, astrid::runtime::RunOutcome::Completed),
        }
        if scenario == "denied" || scenario == "permission_cancel" {
            assert!(!root.join("shell.pid").exists());
        }
        assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
        eprintln!("CHILD_RESULT {:?}", result.outcome);
    });
    if scenario == "broken_pipe" {
        // The app's EPIPE was asserted above. Avoid libtest writing its own
        // summary into the deliberately closed pipe after that assertion.
        drop(_checkpoint);
        std::process::exit(0);
    }
}

#[test]
fn pty_composer_multiline_resize_and_ctrl_c_restore_terminal() {
    for scenario in ["composer", "composer_cancel"] {
        let root = tempfile::tempdir().unwrap();
        let mut pty = Pty::new(scenario, root.path(), "xterm-256color");
        pty.wait_for("Enter sends");
        pty.resize(24, 12);
        if scenario == "composer" {
            pty.send("first\n界e\u{301}\r".as_bytes());
        } else {
            pty.send(&[3]);
        }
        assert!(
            pty.finish().success(),
            "{}",
            String::from_utf8_lossy(&pty.output)
        );
        assert!(!pty.output.windows(8).any(|b| b == b"\x1b[?1049h"));
    }
}

#[test]
fn pty_conversation_continues_switches_modes_and_returns_to_selected_session() {
    let root = tempfile::tempdir().unwrap();
    let mut pty = Pty::new("conversations", root.path(), "xterm-256color");
    pty.wait_for_screen("Message Astrid");
    for (mode, notice, expected) in [
        (
            "unbound",
            "full tool access",
            vt100::Color::Rgb(236, 26, 29),
        ),
        ("ask", "reads allowed", vt100::Color::Rgb(26, 50, 236)),
        (
            "auto",
            "workspace edits allowed",
            vt100::Color::Rgb(26, 50, 236),
        ),
    ] {
        pty.send(format!("/mode {mode}\r").as_bytes());
        pty.wait_for_screen(notice);
        let screen = pty.parser.screen();
        assert_eq!(screen.contents().matches("astrid  0.1.0").count(), 1);
        assert!(
            (0..24).any(|row| (0..30).any(|col| {
                screen
                    .cell(row, col)
                    .is_some_and(|cell| cell.contents() == "█" && cell.fgcolor() == expected)
            })),
            "logo did not recolor for {mode}"
        );
    }
    pty.send(b"first\r");
    pty.wait_for("REPLY:first:COUNT:1");
    // The prompt must be back before supplying each new task/command.
    pty.wait_for_screen("Message Astrid");
    pty.send(b"second\r");
    pty.wait_for("REPLY:second:COUNT:2");
    pty.wait_for_screen("Message Astrid");
    pty.send(b"/mode unbound\r");
    pty.wait_for("full tool access");
    pty.send(b"/new\r");
    pty.wait_for("New session");
    pty.send(b"other\r");
    pty.wait_for("REPLY:other:COUNT:1");
    pty.wait_for_screen("Message Astrid");
    pty.send(b"/sessions\r");
    pty.wait_for("Sessions — select");
    pty.send(b"\x1b[A\r");
    pty.wait_for_screen("Message Astrid");
    pty.send(b"third\r");
    pty.wait_for("REPLY:third:COUNT:3");
    pty.wait_for_screen("Message Astrid");
    pty.send(b"/quit\r");
    assert!(
        pty.finish().success(),
        "{}",
        String::from_utf8_lossy(&pty.output)
    );
    assert!(String::from_utf8_lossy(&pty.output).contains("unbound"));
    assert!(!pty.output.windows(8).any(|b| b == b"\x1b[?1049h"));
}
#[test]
fn pty_stream_cancel_and_permission_cancel_are_responsive() {
    for (scenario, needle) in [
        ("stream_cancel", "streaming tail"),
        ("permission_cancel", "type yes to approve"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut pty = Pty::new(scenario, root.path(), "xterm-256color");
        pty.wait_for(needle);
        pty.resize(24, 12);
        let start = Instant::now();
        pty.send(&[3]);
        assert!(
            pty.finish().success(),
            "{}",
            String::from_utf8_lossy(&pty.output)
        );
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "raw Ctrl-C wasn't responsive"
        );
    }
}
#[test]
fn pty_approvals_deny_paste_and_cancel_live_shell() {
    for scenario in ["approved", "denied", "shell_cancel"] {
        let root = tempfile::tempdir().unwrap();
        let mut pty = Pty::new(scenario, root.path(), "xterm-256color");
        pty.wait_for("type yes to approve");
        // Wait for the CLI to flush and drain pre-prompt typeahead before sending.
        std::thread::sleep(Duration::from_millis(60));
        pty.pump();
        if scenario == "denied" {
            pty.send(b"\x1b[200~yes\n\x1b[201~\r");
        } else {
            pty.send(b"YeS\r");
        }
        if scenario == "shell_cancel" {
            let start = Instant::now();
            while !root.path().join("shell.pid").exists() {
                pty.pump();
                assert!(
                    start.elapsed() < Duration::from_secs(8),
                    "shell did not start"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            pty.send(&[3]);
        }
        assert!(
            pty.finish().success(),
            "{}",
            String::from_utf8_lossy(&pty.output)
        );
        if scenario == "shell_cancel" {
            let pid = std::fs::read_to_string(root.path().join("shell.pid"))
                .unwrap()
                .parse::<i32>()
                .unwrap();
            assert_eq!(
                unsafe { libc::kill(pid, 0) },
                -1,
                "shell leader survived cancellation"
            );
        } else if scenario == "approved" {
            let output = String::from_utf8_lossy(&pty.output);
            assert!(output.contains("tool stdout"));
            assert!(output.contains("tool stderr"));
            assert!(output.contains("final fixture response"));
        } else {
            assert!(String::from_utf8_lossy(&pty.output).contains("permission   denied"));
        }
    }
}
#[test]
fn pty_panic_setup_failure_and_dumb_fallback_restore_terminal() {
    for scenario in ["panic", "setup_failure", "fallback"] {
        let root = tempfile::tempdir().unwrap();
        let mut pty = Pty::new(
            scenario,
            root.path(),
            if scenario == "fallback"
                || scenario == "redirected_stdout"
                || scenario == "redirected_stderr"
            {
                "dumb"
            } else {
                "xterm-256color"
            },
        );
        let status = pty.finish();
        assert_eq!(
            status.success(),
            scenario != "panic",
            "{}",
            String::from_utf8_lossy(&pty.output)
        );
        if scenario == "fallback"
            || scenario == "redirected_stdout"
            || scenario == "redirected_stderr"
        {
            assert!(!pty.output.contains(&27));
        }
    }
}

#[test]
fn pty_redirected_streams_keep_plain_output() {
    for scenario in ["redirected_stdout", "redirected_stderr"] {
        let root = tempfile::tempdir().unwrap();
        let mut pty = Pty::new(scenario, root.path(), "xterm-256color");
        assert!(
            pty.finish().success(),
            "{}",
            String::from_utf8_lossy(&pty.output)
        );
        assert!(!pty.output.contains(&27));
        assert!(String::from_utf8_lossy(&pty.output).contains("plain output"));
    }
}

#[test]
fn pty_piped_output_preserves_canonical_approval_and_broken_pipe_exit() {
    for scenario in ["redirected_approval", "broken_pipe"] {
        let root = tempfile::tempdir().unwrap();
        let mut pty = Pty::new(scenario, root.path(), "xterm-256color");
        pty.wait_for("type yes to approve");
        assert!(
            String::from_utf8_lossy(&pty.output).contains("Runs with your account's permissions.")
        );
        if scenario == "broken_pipe" {
            drop(pty.child.stdout.take());
        }
        pty.send(b"yes\n");
        assert!(
            pty.finish().success(),
            "{}",
            String::from_utf8_lossy(&pty.output)
        );
        assert!(!pty.output.contains(&27));
    }
}

#[test]
fn detached_client_without_tty_denies_approval() {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "console::pty_tests::pty_child", "--nocapture"])
        .env("ASTRID_PTY_SCENARIO", "absent_tty")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pty_file_mentions_lookup_and_insert_without_dispatching() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("docs")).unwrap();
    std::fs::write(root.path().join("docs/architecture.md"), "").unwrap();
    let mut pty = Pty::new("conversations", root.path(), "xterm-256color");
    pty.wait_for_screen("Message Astrid");
    pty.send(b"@doc");
    pty.wait_for_screen("docs/architecture.md");
    pty.send(b"\t");
    pty.wait_for_screen("@docs/architecture.md");
    assert!(!String::from_utf8_lossy(&pty.output).contains("REPLY:"));
    pty.send(b"\r");
    pty.wait_for("REPLY:@docs/architecture.md");
    pty.wait_for_screen("Message Astrid");
    pty.send(b"/quit\r");
    pty.finish();
}

#[test]
fn pty_sessions_survive_restart_with_first_message_names_and_active_selection() {
    let root = tempfile::tempdir().unwrap();
    let mut first = Pty::new("conversations", root.path(), "xterm-256color");
    first.wait_for_screen("Message Astrid");
    first.send(b"first session name\r");
    first.wait_for("REPLY:first session name:COUNT:1");
    first.wait_for_screen("Message Astrid");
    first.send(b"/new\r");
    first.wait_for_screen("New session");
    first.send(b"second session name\r");
    first.wait_for("REPLY:second session name:COUNT:1");
    first.wait_for_screen("Message Astrid");
    first.send(b"/quit\r");
    assert!(first.finish().success());
    let mut second = Pty::new("conversations", root.path(), "xterm-256color");
    second.wait_for_screen("Message Astrid");
    second.send(b"follow up after restart\r");
    second.wait_for("REPLY:follow up after restart:COUNT:2");
    second.wait_for_screen("Message Astrid");
    second.send(b"/sessions\r");
    second.wait_for_screen("first session name");
    second.wait_for_screen("second session name");
    assert!(!second.parser.screen().contents().contains("in memory"));
    second.send(b"\x1b[A\r");
    second.wait_for_screen("saved locally");
    second.send(b"resume the first session\r");
    second.wait_for("REPLY:resume the first session:COUNT:2");
    second.wait_for_screen("Message Astrid");
    second.send(b"/quit\r");
    assert!(second.finish().success());
}
