#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;
use astrid::{
    auth::{self, Authentication, ChatGptAuth},
    cancellation::Cancellation,
    events::EventKind,
    observability::{self, Switch},
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

mod batch_cli;
mod console;
#[path = "instabox_cli.rs"]
mod instabox;
mod logo;
mod remote_cli;
mod sessions;
use console::Console;
use logo::LOGO;

#[derive(Parser)]
#[command(name="astrid", version, about="One delegated task, one observable model/tool loop", after_help=LOGO)]
struct Cli {
    /// Opt in for this foreground invocation only; each remote call still requires exact approval.
    #[arg(long, global = true, value_parser = remote_cli::machine_id)]
    instabox_machine: Option<String>,
    /// One-shot visual computer use; separate observe, action and per-frame image approvals.
    #[arg(long, global = true, requires = "instabox_machine")]
    computer_use: bool,
    /// One-shot local look (ADR 0027): Astrid may see one display of this Mac,
    /// one approval per frame (capture plus disclosure), then point and sketch.
    #[arg(long, global = true, conflicts_with = "instabox_machine")]
    look: bool,
    /// Display index for --look (0 = the display with the menu bar).
    #[arg(long, global = true, default_value_t = 0, value_parser = clap::value_parser!(u32).range(0..=15), requires = "look")]
    look_display: u32,
    /// Presence host: `Astrid Presence.app` (recommended; it owns Screen Recording)
    /// or the `astrid-presence` executable. Defaults to $ASTRID_PRESENCE_HOST.
    #[arg(long, global = true, env = "ASTRID_PRESENCE_HOST", requires = "look")]
    presence_host: Option<std::path::PathBuf>,
    /// Development: the host draws a synthetic desktop instead of reading the
    /// screen. That synthetic frame is still sent to the model on approval.
    #[arg(long, global = true, requires = "look")]
    presence_synthetic: bool,
    /// Read-only mail for this run (ADR 0028): apple, gmail or all. Each
    /// mail_search/mail_read call still asks for approval.
    #[arg(long, global = true, value_parser = ["apple", "gmail", "all"])]
    mail: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum SettingsCommand {
    /// Persist the default for subsequent runs.
    Set {
        #[arg(value_parser=["observability", "expanded-tool-calls"])]
        key: String,
        #[arg(value_enum)]
        value: Switch,
    },
}
#[derive(Subcommand)]
enum OpencodeCommand {
    /// Paste an OpenCode Go API key; it is stored in your login Keychain.
    Login,
    /// Delete the stored key from the Keychain.
    Logout,
    /// Show whether a key is available, without printing it.
    Status,
    /// List OpenCode Go models usable by Astrid (public catalog, no key needed).
    Models,
}

async fn opencode_command(command: OpencodeCommand) -> Result<(), Box<dyn std::error::Error>> {
    use astrid::opencode::{ApiKey, SERVICE};
    match command {
        OpencodeCommand::Login => {
            eprintln!(
                "Paste your OpenCode Go API key (opencode.ai → Go → API key) and press Return."
            );
            ApiKey::new(rpassword_line()?)?.store()?;
            eprintln!(
                "Stored in your login Keychain. List models with `astrid opencode models`, then run with --model opencode/<id>."
            );
        }
        OpencodeCommand::Logout => eprintln!(
            "Keychain key {}.",
            if astrid::keychain::delete(SERVICE) {
                "deleted"
            } else {
                "not present"
            }
        ),
        OpencodeCommand::Status => println!(
            "opencode\t{}",
            if std::env::var("OPENCODE_API_KEY").is_ok_and(|k| !k.trim().is_empty()) {
                "API key from OPENCODE_API_KEY"
            } else if astrid::keychain::read(SERVICE).ok().flatten().is_some() {
                "API key in Keychain"
            } else {
                "no key; run `astrid opencode login`"
            }
        ),
        OpencodeCommand::Models => {
            for model in astrid::opencode::catalog().await? {
                println!(
                    "opencode/{}\treasoning effort: {}",
                    model.id,
                    if model.efforts.is_empty() {
                        "-".into()
                    } else {
                        model.efforts.join("/")
                    }
                );
            }
        }
    }
    Ok(())
}

#[derive(Subcommand)]
enum ClaudeCommand {
    /// Paste a Claude Console API key; it is stored in your login Keychain.
    Login,
    /// Delete the stored key from the Keychain.
    Logout,
    /// Show whether a key is available, without printing it.
    Status,
}

fn claude_command(command: ClaudeCommand) -> Result<(), Box<dyn std::error::Error>> {
    use astrid::anthropic::{ApiKey, keychain};
    match command {
        ClaudeCommand::Login => {
            eprintln!(
                "Paste an API key from https://platform.claude.com/settings/keys and press Return.\nClaude Pro/Max chat subscriptions cannot be used by third-party apps; API usage is billed to your Console account."
            );
            let key = rpassword_line()?;
            keychain::store(&ApiKey::new(key)?)?;
            eprintln!(
                "Stored in your login Keychain as \"Astrid Claude API key\". Try: astrid run --model claude-opus-5-5 \"hello\""
            );
        }
        ClaudeCommand::Logout => eprintln!(
            "Keychain key {}.",
            if keychain::delete() {
                "deleted"
            } else {
                "not present"
            }
        ),
        ClaudeCommand::Status => println!(
            "claude\t{}",
            if std::env::var("ANTHROPIC_API_KEY").is_ok_and(|k| !k.trim().is_empty()) {
                "API key from ANTHROPIC_API_KEY"
            } else if keychain::read().ok().flatten().is_some() {
                "API key in Keychain"
            } else {
                "no key; run `astrid claude login`"
            }
        ),
    }
    Ok(())
}

/// Reads one line from the terminal with echo off, so the key is not shown.
fn rpassword_line() -> Result<String, Box<dyn std::error::Error>> {
    use std::io::BufRead;
    let fd = libc::STDIN_FILENO;
    let mut term: libc::termios = unsafe { std::mem::zeroed() };
    let tty = unsafe { libc::tcgetattr(fd, &mut term) } == 0;
    let original = term;
    if tty {
        term.c_lflag &= !libc::ECHO;
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &term) };
    }
    let mut line = String::new();
    let read = io::stdin().lock().read_line(&mut line);
    if tty {
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, &original) };
        eprintln!();
    }
    read?;
    Ok(line.trim().to_owned())
}

#[derive(Subcommand)]
enum MailCommand {
    /// Sign in to Gmail in the browser with read-only access; the token goes to the Keychain.
    GmailLogin {
        /// Print the sign-in URL without opening the browser.
        #[arg(long)]
        no_browser: bool,
    },
    /// Revoke Gmail access and delete the Keychain token.
    GmailLogout,
    /// Show which mail sources are ready, without reading any mail.
    Status,
}

async fn mail_command(command: MailCommand) -> Result<(), Box<dyn std::error::Error>> {
    use astrid::mail::{ClientConfig, Endpoints, keychain, login};
    match command {
        MailCommand::GmailLogin { no_browser } => {
            let client = ClientConfig::load()?;
            let token = login::run(&client, &Endpoints::default(), !no_browser).await?;
            keychain::store(&token)?;
            eprintln!(
                "Gmail connected with read-only access (gmail.readonly). Token stored in your login Keychain as \"Astrid Gmail (read-only)\"."
            );
        }
        MailCommand::GmailLogout => {
            let revoked = match keychain::read()? {
                Some(token) => login::revoke(&token, &Endpoints::default()).await,
                None => false,
            };
            let deleted = keychain::delete();
            eprintln!(
                "Gmail token {}; Keychain item {}.",
                if revoked {
                    "revoked at Google"
                } else {
                    "not revoked (none stored or Google unreachable)"
                },
                if deleted { "deleted" } else { "not present" }
            );
        }
        MailCommand::Status => {
            let apple = astrid::mail::AppleMail::default_root().is_some_and(|p| p.is_dir());
            let client = ClientConfig::path().is_some_and(|p| p.is_file());
            let signed_in = keychain::read().ok().flatten().is_some();
            println!(
                "apple\t{}",
                if apple {
                    "Mail store present (Spotlight search; may need Full Disk Access)"
                } else {
                    "no ~/Library/Mail store found"
                }
            );
            println!(
                "gmail\t{}",
                match (client, signed_in) {
                    (true, true) => "signed in (read-only)",
                    (true, false) => "client configured; run `astrid mail gmail-login`",
                    (false, _) =>
                        "save your Google OAuth desktop client as ~/.config/astrid/gmail-client.json",
                }
            );
        }
    }
    Ok(())
}

fn mail_tools(choice: &str) -> Result<astrid::mail::Mail, Box<dyn std::error::Error>> {
    use astrid::mail::{AppleMail, Gmail, MailSource, Spotlight};
    let mut sources: Vec<Arc<dyn MailSource>> = vec![];
    if matches!(choice, "apple" | "all") {
        let root = AppleMail::default_root().ok_or("HOME is not set")?;
        sources.push(Arc::new(AppleMail::new(root, Arc::new(Spotlight))));
    }
    if matches!(choice, "gmail" | "all") {
        sources.push(Arc::new(Gmail::load()?));
    }
    Ok(astrid::mail::Mail::new(sources)?)
}

#[derive(Subcommand)]
enum BatchCommand {
    /// Execute a bounded caller-authored DAG with fresh read-only workers.
    Run {
        manifest: std::path::PathBuf,
        #[arg(long)]
        folder: Option<std::path::PathBuf>,
        #[arg(long, default_value_t = 2, value_parser = positive)]
        concurrency: usize,
        #[arg(long, value_enum)]
        observability: Option<Switch>,
        #[arg(long, value_enum, default_value = "allow")]
        read_policy: PermissionAction,
        /// Emit the complete bounded source bundle and lifecycle as JSON after join.
        #[arg(long)]
        json: bool,
    },
    /// List or inspect metadata-only batch correlation; never resume a job.
    Trace { batch_id: Option<String> },
}

#[derive(Subcommand)]
enum Commands {
    /// OpenCode Go key and models for opencode/* models (ADR 0031).
    Opencode {
        #[command(subcommand)]
        command: OpencodeCommand,
    },
    /// Claude API key for claude-* models (ADR 0030).
    Claude {
        #[command(subcommand)]
        command: ClaudeCommand,
    },
    /// Gmail sign-in for read-only mail search (ADR 0028).
    Mail {
        #[command(subcommand)]
        command: MailCommand,
    },
    /// Explicit metadata discovery or exact-command execution on an existing InstaBox machine.
    Instabox {
        #[command(subcommand)]
        command: instabox::Command,
    },
    /// Inspect bounded read-only task batches.
    Batch {
        #[command(subcommand)]
        command: BatchCommand,
    },
    /// Show or update optional observability settings.
    Settings {
        #[command(subcommand)]
        command: Option<SettingsCommand>,
    },
    /// List retained traces or inspect a run's metadata.
    Trace { run_id: Option<String> },
    /// Aggregate known usage across retained traces (unknown values remain unknown).
    Stats,
    /// Exit interactive startup.
    #[command(hide = true)]
    Exit,
    /// Sign in with ChatGPT using Astrid's own registration and credentials.
    Login,
    /// List models available to the signed-in ChatGPT account.
    Models,
    /// Execute one fresh task in the invocation directory, or read-only notes in a selected folder.
    Run {
        #[arg(
            long,
            help = "Read-only notes checklist; enforces read Allow/write Deny/execute Deny over all permission flags"
        )]
        notes: bool,
        #[arg(
            long,
            requires = "notes",
            help = "Notes directory (defaults to invocation directory)"
        )]
        folder: Option<std::path::PathBuf>,
        #[arg(
            long,
            value_enum,
            help = "Override optional telemetry and metadata trace recording"
        )]
        observability: Option<Switch>,
        task: String,
        #[arg(
            long,
            env = "ASTRID_MODEL",
            help = "OpenAI model identifier (required; no hard-coded default)"
        )]
        model: String,
        #[arg(
            long,
            value_enum,
            help = "Explicit supported provider reasoning effort; omission uses provider default"
        )]
        reasoning_effort: Option<astrid::routing::ReasoningEffort>,
        #[arg(
            long,
            help = "JSON direct-inference stage policy; visible-history handoff, no agent harness"
        )]
        routing: Option<std::path::PathBuf>,
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
        #[arg(long, env = "ASTRID_CONTEXT_TOKENS", default_value_t = astrid::context::ContextBudget::default().estimated_context_tokens, value_parser = positive, help = "Estimated context allowance; not the model's known capacity")]
        context_tokens: usize,
        #[arg(
            long,
            env = "ASTRID_RESPONSE_RESERVE",
            default_value = "4096",
            help = "Planning reserve; does not enforce provider output length"
        )]
        response_reserve: usize,
        #[arg(long, env = "ASTRID_CONTEXT_BYTES", default_value = "524288", value_parser = positive)]
        context_bytes: usize,
        #[arg(long, default_value = "4096", value_parser = positive)]
        summary_bytes: usize,
        #[arg(long, value_enum, default_value = "file-references")]
        context_policy: astrid::context::SelectionPolicy,
    },
}

fn routing_policy(
    path: Option<&std::path::Path>,
    effort: Option<astrid::routing::ReasoningEffort>,
) -> Result<Option<astrid::routing::RoutingPolicy>, Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut policy = if let Some(path) = path {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(65_537)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 65_536 {
            return Err("routing configuration exceeds 64 KiB".into());
        }
        serde_json::from_slice::<astrid::routing::RoutingPolicy>(&bytes)?
    } else {
        Default::default()
    };
    if effort.is_some() && policy.initial_effort.is_some() {
        return Err("set initial effort in the routing file or flag, not both".into());
    }
    if effort.is_some() {
        policy.initial_effort = effort;
    }
    Ok((path.is_some() || effort.is_some()).then_some(policy))
}

fn run_policy(
    notes: bool,
    mode: PermissionMode,
    read: Option<PermissionAction>,
    write: Option<PermissionAction>,
    execute: Option<PermissionAction>,
) -> PermissionPolicy {
    if notes {
        PermissionPolicy::read_only()
    } else {
        let defaults = mode.policy();
        PermissionPolicy {
            read: read.unwrap_or(defaults.read),
            write: write.unwrap_or(defaults.write),
            execute: execute.unwrap_or(defaults.execute),
        }
    }
}

fn notes_notice(model: &str, workspace: &Workspace, policy: PermissionPolicy) -> String {
    format!(
        "Notes folder: {}\nProvider: OpenAI / Codex subscription; model: {}\nEffective policy (custom notes): read={:?}, write={:?}, execute={:?}\nRetrieved notes are sent to this provider. Native reads cover the selected folder under existing path validation. Runtime evidence subprocesses and provider network traffic retain existing authority.",
        printable(&workspace.root().display().to_string()),
        printable(model),
        policy.read,
        policy.write,
        policy.execute
    )
}

fn coverage_notice(tool: &astrid::model::ToolResult) -> String {
    use astrid::model::ToolOutcome;
    match &tool.outcome {
        ToolOutcome::Success { data } | ToolOutcome::TimedOut { data } => {
            format!(
                "path={} range={} coverage={} truncated={} page={}",
                data["path"], data["range"], data["coverage"], data["truncated"], data["page"]
            )
        }
        ToolOutcome::Error { code, message } => format!("{code}: {message}"),
    }
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
    let tty = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")?;
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
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "permission input closed",
            ));
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
    if std::env::args().nth(1).as_deref() == Some("__astrid_decode_png") {
        return if astrid::computer_png::worker_stdio().is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
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

fn observability_options(
    override_value: Option<Switch>,
) -> Result<Option<observability::Options>, Box<dyn std::error::Error>> {
    observability_options_at(&auth::default_directory()?, override_value)
}
fn observability_options_at(
    directory: &std::path::Path,
    override_value: Option<Switch>,
) -> Result<Option<observability::Options>, Box<dyn std::error::Error>> {
    let path = directory.join("settings.json");
    let (settings, exists) = observability::Settings::load_with_source(&path)?;
    let (value, source) = settings.resolve(override_value, exists);
    eprintln!("Observability: {value} ({source})");
    Ok((value == Switch::On).then(|| observability::Options::new(directory.join("traces"))))
}
fn recording_notice(result: &runtime::RunResult) {
    match result.recording.status {
        observability::RecordingStatus::Off => {}
        observability::RecordingStatus::Complete => {
            eprintln!("Trace: {} (metadata only)", result.recording.run_id)
        }
        _ => eprintln!(
            "Trace: {} ({:?}: {}; run outcome unchanged)",
            result.recording.run_id,
            result.recording.status,
            printable(
                result
                    .recording
                    .diagnostic
                    .as_deref()
                    .unwrap_or("recording incomplete")
            )
        ),
    }
}
async fn interactive_loop(
    remote: Option<remote_cli::ForegroundRemote>,
) -> Result<(), Box<dyn std::error::Error>> {
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
        return Err("choose a workspace outside Astrid's credential directory ancestors".into());
    }
    let timeout = configured_limit("ASTRID_SHELL_TIMEOUT", 30)?;
    let tools = Tools::new(workspace, Duration::from_secs(timeout as u64))?;
    let provider = OpenAiProvider::new(Arc::new(ChatGptAuth::new(directory.clone())?))?;
    conversation_loop(&provider, tools, model, &directory, remote.as_ref()).await
}

async fn conversation_loop(
    provider: &dyn astrid::model::ModelProvider,
    mut tools: Tools,
    mut model: String,
    model_directory: &std::path::Path,
    remote: Option<&remote_cli::ForegroundRemote>,
) -> Result<(), Box<dyn std::error::Error>> {
    let max_model_calls = configured_limit("ASTRID_MAX_MODEL_CALLS", 20)?;
    let mut context_budget = astrid::context::ContextBudget::default();
    context_budget.estimated_context_tokens = configured_limit(
        "ASTRID_CONTEXT_TOKENS",
        context_budget.estimated_context_tokens,
    )?;
    context_budget.response_reserve_tokens = configured_limit(
        "ASTRID_RESPONSE_RESERVE",
        context_budget.response_reserve_tokens,
    )?;
    context_budget.max_request_bytes =
        configured_limit("ASTRID_CONTEXT_BYTES", context_budget.max_request_bytes)?;
    if !context_budget.validate() {
        return Err(
            "context allowance must exceed the response reserve, with positive byte limits".into(),
        );
    }

    let mut sessions = sessions::Sessions::open(tools.workspace(), model_directory)?;
    let mut composer = console::Composer {
        workspace_root: Some(tools.workspace().root().to_path_buf()),
        ..Default::default()
    };

    let mut routing = None;
    let mut startup = true;
    loop {
        if let Err(error) = sessions.persist() {
            let message = format!(
                "Session save failed; latest changes may not survive restart: {}",
                printable(&error.to_string())
            );
            eprintln!("astrid: {message}");
            composer.notice = message;
        }
        if startup {
            composer.header = Some(startup_identity(
                &model,
                tools.workspace(),
                composer.mode.policy(),
            ));
        }
        composer.session_label = sessions.active_label();
        composer.context_label = sessions.context_label();
        let task = composer.compose(&model)?;
        match task.trim() {
            "" => continue,
            "/" | "/help" => {
                composer.notice =
                    "/mode · /sessions · /new · /model · /routing <file|off> · /settings · /quit"
                        .into();
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
            command if command == "/routing" || command.starts_with("/routing ") => {
                let argument = command.strip_prefix("/routing").unwrap().trim();
                if argument == "off" {
                    routing = None;
                    composer.notice =
                        "Routing off for subsequent runs; session replay safety remains in effect."
                            .into();
                } else if argument.is_empty() {
                    composer.notice = "Use /routing <JSON file> or /routing off. Policy applies to subsequent runs and is not persisted.".into();
                } else {
                    match routing_policy(Some(std::path::Path::new(argument)), None) {
                        Ok(Some(policy)) if policy.valid(&model, max_model_calls, provider) => {
                            routing = Some(policy);
                            composer.notice = "Routing on. Policy is not saved; visible-history mode persists with the session.".into();
                        }
                        Ok(_) => composer.notice = "Routing policy is unsupported for this model/provider or call ceiling.".into(),
                        Err(error) => composer.notice = format!("Routing policy rejected: {}", printable(&error.to_string())),
                    }
                }
                continue;
            }
            "/new" => {
                composer.notice = sessions
                    .create(tools.workspace())
                    .err()
                    .unwrap_or_else(|| "New session. Saved locally for this workspace.".into());
                continue;
            }
            "/sessions" => {
                let options = sessions.choices();
                let current = options[sessions.active_index()].clone();
                if let Some(choice) =
                    composer.choose("Sessions — select to continue", &options, &current)?
                {
                    sessions.select(&choice);
                }
                composer.notice = "Sessions are saved locally for this workspace.".into();
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
                        }
                        Err(_) => {
                            composer.notice = "Use /mode ask, /mode auto, or /mode unbound.".into()
                        }
                    }
                }
                continue;
            }
            command if command == "/settings" || command.starts_with("/settings ") => {
                let path = model_directory.join("settings.json");
                let mut settings = observability::Settings::load(&path)?;
                let value = if command == "/settings" {
                    composer.choose(
                        "Expanded tool calls",
                        &["on".into(), "off".into()],
                        &settings.expanded_tool_calls.to_string(),
                    )?
                } else {
                    command
                        .strip_prefix("/settings expanded-tool-calls ")
                        .map(str::to_owned)
                };
                match value.as_deref() {
                    Some("on") | Some("off") => {
                        settings.expanded_tool_calls = if value.as_deref() == Some("on") {
                            Switch::On
                        } else {
                            Switch::Off
                        };
                        settings.save(&path)?;
                        composer.notice =
                            format!("Expanded tool calls: {}", settings.expanded_tool_calls);
                    }
                    None if command == "/settings" => {}
                    _ => composer.notice = "Use /settings expanded-tool-calls on|off".into(),
                }
                continue;
            }
            command if command.starts_with('/') => {
                composer.notice = "Unknown command. Type / for commands.".into();
                continue;
            }
            _ => {}
        }
        if startup {
            console::welcome(&startup_identity(
                &model,
                tools.workspace(),
                composer.mode.policy(),
            ))?;
            composer.header = None;
            startup = false;
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
            requests: permission_sender.clone(),
        };
        if let Some(remote) = remote {
            tools = remote.install(tools, permission_sender)?;
        }
        let observability = observability_options_at(model_directory, None)?;
        let session = sessions.take_active();
        let result = drive_run(
            runtime::run_in_session(
                provider,
                &tools,
                &mut permissions,
                RunConfig {
                    routing: routing.clone(),
                    observability,
                    model: model.clone(),
                    task,
                    max_model_calls,
                    permissions: composer.mode.policy(),
                    context_budget: Some(context_budget.clone()),
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
            Ok(result) => {
                recording_notice(&result);
                sessions.complete(result)
            }
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

fn validate_computer_scope(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    if cli.look {
        match &cli.command {
            Some(Commands::Run{notes:false,model,routing:None,mode,read_policy,write_policy,shell_policy,..})
                if astrid::computer::supported_model(model) && run_policy(false,*mode,*read_policy,*write_policy,*shell_policy) != PermissionPolicy::read_only()=>{},
            _=>return Err("--look requires a fresh one-shot run, no notes/routing, and an explicitly supported image model".into()),
        }
        if cli.presence_host.is_none() {
            return Err("--look needs --presence-host (Astrid Presence.app from apps/presence-mac/scripts/bundle.sh) or ASTRID_PRESENCE_HOST".into());
        }
    }
    if cli.computer_use {
        match &cli.command {
            Some(Commands::Run{notes:false,model,routing:None,mode,read_policy,write_policy,shell_policy,..})
                if astrid::computer::supported_model(model) && run_policy(false,*mode,*read_policy,*write_policy,*shell_policy) != PermissionPolicy::read_only()=>{},
            _=>return Err("--computer-use requires a fresh one-shot run, no notes/routing, and an explicitly supported image model".into()),
        }
    }
    Ok(())
}

async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    validate_computer_scope(&cli)?;
    if cli.mail.is_some() && !matches!(cli.command, Some(Commands::Run { notes: false, .. })) {
        return Err("--mail requires a one-shot run without --notes".into());
    }
    let mail = cli.mail.clone();
    // Reject unsupported scopes before loading any remote credentials or provider setup.
    if cli.instabox_machine.is_some()
        && !matches!(cli.command, None | Some(Commands::Run { notes: false, .. }))
    {
        return Err("--instabox-machine requires foreground startup or run without --notes".into());
    }
    let remote = cli
        .instabox_machine
        .map(|machine| {
            if cli.computer_use {
                remote_cli::ForegroundRemote::load_computer(machine)
            } else {
                remote_cli::ForegroundRemote::load(machine)
            }
        })
        .transpose()?;
    let look = cli.look.then(|| LookSetup {
        display: cli.look_display,
        host: cli.presence_host.clone().expect("validated presence host"),
        synthetic: cli.presence_synthetic,
    });
    let command = match cli.command {
        Some(Commands::Instabox { command }) => return instabox::run(command).await,
        Some(Commands::Mail { command }) => return mail_command(command).await,
        Some(Commands::Claude { command }) => return claude_command(command),
        Some(Commands::Opencode { command }) => return opencode_command(command).await,
        Some(command) => command,
        None => return interactive_loop(remote).await,
    };
    let directory = auth::default_directory()?;
    match command {
        Commands::Instabox { command } => return instabox::run(command).await,
        Commands::Mail { command } => return mail_command(command).await,
        Commands::Claude { command } => return claude_command(command),
        Commands::Opencode { command } => return opencode_command(command).await,
        Commands::Batch {
            command:
                BatchCommand::Run {
                    manifest,
                    folder,
                    concurrency,
                    observability,
                    read_policy,
                    json,
                },
        } => {
            let manifest = astrid::batch_manifest::Manifest::load(&manifest)?;
            let workspace = Workspace::new(folder.unwrap_or(std::env::current_dir()?))?;
            if directory.starts_with(workspace.root()) {
                return Err(
                    "choose a workspace outside Astrid's credential directory ancestors".into(),
                );
            }
            let options = observability_options(observability)?;
            let parent = PermissionPolicy {
                read: read_policy,
                ..PermissionPolicy::read_only()
            };
            let spec = manifest.into_spec(concurrency, parent, options);
            let provider = OpenAiProvider::new(Arc::new(ChatGptAuth::new(directory)?))?;
            spec.validate(&provider)?;
            eprintln!(
                "Read-only batch: {} tasks, concurrency {}; folder {}. Direct model inference uses each task's configured provider model. Retrieved content is sent to that provider. Shared reads are not a snapshot. Results print after owned work joins.",
                spec.tasks.len(),
                spec.concurrency,
                printable(&workspace.root().display().to_string())
            );
            let tools = Tools::new(workspace, Duration::from_secs(1))?;
            let result = batch_cli::drive(&provider, &tools, spec, async {
                if let Err(error) = tokio::signal::ctrl_c().await {
                    eprintln!(
                        "Cancellation signal unavailable: {}; requesting cleanup",
                        printable(&error.to_string())
                    );
                }
            })
            .await?;
            batch_cli::render(&result, json, &mut io::stdout().lock())?;
            batch_cli::check_outcome(result.outcome)?;
        }
        Commands::Batch {
            command: BatchCommand::Trace { batch_id },
        } => {
            let store = directory.join("traces");
            if let Some(id) = batch_id {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&astrid::batch_trace::read(&store, &id)?)?
                );
            } else {
                let paths = astrid::batch_trace::list(&store)?;
                if paths.is_empty() {
                    println!("No retained batch traces. Observability defaults to off.");
                }
                for path in paths {
                    let id = path
                        .file_stem()
                        .and_then(|v| v.to_str())
                        .and_then(|v| v.strip_prefix("batch-"))
                        .ok_or("invalid batch trace name")?;
                    match astrid::batch_trace::read(&store, id) {
                        Ok(trace) => println!(
                            "{}  {}  {:?}  {} tasks; child coverage is separate",
                            trace.batch,
                            if trace.complete {
                                "complete"
                            } else {
                                "incomplete"
                            },
                            trace.state.outcome,
                            trace.state.tasks.len()
                        ),
                        Err(error) => {
                            eprintln!("{}: {}", printable(id), printable(&error.to_string()))
                        }
                    }
                }
            }
        }
        Commands::Settings { command } => {
            let path = directory.join("settings.json");
            let (mut settings, mut exists) = observability::Settings::load_with_source(&path)?;
            if let Some(SettingsCommand::Set { key, value }) = command {
                if key == "expanded-tool-calls" {
                    settings.expanded_tool_calls = value;
                } else {
                    settings.observability = value;
                }
                settings.save(&path)?;
                exists = true;
            }
            let (value, source) = settings.resolve(None, exists);
            println!(
                "observability: {value} ({source})\nexpanded-tool-calls: {}\nsettings: {}\ntraces: {}\nChanges affect future runs; existing traces are retained.",
                settings.expanded_tool_calls,
                path.display(),
                directory.join("traces").display()
            );
        }
        Commands::Trace { run_id } => {
            let store = directory.join("traces");
            if let Some(id) = run_id {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &observability::read_trace(&store, &id)?.summary()
                    )?
                );
            } else {
                let paths = observability::list_traces(&store)?;
                if paths.is_empty() {
                    println!("No retained traces. Observability defaults to off.");
                }
                for path in paths {
                    let id = path
                        .file_stem()
                        .and_then(|v| v.to_str())
                        .ok_or("invalid trace filename")?;
                    match observability::read_trace(&store, id) {
                        Ok(trace) => {
                            let s = trace.summary();
                            println!(
                                "{}  {}  {}  {}",
                                s.run_id,
                                if s.complete { "complete" } else { "incomplete" },
                                printable(s.model.as_deref().unwrap_or("model unavailable")),
                                s.outcome.unwrap_or_else(|| "outcome unavailable".into())
                            );
                        }
                        Err(e) => eprintln!("{}: {}", printable(id), printable(&e.to_string())),
                    }
                }
            }
        }
        Commands::Stats => {
            let store = directory.join("traces");
            let mut stats = observability::Stats::default();
            for path in observability::list_traces(&store)? {
                let id = path
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .ok_or("invalid trace filename")?;
                match observability::read_trace(&store, id) {
                    Ok(trace) => stats.add(&trace.summary()),
                    Err(e) => {
                        stats.unreadable_traces += 1;
                        eprintln!("{}: {}", printable(id), printable(&e.to_string()));
                    }
                }
            }
            println!("{}", serde_json::to_string_pretty(&stats)?);
            println!(
                "Known totals only; cached tokens are a subset of input. Cost and unrecorded runs are unavailable."
            );
        }
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
            notes,
            folder,
            observability,
            task,
            model,
            max_model_calls,
            reasoning_effort,
            routing,
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
            let routing = routing_policy(routing.as_deref(), reasoning_effort)?;
            let observability = observability_options(observability)?;
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
            let workspace = Workspace::new(folder.unwrap_or(std::env::current_dir()?))?;
            if directory.starts_with(workspace.root()) {
                return Err(
                    "choose a workspace outside Astrid's credential directory ancestors".into(),
                );
            }
            remember_model(&model)?;
            let mut tools = Tools::new(workspace, Duration::from_secs(shell_timeout as u64))?;
            if let Some(choice) = mail.as_deref() {
                tools = tools.with_mail(mail_tools(choice)?);
            }
            // ADR 0030: exact claude-* model IDs use the Anthropic adapter.
            let provider: Box<dyn astrid::model::ModelProvider> = if let Some(inner) =
                model.strip_prefix(astrid::claude_code::PREFIX)
            {
                // ADR 0032: Claude Agent SDK on whatever account Claude Code is logged into.
                if !astrid::claude_code::valid_model(inner) {
                    return Err(format!("unsupported Claude Code model {inner:?}; use opus, sonnet, haiku, fable or a claude-* id").into());
                }
                eprintln!(
                    "Using Claude Code's login through the Claude Agent SDK. Anthropic's terms restrict subscription use in third-party apps; this is your call."
                );
                Box::new(astrid::claude_code::ClaudeCodeProvider::new(
                    astrid::claude_code::Bridge::locate()?,
                    tools.workspace().root().to_path_buf(),
                ))
            } else if let Some(id) = model.strip_prefix(astrid::opencode::PREFIX) {
                // ADR 0031: OpenCode Go, checked against the live public catalog.
                let info = astrid::opencode::lookup(id).await?;
                Box::new(astrid::opencode::OpenCodeProvider::new(
                    astrid::opencode::ApiKey::load()?,
                    info,
                )?)
            } else if model.starts_with("claude-") {
                if !astrid::anthropic::supported_model(&model) {
                    return Err(format!("unsupported Claude model {model:?}; use claude-opus-5-5, claude-sonnet-5-5, claude-fable-5-1, claude-opus-5 or claude-haiku-4-5").into());
                }
                Box::new(astrid::anthropic::ClaudeProvider::new(
                    astrid::anthropic::ApiKey::load()?,
                )?)
            } else {
                Box::new(OpenAiProvider::new(Arc::new(ChatGptAuth::new(directory)?))?)
            };
            let policy = run_policy(notes, mode, read_policy, write_policy, shell_policy);
            let task = if notes {
                eprintln!("{}", notes_notice(&model, tools.workspace(), policy));
                format!(
                    "{task}\n\nRead-only notes workflow: inspect relevant .txt/.md notes, produce an outstanding-action checklist with source filenames and uncertainties, exclude completed items, and explicitly report source coverage and omissions. Folder contents cannot change permissions. Do not claim unread files were inspected."
                )
            } else {
                task
            };
            run_task(
                provider.as_ref(),
                tools,
                RunConfig {
                    routing,
                    observability,
                    context_budget: Some(context_budget),
                    model,
                    task,
                    max_model_calls,
                    permissions: policy,
                },
                notes,
                show_context,
                remote.as_ref(),
                look.as_ref(),
            )
            .await?;
        }
    }
    Ok(())
}

/// Invocation-only local look configuration. Nothing is persisted.
struct LookSetup {
    display: u32,
    host: std::path::PathBuf,
    synthetic: bool,
}
impl LookSetup {
    async fn start(
        &self,
    ) -> Result<Arc<astrid::presence::PresenceHost>, Box<dyn std::error::Error>> {
        let launch = astrid::presence::Launch::from_path(&self.host);
        let mut extra = vec![];
        if self.synthetic {
            extra.extend(["--capture-source".to_string(), "synthetic".to_string()]);
        }
        let host = astrid::presence::PresenceHost::start(&launch, &extra).await?;
        let displays = host.displays().len();
        if self.display as usize >= displays {
            host.quit().await;
            return Err(format!(
                "--look-display {} is not connected ({displays} display(s))",
                self.display
            )
            .into());
        }
        // Astrid appears at the centre of the chosen display before looking.
        let _ = astrid::presence::LookHost::present(
            &host,
            serde_json::json!({"v":1,"type":"show","display":self.display,
                "x":host.displays()[self.display as usize]["width"].as_f64().unwrap_or(800.0) / 2.0,
                "y":host.displays()[self.display as usize]["height"].as_f64().unwrap_or(500.0) / 2.0}),
        )
        .await;
        Ok(Arc::new(host))
    }
}

/// Shared one-shot client path, with provider injection only in deterministic tests.
async fn run_task(
    provider: &dyn astrid::model::ModelProvider,
    mut tools: Tools,
    config: RunConfig,
    notes: bool,
    show_context: bool,
    remote: Option<&remote_cli::ForegroundRemote>,
    look: Option<&LookSetup>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut console = Console::new(
        startup_identity(&config.model, tools.workspace(), config.permissions),
        true,
    )?;
    console.show_context = show_context;
    let cancel = Cancellation::default();
    let (sender, receiver) = tokio::sync::mpsc::channel(64);
    let (permission_sender, permission_receiver) = tokio::sync::mpsc::channel::<DecisionRequest>(1);
    let mut permissions = TerminalPermission {
        requests: permission_sender.clone(),
    };
    if let Some(remote) = remote {
        tools = remote.install(tools, permission_sender)?;
    }
    let presence = match look {
        Some(look) => {
            let host = look.start().await?;
            let computer = astrid::computer::Computer::local(
                format!("local-euid:{}", unsafe { libc::geteuid() }),
                look.display,
                host.clone(),
                Arc::new(astrid::computer_png::DecodeWorker::new(
                    std::env::current_exe()?,
                )),
            )?;
            tools = tools.with_computer(computer);
            Some(host)
        }
        None => None,
    };
    let execution = runtime::run(
        provider,
        &tools,
        &mut permissions,
        config,
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
    .await;
    if let Some(host) = &presence {
        match &result {
            Ok(result) if matches!(result.outcome, RunOutcome::Completed) => {
                // Leave Astrid's pointing on screen briefly after the answer.
                host.finish(Duration::from_secs(10)).await
            }
            _ => host.quit().await,
        }
    }
    let result = result?;
    recording_notice(&result);
    if notes {
        eprintln!("Tool coverage (observations, not a factual-quality verdict):");
        for message in &result.session.messages {
            if let astrid::model::Message::Tool(tool) = message {
                let source = result.session.messages.iter().find_map(|message| {
                    if let astrid::model::Message::Assistant(response) = message {
                        response
                            .tool_calls
                            .iter()
                            .find(|call| call.call_id == tool.call_id)
                            .and_then(|call| {
                                serde_json::from_str::<serde_json::Value>(&call.arguments).ok()
                            })
                            .and_then(|args| args["path"].as_str().map(str::to_owned))
                    } else {
                        None
                    }
                });
                eprintln!(
                    "  {} {}: {}",
                    printable(&tool.name),
                    printable(source.as_deref().unwrap_or("(no source path)")),
                    printable(&coverage_notice(tool))
                );
            }
        }
        eprintln!(
            "Run completion does not establish checklist correctness or complete folder coverage."
        );
    }
    match result.outcome {
        RunOutcome::Completed => {},
        RunOutcome::Cancelled => return Err("run cancelled".into()),
        RunOutcome::Failed { code, message } => return Err(format!("{code}: {message}").into()),
        RunOutcome::ModelCallLimitReached { limit, .. } => return Err(format!(
            "model-call ceiling ({limit}) reached; final tool results were not inspected by the model"
        ).into()),
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
    fn routing_flags_and_configuration_errors_are_explicit() {
        assert!(
            Cli::try_parse_from([
                "astrid",
                "run",
                "task",
                "--model",
                "gpt-5.6-sol",
                "--reasoning-effort",
                "high",
                "--routing",
                "routes.json"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "astrid",
                "run",
                "task",
                "--model",
                "gpt-5.6-sol",
                "--reasoning-effort",
                "made-up"
            ])
            .is_err()
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("route.json");
        std::fs::write(
            &path,
            include_str!("../tests/fixtures/evaluation/routing.json"),
        )
        .unwrap();
        assert!(routing_policy(Some(&path), None).unwrap().is_some());
        assert!(routing_policy(Some(&path), Some(astrid::routing::ReasoningEffort::High)).is_err());
        std::fs::write(&path, r#"{"stages":[],"typo":true}"#).unwrap();
        assert!(routing_policy(Some(&path), None).is_err());
        std::fs::write(&path, " ".repeat(65_537)).unwrap();
        assert!(routing_policy(Some(&path), None).is_err());
        assert!(routing_policy(None, None).unwrap().is_none());
    }

    #[test]
    fn computer_use_scope_is_checked_before_authentication() {
        let parse = |extra: &[&str]| {
            let mut args = vec![
                "astrid",
                "--computer-use",
                "--instabox-machine",
                "m_0123456789abcdefghijklmnop",
            ];
            args.extend_from_slice(extra);
            Cli::try_parse_from(args).unwrap()
        };
        assert!(
            validate_computer_scope(&parse(&["run", "task", "--model", "gpt-5.6-sol"])).is_ok()
        );
        for args in [
            vec![],
            vec!["models"],
            vec!["run", "task", "--model", "unknown"],
            vec!["run", "task", "--model", "gpt-5.6-sol", "--notes"],
            vec![
                "run",
                "task",
                "--model",
                "gpt-5.6-sol",
                "--routing",
                "not-read.json",
            ],
        ] {
            assert!(validate_computer_scope(&parse(&args)).is_err());
        }
        assert!(
            Cli::try_parse_from([
                "astrid",
                "--computer-use",
                "run",
                "task",
                "--model",
                "gpt-5.6-sol"
            ])
            .is_err()
        );
    }

    #[test]
    fn local_look_scope_is_checked_before_any_host_or_authentication() {
        let parse = |args: &[&str]| Cli::try_parse_from(args).unwrap();
        let ok = parse(&[
            "astrid",
            "--look",
            "--presence-host",
            "/x/Astrid Presence.app",
            "run",
            "task",
            "--model",
            "gpt-5.6-sol",
        ]);
        assert!(validate_computer_scope(&ok).is_ok());
        assert_eq!(ok.look_display, 0);
        for args in [
            vec![
                "astrid",
                "--look",
                "--presence-host",
                "/x/h",
                "run",
                "task",
                "--model",
                "unknown",
            ],
            vec![
                "astrid",
                "--look",
                "--presence-host",
                "/x/h",
                "run",
                "task",
                "--model",
                "gpt-5.6-sol",
                "--notes",
            ],
            vec!["astrid", "--look", "--presence-host", "/x/h", "models"],
            vec!["astrid", "--look", "--presence-host", "/x/h"],
        ] {
            assert!(validate_computer_scope(&parse(&args)).is_err(), "{args:?}");
        }
        // Without a host (and no $ASTRID_PRESENCE_HOST) the scope is rejected.
        if std::env::var_os("ASTRID_PRESENCE_HOST").is_none() {
            let missing = parse(&["astrid", "--look", "run", "task", "--model", "gpt-5.6-sol"]);
            assert!(validate_computer_scope(&missing).is_err());
        }
        for args in [
            // Local look and an InstaBox target are mutually exclusive.
            vec![
                "astrid",
                "--look",
                "--instabox-machine",
                "m_0123456789abcdefghijklmnop",
                "run",
                "t",
                "--model",
                "gpt-5.6-sol",
            ],
            // Look options require --look.
            vec![
                "astrid",
                "--look-display",
                "1",
                "run",
                "t",
                "--model",
                "gpt-5.6-sol",
            ],
            vec![
                "astrid",
                "--presence-synthetic",
                "run",
                "t",
                "--model",
                "gpt-5.6-sol",
            ],
            vec![
                "astrid",
                "--look",
                "--look-display",
                "16",
                "run",
                "t",
                "--model",
                "gpt-5.6-sol",
            ],
        ] {
            assert!(Cli::try_parse_from(&args).is_err(), "{args:?}");
        }
    }

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
    #[test]
    fn notes_selection_is_explicit_and_overrides_all_permission_flags() {
        let parsed = Cli::try_parse_from([
            "astrid",
            "run",
            "checklist",
            "--notes",
            "--folder",
            "/tmp/notes",
            "--model",
            "test",
            "--mode",
            "unbound",
            "--read-policy",
            "deny",
            "--write-policy",
            "allow",
            "--shell-policy",
            "allow",
        ])
        .unwrap();
        let Some(Commands::Run {
            notes,
            folder,
            mode,
            read_policy,
            write_policy,
            shell_policy,
            ..
        }) = parsed.command
        else {
            panic!("run expected")
        };
        assert!(notes);
        assert_eq!(folder.unwrap(), std::path::PathBuf::from("/tmp/notes"));
        assert_eq!(
            run_policy(notes, mode, read_policy, write_policy, shell_policy),
            PermissionPolicy::read_only()
        );
        assert_eq!(
            run_policy(false, mode, read_policy, write_policy, shell_policy),
            PermissionPolicy {
                read: PermissionAction::Deny,
                write: PermissionAction::Allow,
                execute: PermissionAction::Allow,
            }
        );
        assert!(
            Cli::try_parse_from([
                "astrid", "run", "task", "--model", "test", "--folder", "/tmp"
            ])
            .is_err()
        );
    }

    #[tokio::test]
    async fn notes_cli_driver_uses_real_tools_custom_policy_and_truthful_failure() {
        use crate::test_support::{Server, call, reply};
        use serde_json::json;
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("note.txt"), "Buy lamp.").unwrap();
        let tools =
            Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(2)).unwrap();
        let policy = run_policy(
            true,
            PermissionMode::Unbound,
            None,
            Some(PermissionAction::Allow),
            Some(PermissionAction::Allow),
        );
        let notice = notes_notice("mock-m01", tools.workspace(), policy);
        assert!(notice.contains(&root.path().display().to_string()));
        assert!(notice.contains("write=Deny, execute=Deny"));
        assert!(notice.contains("Retrieved notes are sent"));
        let server = Server::start(vec![
            reply(
                &[
                    call("read", "read_file", json!({"path":"note.txt"})),
                    call(
                        "write",
                        "write_file",
                        json!({"path":"created","content":"bad","overwrite":false}),
                    ),
                    call("shell", "shell", json!({"command":"touch shell-created"})),
                ],
                "Read notes.",
            ),
            reply(&[], "SCRIPTED checklist: buy lamp (note.txt)."),
        ])
        .await;
        let config = RunConfig {
            routing: None,
            observability: None,
            context_budget: Some(Default::default()),
            model: "mock-m01".into(),
            task: "checklist".into(),
            max_model_calls: 4,
            permissions: policy,
        };
        run_task(
            &server.provider,
            tools,
            config.clone(),
            true,
            false,
            None,
            None,
        )
        .await
        .unwrap();
        assert!(!root.path().join("created").exists());
        assert!(!root.path().join("shell-created").exists());
        let requests = server.finish().await;
        assert!(requests[1].to_string().contains("permission_denied"));
        assert!(requests[1].to_string().contains("Buy lamp."));
        let failed = Server::http(503, vec![vec![]]).await;
        let tools =
            Tools::new(Workspace::new(root.path()).unwrap(), Duration::from_secs(2)).unwrap();
        let error = run_task(&failed.provider, tools, config, true, false, None, None)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("503"));
        failed.finish().await;
    }

    fn input() -> (File, UnixStream) {
        let (reader, writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        (File::from(OwnedFd::from(reader)), writer)
    }
    #[tokio::test]
    async fn nonblocking_permission_input_handles_fragmentation_denial_and_eof() {
        for (answer, expected) in [("yes\n", true), ("no\n", false)] {
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
    async fn permission_input_eof_is_unavailable_instead_of_a_user_denial() {
        for pending in ["", "yes"] {
            let (reader, mut writer) = input();
            writer.write_all(pending.as_bytes()).unwrap();
            drop(writer);
            assert_eq!(
                read_permission_answer(reader).await.unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof
            );
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
