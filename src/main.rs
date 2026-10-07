use astrid::{
    auth::{self, Authentication, ChatGptAuth},
    cancellation::Cancellation,
    events::EventKind,
    openai::OpenAiProvider,
    permissions::{PermissionAction, PermissionPolicy},
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
        #[arg(long, value_enum, default_value = "allow")]
        read_policy: PermissionAction,
        #[arg(long, value_enum, default_value = "allow")]
        write_policy: PermissionAction,
        #[arg(
            long,
            value_enum,
            default_value = "ask",
            help = "Shell authority: allow grants broad account-level execution"
        )]
        shell_policy: PermissionAction,
    },
}

fn positive(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(|| "value must be a positive integer".into())
}

use console::printable;

type DecisionRequest = (
    PermissionRequest,
    tokio::sync::oneshot::Sender<io::Result<bool>>,
);
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
    let directory = auth::default_directory().map_err(io::Error::other)?;
    std::fs::create_dir_all(&directory)?;
    let mut file = tempfile::NamedTempFile::new_in(&directory)?;
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

async fn interactive_command() -> Result<Commands, Box<dyn std::error::Error>> {
    use std::io::IsTerminal;
    if !io::stdin().is_terminal() {
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
    let mut model = model.ok_or("no model selected")?;
    let workspace = Workspace::new(std::env::current_dir()?)?;
    console::welcome(&model, &workspace)?;
    let mut composer = console::Composer::default();
    loop {
        let task = composer.compose(&model)?;
        match task.trim() {
            "" => continue,
            "/" | "/help" => {
                composer.notice = "/model change model · /help commands · /quit exit".into();
                continue;
            }
            "/quit" | "/exit" => return Ok(Commands::Exit),
            "/model" => {
                let models = model_catalog().await?;
                if let Some(next) = composer.select_model(&models, &model)? {
                    model = next;
                    remember_model(&model)?;
                }
                continue;
            }
            command if command.starts_with('/') => {
                composer.notice = "Unknown command. Type / for commands.".into();
                continue;
            }
            _ => {}
        }
        return Cli::try_parse_from(["astrid", "run", &task, "--model", &model])?
            .command
            .ok_or_else(|| "missing run command".into());
    }
}

async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let directory = auth::default_directory()?;
    let interactive = cli.command.is_none();
    let command = match cli.command {
        Some(command) => command,
        None => interactive_command().await?,
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
            read_policy,
            write_policy,
            shell_policy,
        } => {
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
            let mut console = Console::new(&model, tools.workspace(), !interactive)?;
            let cancel = Cancellation::default();
            let (sender, mut receiver) = tokio::sync::mpsc::channel(64);
            let (permission_sender, mut permission_receiver) =
                tokio::sync::mpsc::channel::<DecisionRequest>(1);
            let (rendered_sender, mut rendered_receiver) = tokio::sync::mpsc::channel::<()>(1);
            let permission_ui = tokio::spawn(async move {
                while rendered_receiver.recv().await.is_some() {
                    let Some((_request, reply)) = permission_receiver.recv().await else {
                        break;
                    };
                    let answer = terminal_answer().await;
                    let _ = reply.send(answer);
                }
            });
            let mut permissions = TerminalPermission {
                requests: permission_sender,
            };
            let execution = runtime::run(
                &provider,
                &tools,
                &mut permissions,
                RunConfig {
                    model,
                    task,
                    max_model_calls,
                    permissions: PermissionPolicy {
                        read: read_policy,
                        write: write_policy,
                        execute: shell_policy,
                    },
                },
                cancel.clone(),
                Some(sender),
            );
            tokio::pin!(execution);
            let mut resize = tokio::time::interval(Duration::from_millis(150));
            resize.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut output_error = None;
            let mut attached = true;
            let mut signal_enabled = true;
            let result = loop {
                tokio::select! {
                    result=&mut execution=>break result,
                    _=resize.tick(), if attached=>{
                        if let Err(err)=console.resize() {
                            output_error=Some(err);cancel.cancel();receiver.close();attached=false;
                        }
                    }
                    signal=tokio::signal::ctrl_c(), if signal_enabled=>{
                        signal_enabled=false;
                        if let Err(err)=signal {output_error=Some(err);}
                        cancel.cancel();
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
            permission_ui.abort();
            let _ = permission_ui.await;
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
                read_policy: PermissionAction::Deny,
                write_policy: PermissionAction::Ask,
                shell_policy: PermissionAction::Allow,
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
