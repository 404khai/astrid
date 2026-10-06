use astrid::{
    auth::{self, Authentication, ChatGptAuth},
    cancellation::Cancellation,
    events::{EventKind, ExecutionEvent},
    model::ToolOutcome,
    openai::OpenAiProvider,
    runtime::{self, PermissionHandler, RunConfig, RunOutcome},
    tools::{PermissionRequest, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use std::{
    fs::OpenOptions,
    io::{self, IsTerminal, Write},
    os::fd::AsRawFd,
    os::unix::fs::OpenOptionsExt,
    process::ExitCode,
    sync::Arc,
    time::Duration,
};

// Pixel interpretation of the supplied logo.png: bracketed face with two eyes.
const LOGO: &str = "       ████████████████\n     ████            ████\n██████                  ██████\n██████    ██    ██      ██████\n          ██    ██\n          ██    ██\n██████                  ██████\n██████                  ██████\n     ████            ████\n       ████████████████";

fn terminal_logo() -> String {
    if std::env::var_os("NO_COLOR").is_some() {
        return LOGO.into();
    }
    let mut result = String::new();
    for (row, line) in LOGO.lines().enumerate() {
        if row > 0 {
            result.push('\n');
        }
        for (column, pixel) in line.chars().enumerate() {
            if pixel == ' ' {
                result.push(pixel);
                continue;
            }
            let (r, g, b) = if (3..=5).contains(&row) && matches!(column, 10 | 11 | 16 | 17) {
                (0, 247, 213)
            } else {
                // Horizontal gradient: outer blue -> center blue -> outer blue.
                let t = (column as f64 / 29.0 - 0.5).abs() * 2.0;
                let blend = |center: f64, edge: f64| (center + (edge - center) * t).round() as u8;
                (blend(26.0, 68.0), blend(50.0, 89.0), blend(236.0, 249.0))
            };
            result.push_str(&format!("\x1b[38;2;{r};{g};{b}m{pixel}\x1b[0m"));
        }
    }
    result
}

#[derive(Parser)]
#[command(name="astrid", version, about="One repository task, one observable model/tool loop", after_help=LOGO)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    },
}

fn positive(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(|| "value must be a positive integer".into())
}

fn printable(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect()
}

struct Console;
impl Console {
    fn render(&mut self, event: &ExecutionEvent) -> io::Result<()> {
        let mut err = io::stderr().lock();
        let tool = event
            .tool_call_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        match &event.kind {
            EventKind::ModelTextDelta { text } => {
                let mut out = io::stdout().lock();
                write!(out, "{}", printable(text))?;
                out.flush()?;
            }
            EventKind::ModelCallStarted { number } => writeln!(err, "[model {number}] started")?,
            EventKind::ModelCallCompleted { .. } => {
                writeln!(io::stdout().lock())?;
                writeln!(err, "[model] completed")?;
            }
            EventKind::ModelCallFailed { .. } | EventKind::ModelCallCancelled => {
                writeln!(err, "\n[response interrupted]")?
            }
            EventKind::ToolCallRequested { call } => writeln!(
                err,
                "[tool {} {}] requested: {:?}",
                tool,
                printable(&call.name),
                call.arguments
            )?,
            EventKind::ToolCallStarted => writeln!(err, "[tool {}] started", tool)?,
            EventKind::ToolCallCompleted { outcome }
            | EventKind::ToolCallFailed { outcome }
            | EventKind::ToolCallDenied { outcome }
            | EventKind::ToolCallTimedOut { outcome } => {
                writeln!(
                    err,
                    "[tool {}] {}",
                    tool,
                    match event.kind {
                        EventKind::ToolCallCompleted { .. } => "completed",
                        EventKind::ToolCallDenied { .. } => "denied",
                        EventKind::ToolCallTimedOut { .. } => "timed out",
                        _ => "failed",
                    }
                )?;
                match outcome {
                    ToolOutcome::Success { data } | ToolOutcome::TimedOut { data } => {
                        writeln!(err, "{}", serde_json::to_string_pretty(data)?)?
                    }
                    ToolOutcome::Error { code, message } => {
                        writeln!(err, "{}: {}", printable(code), printable(message))?
                    }
                }
            }
            EventKind::ToolCallCancelled => writeln!(err, "[tool {}] cancelled", tool)?,
            EventKind::ToolCallSkipped { .. } => writeln!(err, "[tool {}] skipped", tool)?,
            EventKind::CancellationRequested => writeln!(err, "[run] cancellation requested")?,
            EventKind::RunCompleted { .. } => writeln!(err, "✓ Run completed")?,
            _ => {}
        }
        Ok(())
    }
}

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

async fn terminal_answer(request: &PermissionRequest) -> io::Result<bool> {
    let Ok(mut tty) = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")
    else {
        return Ok(false);
    };
    writeln!(
        tty,
        "\nShell command in {:?}:\n{:?}\nThis command runs with your account's permissions.",
        request.workspace, request.command
    )?;
    write!(tty, "Run this command? Type yes to approve: ")?;
    tty.flush()?;
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

async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let directory = auth::default_directory()?;
    match cli.command {
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
        } => {
            let workspace = Workspace::new(std::env::current_dir()?)?;
            if directory.starts_with(workspace.root()) {
                return Err(
                    "choose a repository workspace outside Astrid's credential directory ancestors"
                        .into(),
                );
            }
            let tools = Tools::new(workspace, Duration::from_secs(shell_timeout as u64))?;
            let provider = OpenAiProvider::new(Arc::new(ChatGptAuth::new(directory)?))?;
            if io::stderr().is_terminal() {
                eprintln!("{}\nAstrid · {model}", terminal_logo());
            }
            let cancel = Cancellation::default();
            let (sender, mut receiver) = tokio::sync::mpsc::channel(64);
            let (permission_sender, mut permission_receiver) =
                tokio::sync::mpsc::channel::<DecisionRequest>(1);
            let (rendered_sender, mut rendered_receiver) = tokio::sync::mpsc::channel::<()>(1);
            let permission_ui = tokio::spawn(async move {
                while rendered_receiver.recv().await.is_some() {
                    let Some((request, reply)) = permission_receiver.recv().await else {
                        break;
                    };
                    let answer = terminal_answer(&request).await;
                    let _ = reply.send(answer);
                }
            });
            let mut permissions = TerminalPermission {
                requests: permission_sender,
            };
            let mut console = Console;
            let execution = runtime::run(
                &provider,
                &tools,
                &mut permissions,
                RunConfig {
                    model,
                    task,
                    max_model_calls,
                },
                cancel.clone(),
                Some(sender),
            );
            tokio::pin!(execution);
            let mut output_error = None;
            let mut attached = true;
            let mut signal_enabled = true;
            let result = loop {
                tokio::select! {
                    result=&mut execution=>break result,
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
            eprintln!(
                "[finished] {} model calls, {} tool outcomes",
                result.model_calls, result.tool_calls
            );
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
        os::{fd::OwnedFd, unix::net::UnixStream},
    };
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
