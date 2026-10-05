use astrid::{
    agent::{self, Observer, Progress},
    auth::{self, Authentication, ChatGptAuth},
    model::ToolOutcome,
    openai::OpenAiProvider,
    tools::{ShellConfirmation, Tools},
    workspace::Workspace,
};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use std::{
    fs::OpenOptions,
    io::{self, BufRead, IsTerminal, Write},
    path::Path,
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
impl Observer for Console {
    fn observe(&mut self, progress: Progress<'_>) -> io::Result<()> {
        match progress {
            Progress::Text(text) => {
                let mut out = io::stdout().lock();
                write!(out, "{}", printable(text))?;
                out.flush()?;
            }
            Progress::ModelStarted(number) => {
                writeln!(io::stderr().lock(), "[model {number}] started")?;
            }
            Progress::ModelCompleted(number) => {
                writeln!(io::stdout().lock())?;
                writeln!(io::stderr().lock(), "[model {number}] completed")?;
            }
            Progress::ToolStarted(call) => {
                writeln!(
                    io::stderr().lock(),
                    "[tool {} {}] started: {:?}",
                    call.call_id,
                    printable(&call.name),
                    call.arguments
                )?;
            }
            Progress::ToolCompleted(result) => {
                let status = if result.is_error() {
                    "failed"
                } else {
                    "completed"
                };
                writeln!(
                    io::stderr().lock(),
                    "[tool {} {}] {status}",
                    result.call_id,
                    printable(&result.name)
                )?;
                match &result.outcome {
                    ToolOutcome::Success { data } => writeln!(
                        io::stderr().lock(),
                        "{}",
                        serde_json::to_string_pretty(data)?
                    )?,
                    ToolOutcome::Error { code, message } => writeln!(
                        io::stderr().lock(),
                        "{}: {}",
                        printable(code),
                        printable(message)
                    )?,
                }
            }
        }
        Ok(())
    }
}

struct TerminalConfirmation;
#[async_trait]
impl ShellConfirmation for TerminalConfirmation {
    async fn confirm(&mut self, command: &str, workspace: &Path) -> io::Result<bool> {
        let command = command.to_owned();
        let workspace = workspace.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let Ok(mut tty)=OpenOptions::new().read(true).write(true).open("/dev/tty") else { return Ok(false); };
            // Debug escaping makes the exact command visible, including control characters.
            writeln!(tty,"\nShell command in {:?}:\n{command:?}\nThis command runs with your account's permissions.",workspace)?;
            write!(tty,"Run this command? Type yes to approve: ")?; tty.flush()?;
            let mut answer=String::new(); io::BufReader::new(&tty).read_line(&mut answer)?;
            Ok(answer.trim().eq_ignore_ascii_case("yes"))
        }).await.map_err(io::Error::other)?
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
            let result = agent::run(
                &provider,
                &tools,
                &mut TerminalConfirmation,
                &mut Console,
                &model,
                &task,
                max_model_calls,
            )
            .await?;
            eprintln!(
                "[finished] {} model calls, {} tool calls",
                result.model_calls, result.tool_calls
            );
        }
    }
    Ok(())
}
