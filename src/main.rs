use astrid::{
    auth::{self, Authentication, ChatGptAuth},
    cancellation::Cancellation,
    events::EventKind,
    openai::OpenAiProvider,
    permissions::{PermissionAction, PermissionMode, PermissionPolicy},
    runtime::{self, PermissionHandler, RunConfig, RunOutcome},
    tools::{PermissionRequest, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use std::{
    fs::OpenOptions, io, io::Write, os::fd::AsRawFd, os::unix::fs::OpenOptionsExt,
    process::ExitCode, sync::Arc, time::Duration,
};

mod console;
mod logo;
mod sessions;
use console::Console;
use logo::LOGO;

#[derive(Parser)]
#[command(name="astrid", version, about="One repository task, one observable model/tool loop", after_help=LOGO)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Exit interactive startup.
    #[command(hide = true)]
    Exit,
    /// Sign in with ChatGPT using Astrid's own registration and credentials.
    Login,
    /// List models available to the signed-in ChatGPT account.
    Models,
    /// Execute one fresh repository task in the invocation directory.
    Run {
        task: String,
        #[arg(
            long,
            env = "ASTRID_MODEL",
            help = "OpenAI model identifier (required; no hard-coded default)"
        )]
        model: String,
        #[arg(long,env="ASTRID_MAX_MODEL_CALLS",default_value="20",value_parser=positive)]
        max_model_calls: usize,
        #[arg(long,env="ASTRID_SHELL_TIMEOUT",default_value="30",value_parser=positive,help="Command timeout in seconds")]
        shell_timeout: usize,
        #[arg(long, value_enum, default_value = "auto")]
        mode: PermissionMode,
        #[arg(long, value_enum)]
        read_policy: Option<PermissionAction>,
        #[arg(long, value_enum)]
        write_policy: Option<PermissionAction>,
        #[arg(
            long,
            value_enum,
            help = "Shell authority: allow grants broad account-level execution"
        )]
        shell_policy: Option<PermissionAction>,
        #[arg(long, help = "Show prepared request sizes and token-count uncertainty")]
        show_context: bool,
        #[arg(long, default_value = "32768", value_parser = positive, help = "Estimated context allowance; not the model's known capacity")]
        context_tokens: usize,
        #[arg(
            long,
            default_value = "4096",
            help = "Planning reserve; does not enforce provider output length"
        )]
        response_reserve: usize,
        #[arg(long, default_value = "524288", value_parser = positive)]
        context_bytes: usize,
        #[arg(long, default_value = "4096", value_parser = positive)]
        summary_bytes: usize,
        #[arg(long, value_enum, default_value = "file-references")]
        context_policy: astrid::context::SelectionPolicy,
    },
}

fn positive(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(|| "value must be a positive integer".into())
}
fn configured_limit(name: &str, fallback: usize) -> Result<usize, Box<dyn std::error::Error>> {
    match std::env::var(name) {
        Ok(value) => positive(&value).map_err(|message| format!("{name}: {message}").into()),
        Err(std::env::VarError::NotPresent) => Ok(fallback),
        Err(error) => Err(error.into()),
    }
}

use console::printable;

type DecisionRequest = (
    PermissionRequest,
    tokio::sync::oneshot::Sender<io::Result<bool>>,
);
// Dropping a CLI future must not leave an input task behind.
struct InputTask(tokio::task::JoinHandle<()>);
impl Drop for InputTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
struct TerminalPermission {
    requests: tokio::sync::mpsc::Sender<DecisionRequest>,
}
#[async_trait]
impl PermissionHandler for TerminalPermission {
    async fn decide(&mut self, request: &PermissionRequest) -> io::Result<bool> {
        let (reply, decision) = tokio::sync::oneshot::channel();
        self.requests
            .send((request.clone(), reply))
            .await
            .map_err(|_| io::Error::other("permission interface closed"))?;
        decision
            .await
            .map_err(|_| io::Error::other("permission interface closed"))?
    }
}

async fn terminal_answer() -> io::Result<bool> {
    let Ok(tty) = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")
    else {
        return Ok(false);
    };
    read_permission_answer(tty).await
}

async fn read_permission_answer(tty: std::fs::File) -> io::Result<bool> {
    let mut answer = Vec::new();
    loop {
        let mut bytes = [0u8; 256];
        let count = unsafe { libc::read(tty.as_raw_fd(), bytes.as_mut_ptr().cast(), bytes.len()) };
        if count < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock || err.kind() == io::ErrorKind::Interrupted {
                // /dev/tty is not kqueue-registerable on all supported Macs.
                // Nonblocking reads plus a cancellable timer leave no blocked thread.
                tokio::time::sleep(Duration::from_millis(25)).await;
                continue;
            }
            return Err(err);
        }
        if count == 0 {
            return Ok(false);
        }
        answer.extend_from_slice(&bytes[..count as usize]);
        if answer.contains(&b'\n') {
            return Ok(String::from_utf8_lossy(&answer)
                .trim()
                .eq_ignore_ascii_case("yes"));
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match execute(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("astrid: {}", printable(&error.to_string()));
            ExitCode::FAILURE
        }
    }
}

fn remember_model(model: &str) -> io::Result<()> {
    remember_model_at(&auth::default_directory().map_err(io::Error::other)?, model)
}
fn remember_model_at(directory: &std::path::Path, model: &str) -> io::Result<()> {
    std::fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    writeln!(file, "{model}")?;
    file.persist(directory.join("last-model"))
        .map_err(|e| e.error)?;
    Ok(())
}

async fn model_catalog() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let auth = ChatGptAuth::new(auth::default_directory()?)?;
    let token = auth.bearer_token().await?;
    let catalog: serde_json::Value = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get("https://api.openai.com/v1/models")
        .bearer_auth(token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let entries = catalog["models"]
        .as_array()
        .ok_or("invalid model catalog")?;
    let models: Vec<String> = entries
        .iter()
        .filter(|v| v["visibility"] == "list")
        .filter_map(|v| v["slug"].as_str().map(str::to_owned))
        .collect();
    if models.is_empty() {
        return Err("no models available to this account".into());
    }
    Ok(models)
}

fn startup_identity(
    model: &str,
    workspace: &Workspace,
    policy: PermissionPolicy,
) -> console::Identity {
    let cwd = workspace.root();
    let cwd = std::env::var_os("HOME")
        .and_then(|home| cwd.strip_prefix(home).ok())
        .map_or_else(
            || cwd.display().to_string(),
            |path| format!("~/{}", path.display()),
        );
    console::Identity {
        mode: PermissionMode::from_policy(policy)
            .map_or_else(|| "custom".into(), |mode| mode.to_string()),
        model: printable(model),
        provider: "OpenAI / Codex subscription".into(),
        cwd,
        instructions: workspace
            .instructions()
            .ok()
            .flatten()
            .map(|_| "AGENTS.md".into()),
        tools: astrid::tools::definitions()
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
            .collect(),
    }
}

async fn interactive_loop() -> Result<(), Box<dyn std::error::Error>> {
    if !console::interactive_available() {
        return Err(
            "interactive startup requires a terminal; use astrid run \"task\" --model <model>"
                .into(),
        );
    }
    let saved = std::fs::read_to_string(auth::default_directory()?.join("last-model")).ok();
    let mut model = std::env::var("ASTRID_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| saved.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()));
    if model.is_none() {
        model = Some(model_catalog().await?.remove(0));
    }
    let model = model.ok_or("no model selected")?;
    let workspace = Workspace::new(std::env::current_dir()?)?;
    let directory = auth::default_directory()?;
    if directory.starts_with(workspace.root()) {
        return Err(
            "choose a repository workspace outside Astrid's credential directory ancestors".into(),
        );
    }
    let timeout = configured_limit("ASTRID_SHELL_TIMEOUT", 30)?;
    let tools = Tools::new(workspace, Duration::from_secs(timeout as u64))?;
    let provider = OpenAiProvider::new(Arc::new(ChatGptAuth::new(directory.clone())?))?;
    conversation_loop(&provider, &tools, model, &directory).await
}

async fn conversation_loop(
    provider: &dyn astrid::model::ModelProvider,
    tools: &Tools,
    mut model: String,
    model_directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let max_model_calls = configured_limit("ASTRID_MAX_MODEL_CALLS", 20)?;
    let mut sessions = sessions::Sessions::new(tools.workspace());
    let mut composer = console::Composer::default();
    console::welcome(&startup_identity(
        &model,
        tools.workspace(),
        composer.mode.policy(),
    ))?;
    loop {
        composer.session_label = sessions.active_label();
        let task = composer.compose(&model)?;
        match task.trim() {
            "" => continue,
            "/" | "/help" => {
                composer.notice = "/mode · /sessions · /new · /model · /quit".into();
                continue;
            }
            "/quit" | "/exit" => return Ok(()),
            "/model" => {
                let models = model_catalog().await?;
                if let Some(next) = composer.select_model(&models, &model)? {
                    model = next;
                    remember_model_at(model_directory, &model)?;
                }
                continue;
            }
            "/new" => {
                composer.notice = sessions
                    .create(tools.workspace())
                    .err()
                    .unwrap_or_else(|| "New session. Sessions last until Astrid exits.".into());
                continue;
            }
            "/sessions" => {
                let options = sessions.choices();
                let current = options[sessions.active_index()].clone();
                if let Some(choice) = composer.choose(
                    "Sessions — select to continue (in memory)",
                    &options,
                    &current,
                )? {
                    sessions.select(&choice);
                }
                composer.notice = "Sessions last until Astrid exits.".into();
                continue;
            }
            command if command == "/mode" || command.starts_with("/mode ") => {
                let chosen = if command == "/mode" {
                    composer.choose(
                        "Permission mode — ask / auto / unbound",
                        &["ask".into(), "auto".into(), "unbound".into()],
                        &composer.mode.to_string(),
                    )?
                } else {
                    Some(command[6..].trim().to_owned())
                };
                if let Some(chosen) = chosen {
                    match <PermissionMode as clap::ValueEnum>::from_str(&chosen, false) {
                        Ok(mode) => {
                            composer.mode = mode;
                            composer.notice = match mode {
                                PermissionMode::Ask => {
                                    "Ask: reads allowed; edits and shell need approval."
                                }
                                PermissionMode::Auto => {
                                    "Auto: workspace edits allowed; shell needs approval."
                                }
                                PermissionMode::Unbound => {
                                    "Unbound: full tool access; shell runs with account authority."
                                }
                            }
                            .into();
                            console::welcome(&startup_identity(
                                &model,
                                tools.workspace(),
                                mode.policy(),
                            ))?;
                        }
                        Err(_) => {
                            composer.notice = "Use /mode ask, /mode auto, or /mode unbound.".into()
                        }
                    }
                }
                continue;
            }
            command if command.starts_with('/') => {
                composer.notice = "Unknown command. Type / for commands.".into();
                continue;
            }
            _ => {}
        }
        remember_model_at(model_directory, &model)?;
        composer.notice.clear();
        let mut console = Console::new(
            startup_identity(&model, tools.workspace(), composer.mode.policy()),
            false,
        )?;
        let cancel = Cancellation::default();
        let (sender, receiver) = tokio::sync::mpsc::channel(64);
        let (permission_sender, permission_receiver) = tokio::sync::mpsc::channel(1);
        let mut permissions = TerminalPermission {
            requests: permission_sender,
        };
        let session = sessions.take_active();
        let result = drive_run(
            runtime::run_in_session(
                provider,
                tools,
                &mut permissions,
                RunConfig {
                    model: model.clone(),
                    task,
                    max_model_calls,
                    permissions: composer.mode.policy(),
                    context_budget: Some(Default::default()),
                },
                cancel.clone(),
                Some(sender),
                session,
            ),
            cancel,
            receiver,
            permission_receiver,
            &mut console,
        )
        .await;
        drop(console);
        match result {
            Ok(result) => sessions.complete(result),
            Err(error) => match error.downcast::<runtime::SessionStartError>() {
                Ok(error) => {
                    eprintln!(
                        "astrid: {}. Session retained; use /new for a fresh session.",
                        printable(&error.message)
                    );
                    composer.notice = "Cannot continue this session; use /new.".into();
                    sessions.restore(*error.session);
                }
                Err(error) => return Err(error),
            },
        }
    }
}

async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let directory = auth::default_directory()?;
    let command = match cli.command {
        Some(command) => command,
        None => return interactive_loop().await,
    };
    match command {
        Commands::Exit => return Ok(()),
        Commands::Login => {
            auth::login(&directory,|url| {
                eprintln!("Continue with ChatGPT: open this URL in your browser (expires in 5 minutes):\n{url}"); Ok(())
            }).await?;
            eprintln!(
                "ChatGPT sign-in validated. Astrid credentials saved with owner-only permissions."
            );
        }
        Commands::Models => {
            let auth = ChatGptAuth::new(directory)?;
            let token = auth.bearer_token().await?;
            let response = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .build()?
                .get("https://api.openai.com/v1/models")
                .bearer_auth(token.as_str())
                .send()
                .await?
                .error_for_status()?;
            let catalog: serde_json::Value = response.json().await?;
            let models = catalog["models"]
                .as_array()
                .ok_or("provider did not return a subscription model catalog")?;
            for model in models.iter().filter(|v| v["visibility"] == "list") {
                println!(
                    "{}\t{}",
                    printable(model["slug"].as_str().ok_or("model has no slug")?),
                    printable(model["display_name"].as_str().unwrap_or(""))
                );
            }
        }
        Commands::Run {
            task,
            model,
            max_model_calls,
            shell_timeout,
            mode,
            read_policy,
            write_policy,
            shell_policy,
            show_context,
            context_tokens,
            response_reserve,
            context_bytes,
            summary_bytes,
            context_policy,
        } => {
            let context_budget = astrid::context::ContextBudget {
                estimated_context_tokens: context_tokens,
                response_reserve_tokens: response_reserve,
                max_request_bytes: context_bytes,
                max_summary_bytes: summary_bytes,
                policy: context_policy,
            };
            if !context_budget.validate() {
                return Err(
                    "context allowance must exceed the response reserve, with positive byte limits"
                        .into(),
                );
            }
            let workspace = Workspace::new(std::env::current_dir()?)?;
            if directory.starts_with(workspace.root()) {
                return Err(
                    "choose a repository workspace outside Astrid's credential directory ancestors"
                        .into(),
                );
            }
            remember_model(&model)?;
            let tools = Tools::new(workspace, Duration::from_secs(shell_timeout as u64))?;
            let provider = OpenAiProvider::new(Arc::new(ChatGptAuth::new(directory)?))?;
            let defaults = mode.policy();
            let policy = PermissionPolicy {
                read: read_policy.unwrap_or(defaults.read),
                write: write_policy.unwrap_or(defaults.write),
                execute: shell_policy.unwrap_or(defaults.execute),
            };
            let mut console =
                Console::new(startup_identity(&model, tools.workspace(), policy), true)?;
            console.show_context = show_context;
            let cancel = Cancellation::default();
            let (sender, receiver) = tokio::sync::mpsc::channel(64);
            let (permission_sender, permission_receiver) =
                tokio::sync::mpsc::channel::<DecisionRequest>(1);
            let mut permissions = TerminalPermission {
                requests: permission_sender,
            };
            let execution = runtime::run(
                &provider,
                &tools,
                &mut permissions,
                RunConfig {
                    context_budget: Some(context_budget),
                    model,
                    task,
                    max_model_calls,
                    permissions: policy,
                },
                cancel.clone(),
                Some(sender),
            );
            let result = drive_run(
                execution,
                cancel,
                receiver,
                permission_receiver,
                &mut console,
            )
            .await?;
            match result.outcome {
                RunOutcome::Completed=>{},
                RunOutcome::Cancelled=>return Err("run cancelled".into()),
                RunOutcome::Failed{code,message}=>return Err(format!("{code}: {message}").into()),
                RunOutcome::ModelCallLimitReached{limit,..}=>return Err(format!("model-call ceiling ({limit}) reached; final tool results were not inspected by the model").into()),
            }
        }
    }
    Ok(())
}

/// CLI orchestration only; reusable with deterministic runtime fixtures.
async fn drive_run<E: std::error::Error + 'static>(
    execution: impl std::future::Future<Output = Result<runtime::RunResult, E>>,
    cancel: Cancellation,
    mut receiver: tokio::sync::mpsc::Receiver<astrid::events::ExecutionEvent>,
    mut permission_receiver: tokio::sync::mpsc::Receiver<DecisionRequest>,
    console: &mut Console,
) -> Result<runtime::RunResult, Box<dyn std::error::Error>> {
    let inline_input = console.inline();
    let (rendered_sender, mut rendered_receiver) = tokio::sync::mpsc::channel::<()>(1);
    let (inline_permission_sender, mut inline_permission_receiver) =
        tokio::sync::mpsc::channel::<DecisionRequest>(1);
    let mut permission_ui = InputTask(tokio::spawn(async move {
        while rendered_receiver.recv().await.is_some() {
            let Some((request, reply)) = permission_receiver.recv().await else {
                break;
            };
            if inline_input {
                if inline_permission_sender
                    .send((request, reply))
                    .await
                    .is_err()
                {
                    break;
                }
            } else {
                let answer = terminal_answer().await;
                let _ = reply.send(answer);
            }
        }
    }));
    tokio::pin!(execution);
    let mut resize = tokio::time::interval(Duration::from_millis(33));
    resize.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut input_tick = tokio::time::interval(Duration::from_millis(10));
    input_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut pending_permission: Option<tokio::sync::oneshot::Sender<io::Result<bool>>> = None;
    let mut permission_armed = false;
    let mut output_error = None;
    let mut attached = true;
    let mut signal_enabled = true;
    let mut input_enabled = inline_input;
    let result = loop {
        tokio::select! {
            result=&mut execution=>break result,
            _=resize.tick(), if attached=>{
                if let Err(err)=console.tick() {
                    output_error=Some(err);cancel.cancel();receiver.close();attached=false;
                }
            }
            request=inline_permission_receiver.recv(), if attached && inline_input && pending_permission.is_none()=>{
                if let Some((_request,reply))=request {
                    pending_permission=Some(reply); permission_armed=false;
                }
            }
            _=input_tick.tick(), if attached && input_enabled=>{
                let action = if pending_permission.is_some() && !permission_armed && !cancel.is_cancelled() {
                    console.arm_permission().map(|(armed,action)| { permission_armed=armed; action })
                } else { console.poll_input() };
                match action {
                    Ok(console::InputAction::Cancel)=>{ cancel.cancel(); input_enabled=false; }
                    Ok(console::InputAction::Approval(answer))=>{
                        if let Some(reply)=pending_permission.take() { let _=reply.send(Ok(answer)); }
                        permission_armed=false;
                    }
                    Ok(console::InputAction::None)=>{},
                    Err(err)=>{ output_error=Some(err); cancel.cancel(); receiver.close(); attached=false; }
                }
            }
            signal=tokio::signal::ctrl_c(), if signal_enabled=>{
                signal_enabled=false;
                if let Err(err)=signal {output_error=Some(err);}
                cancel.cancel(); input_enabled=false;
            }
            event=receiver.recv(), if attached=>match event {
                Some(event)=>{
                    if let Err(err)=console.render(&event) {
                        output_error=Some(err);cancel.cancel();receiver.close();attached=false;
                    } else if matches!(event.kind,EventKind::PermissionRequested{..}) {
                        // Start the UI prompt only after preceding events rendered.
                        let _=rendered_sender.send(()).await;
                    }
                },
                None=>attached=false,
            }
        }
    };
    permission_ui.0.abort();
    let _ = (&mut permission_ui.0).await;
    let result = result?;
    // Runtime completion may win select while final events remain queued.
    if attached {
        while let Ok(event) = receiver.try_recv() {
            if let Err(err) = console.render(&event) {
                output_error = Some(err);
                break;
            }
        }
    }
    if let Some(err) = output_error {
        return Err(err.into());
    }
    console.finish(result.model_calls, result.tool_calls)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::Write,
        os::{fd::OwnedFd, unix::net::UnixStream},
    };
    #[test]
    fn bare_cli_enters_prompt_but_explicit_run_requires_a_model() {
        assert!(Cli::try_parse_from(["astrid"]).unwrap().command.is_none());
        assert!(Cli::try_parse_from(["astrid", "run"]).is_err());
        assert!(matches!(
            Cli::try_parse_from(["astrid", "models"]).unwrap().command,
            Some(Commands::Models)
        ));
    }

    #[test]
    fn context_flags_expose_estimates_and_reject_zero_limits() {
        let parsed = Cli::try_parse_from([
            "astrid",
            "run",
            "task",
            "--model",
            "test",
            "--context-tokens",
            "8192",
            "--response-reserve",
            "1024",
            "--context-bytes",
            "65536",
            "--summary-bytes",
            "512",
            "--context-policy",
            "recency",
            "--show-context",
        ])
        .unwrap();
        assert!(matches!(
            parsed.command,
            Some(Commands::Run {
                context_tokens: 8192,
                response_reserve: 1024,
                context_bytes: 65536,
                summary_bytes: 512,
                context_policy: astrid::context::SelectionPolicy::Recency,
                show_context: true,
                ..
            })
        ));
        assert!(
            Cli::try_parse_from([
                "astrid",
                "run",
                "task",
                "--model",
                "test",
                "--context-bytes",
                "0"
            ])
            .is_err()
        );
    }
    #[test]
    fn cli_permission_policies_are_explicit_and_reject_unenforceable_options() {
        let parsed = Cli::try_parse_from([
            "astrid",
            "run",
            "task",
            "--model",
            "test",
            "--read-policy",
            "deny",
            "--write-policy",
            "ask",
            "--shell-policy",
            "allow",
        ])
        .unwrap();
        assert!(matches!(
            parsed.command,
            Some(Commands::Run {
                read_policy: Some(PermissionAction::Deny),
                write_policy: Some(PermissionAction::Ask),
                shell_policy: Some(PermissionAction::Allow),
                ..
            })
        ));
        assert!(
            Cli::try_parse_from([
                "astrid",
                "run",
                "task",
                "--model",
                "test",
                "--network-policy",
                "deny"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "astrid",
                "run",
                "task",
                "--model",
                "test",
                "--shell-policy",
                "readonly"
            ])
            .is_err()
        );
    }
    fn input() -> (File, UnixStream) {
        let (reader, writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        (File::from(OwnedFd::from(reader)), writer)
    }
    #[tokio::test]
    async fn nonblocking_permission_input_handles_fragmentation_denial_and_eof() {
        for (answer, expected) in [("yes\n", true), ("no\n", false), ("", false)] {
            let (reader, mut writer) = input();
            let supply = async {
                for byte in answer.as_bytes() {
                    writer.write_all(&[*byte]).unwrap();
                    tokio::task::yield_now().await;
                }
                drop(writer);
            };
            let (decision, ()) = tokio::join!(read_permission_answer(reader), supply);
            assert_eq!(decision.unwrap(), expected);
        }
    }
    #[tokio::test]
    async fn nonblocking_permission_input_is_cancellable_without_a_reader_thread() {
        let (reader, _writer) = input();
        let cancel = Cancellation::default();
        let wait = async {
            tokio::select! {answer=read_permission_answer(reader)=>panic!("unexpected decision: {answer:?}"),_=cancel.cancelled()=>{}}
        };
        let stop = async {
            tokio::task::yield_now().await;
            cancel.cancel();
        };
        tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(wait, stop);
        })
        .await
        .unwrap();
    }
}
