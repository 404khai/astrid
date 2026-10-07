use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};

use crate::{
    cancellation::Cancellation,
    model::{ToolCall, ToolOutcome, ToolResult},
    output::{self, Capture, OutputStream, ToolOutput},
    workspace::Workspace,
};

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool execution cancelled")]
    Cancelled,
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("workspace path denied: {0}")]
    PathDenied(String),
    #[error("edit requires exactly one match; found {0}")]
    EditAmbiguous(usize),
    #[error("shell cleanup failed: {0}")]
    CleanupFailed(String),
    #[error("{0}")]
    Io(#[from] io::Error),
}

impl ToolError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::InvalidArguments(_) => "invalid_arguments",
            Self::PathDenied(_) => "path_denied",
            Self::EditAmbiguous(_) => "edit_ambiguous",
            Self::CleanupFailed(_) => "cleanup_failed",
            Self::Io(_) => "io_error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub command: String,
    pub workspace: PathBuf,
}

#[derive(Debug)]
pub enum ToolExecution {
    Finished(ToolResult),
    Cancelled,
    CancelledWithOutput { data: Value },
    CleanupFailed(String),
}

/// Execution seam: permission is coordinated by the runtime, never the tool.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    fn workspace(&self) -> &Workspace;
    fn permission(&self, call: &ToolCall) -> Result<Option<PermissionRequest>, ToolError>;
    async fn execute(&self, call: &ToolCall, cancellation: &Cancellation) -> ToolExecution;
    async fn execute_stream(
        &self,
        call: &ToolCall,
        cancellation: &Cancellation,
        _output: Option<tokio::sync::mpsc::Sender<ToolOutput>>,
    ) -> ToolExecution {
        self.execute(call, cancellation).await
    }
}

pub struct Tools {
    workspace: Workspace,
    shell_timeout: Duration,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WriteArgs {
    path: String,
    content: String,
    overwrite: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditArgs {
    path: String,
    old_text: String,
    new_text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShellArgs {
    command: String,
}

fn parse<T: serde::de::DeserializeOwned>(call: &ToolCall) -> Result<T, ToolError> {
    serde_json::from_str(&call.arguments)
        .map_err(|error| ToolError::InvalidArguments(error.to_string()))
}

impl Tools {
    pub fn new(workspace: Workspace, shell_timeout: Duration) -> Result<Self, ToolError> {
        if shell_timeout.is_zero() {
            return Err(ToolError::InvalidArguments(
                "shell timeout must be positive".into(),
            ));
        }
        Ok(Self {
            workspace,
            shell_timeout,
        })
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    async fn invoke(
        &self,
        call: &ToolCall,
        cancellation: &Cancellation,
    ) -> Result<Value, ToolError> {
        match call.name.as_str() {
            "read_file" | "list_directory" | "glob" | "grep" => {
                let workspace = self.workspace.clone();
                let call = call.clone();
                let cancellation = cancellation.clone();
                tokio::task::spawn_blocking(move || {
                    crate::native::invoke(&workspace, &call, &cancellation)
                })
                .await
                .map_err(|error| ToolError::Io(io::Error::other(error)))?
            }
            "write_file" => {
                let args: WriteArgs = parse(call)?;
                let path = self.workspace.resolve(&args.path)?;
                atomic_write(&path, &args.content, args.overwrite)?;
                Ok(json!({"path": args.path, "bytes_written": args.content.len()}))
            }
            "edit_file" => {
                let args: EditArgs = parse(call)?;
                if args.old_text.is_empty() {
                    return Err(ToolError::InvalidArguments(
                        "old_text must not be empty".into(),
                    ));
                }
                let path = self.workspace.resolve(&args.path)?;
                regular_file(&path)?;
                let content = fs::read_to_string(&path)?;
                // Count overlapping occurrences too: `aa` in `aaa` is ambiguous.
                let matches = content
                    .char_indices()
                    .filter(|(index, _)| content[*index..].starts_with(&args.old_text))
                    .count();
                if matches != 1 {
                    return Err(ToolError::EditAmbiguous(matches));
                }
                let edited = content.replacen(&args.old_text, &args.new_text, 1);
                atomic_write(&path, &edited, true)?;
                Ok(json!({"path": args.path, "replacements": 1, "bytes_written": edited.len()}))
            }
            "shell" => Err(ToolError::InvalidArguments(
                "shell uses cancellable execution".into(),
            )),
            _ => Err(ToolError::InvalidArguments(format!(
                "unknown tool {}",
                call.name
            ))),
        }
    }

    async fn shell(
        &self,
        command: &str,
        cancellation: &Cancellation,
        sender: Option<tokio::sync::mpsc::Sender<ToolOutput>>,
    ) -> Result<Value, ToolError> {
        let mut process = Command::new("/bin/sh");
        process
            .arg("-c")
            .arg(command)
            .current_dir(self.workspace.root())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_remove("OPENAI_API_KEY")
            .kill_on_drop(true)
            .process_group(0);
        let mut child = process.spawn()?;
        let pid = child
            .id()
            .ok_or_else(|| io::Error::other("shell has no process ID"))?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("shell stdout missing"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("shell stderr missing"))?;
        let mut out = Capture::default();
        let mut err = Capture::default();
        let mut out_buffer = [0u8; output::CHUNK_BYTES];
        let mut err_buffer = [0u8; output::CHUNK_BYTES];
        let mut out_done = false;
        let mut err_done = false;
        let mut status = None;
        let mut live_stopped = false;
        let mut cancelled = false;
        let mut timed_out = false;
        let deadline = tokio::time::sleep(self.shell_timeout);
        tokio::pin!(deadline);
        // No reader tasks or awaited output sends: execution and cleanup remain
        // polled independently of the runtime's lossless event delivery.
        let error = loop {
            if cancellation.is_cancelled() {
                cancelled = true;
                break None;
            }
            if out_done && err_done && status.is_some() {
                break None;
            }
            tokio::select! {
                _ = cancellation.cancelled() => { cancelled = true; break None; }
                _ = &mut deadline => { timed_out = true; break None; }
                read = stdout.read(&mut out_buffer), if !out_done => match read {
                    Ok(0) => out_done = true,
                    Ok(count) => out.observe(&out_buffer[..count], OutputStream::Stdout, sender.as_ref(), &mut live_stopped),
                    Err(error) => break Some(error),
                },
                read = stderr.read(&mut err_buffer), if !err_done => match read {
                    Ok(0) => err_done = true,
                    Ok(count) => err.observe(&err_buffer[..count], OutputStream::Stderr, sender.as_ref(), &mut live_stopped),
                    Err(error) => break Some(error),
                },
                waited = child.wait(), if status.is_none() => match waited {
                    Ok(value) => status = Some(value),
                    Err(error) => break Some(error),
                }
            }
        };
        if cancelled || timed_out || error.is_some() {
            let killed = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
            if killed != 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(ToolError::CleanupFailed(
                    io::Error::last_os_error().to_string(),
                ));
            }
            if status.is_none() {
                status = Some(
                    tokio::time::timeout(Duration::from_secs(5), child.wait())
                        .await
                        .map_err(|_| {
                            ToolError::CleanupFailed("shell leader reap timed out".into())
                        })?
                        .map_err(|error| ToolError::CleanupFailed(error.to_string()))?,
                );
            }
        }
        if let Some(error) = error {
            return Err(error.into());
        }
        let mut data = output::result(
            &out,
            &err,
            status.and_then(|value| value.code()),
            timed_out,
            cancelled,
            out_done && err_done,
            sender.is_some(),
        );
        if timed_out {
            data["timeout_seconds"] = json!(self.shell_timeout.as_secs_f64());
        }
        Ok(data)
    }

    async fn run_execute(
        &self,
        call: &ToolCall,
        cancellation: &Cancellation,
        sender: Option<tokio::sync::mpsc::Sender<ToolOutput>>,
    ) -> ToolExecution {
        if cancellation.is_cancelled() {
            return ToolExecution::Cancelled;
        }
        let result = if call.name == "shell" {
            match parse::<ShellArgs>(call) {
                Ok(args) if !args.command.trim().is_empty() => {
                    self.shell(&args.command, cancellation, sender).await
                }
                Ok(_) => Err(ToolError::InvalidArguments(
                    "command must not be empty".into(),
                )),
                Err(error) => Err(error),
            }
        } else {
            self.invoke(call, cancellation).await
        };
        let outcome = match result {
            Ok(data) if data["cancelled"] == true => {
                return ToolExecution::CancelledWithOutput { data };
            }
            Ok(data) if data["timed_out"] == true => ToolOutcome::TimedOut { data },
            Ok(data) => ToolOutcome::Success { data },
            Err(ToolError::Cancelled) => return ToolExecution::Cancelled,
            Err(error @ ToolError::CleanupFailed(_)) => {
                return ToolExecution::CleanupFailed(error.to_string());
            }
            Err(error) => ToolOutcome::Error {
                code: error.code().into(),
                message: error.to_string(),
            },
        };
        ToolExecution::Finished(ToolResult {
            call_id: call.call_id.clone(),
            name: call.name.clone(),
            outcome,
        })
    }
}

#[async_trait]
impl ToolExecutor for Tools {
    fn workspace(&self) -> &Workspace {
        &self.workspace
    }
    fn permission(&self, call: &ToolCall) -> Result<Option<PermissionRequest>, ToolError> {
        if call.name != "shell" {
            return Ok(None);
        }
        let args: ShellArgs = parse(call)?;
        if args.command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "command must not be empty".into(),
            ));
        }
        Ok(Some(PermissionRequest {
            command: args.command,
            workspace: self.workspace.root().to_owned(),
        }))
    }
    async fn execute(&self, call: &ToolCall, cancellation: &Cancellation) -> ToolExecution {
        self.run_execute(call, cancellation, None).await
    }
    async fn execute_stream(
        &self,
        call: &ToolCall,
        cancellation: &Cancellation,
        output: Option<tokio::sync::mpsc::Sender<ToolOutput>>,
    ) -> ToolExecution {
        self.run_execute(call, cancellation, output).await
    }
}

fn regular_file(path: &Path) -> Result<fs::Metadata, ToolError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(ToolError::PathDenied(path.display().to_string()));
    }
    Ok(metadata)
}

fn atomic_write(path: &Path, content: &str, overwrite: bool) -> Result<(), ToolError> {
    let existing = match fs::symlink_metadata(path) {
        Ok(_) => Some(regular_file(path)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    if existing.is_some() && !overwrite {
        return Err(ToolError::InvalidArguments(
            "file exists; overwrite must be explicitly true".into(),
        ));
    }
    if existing
        .as_ref()
        .is_some_and(|metadata| metadata.nlink() > 1)
    {
        return Err(ToolError::PathDenied(
            "mutating hard-linked files is denied".into(),
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| ToolError::PathDenied(path.display().to_string()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    if let Some(metadata) = existing {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())?;
    }
    temporary.write_all(content.as_bytes())?;
    temporary.as_file().sync_all()?;
    if overwrite {
        temporary.persist(path).map_err(|e| e.error)?;
    } else {
        temporary.persist_noclobber(path).map_err(|e| e.error)?;
    }
    Ok(())
}

/// The same schemas are used for the provider and for documenting the boundary.
pub fn definitions() -> Vec<Value> {
    let string = || json!({"type":"string"});
    let specs = [
        (
            "read_file",
            "Read a UTF-8 file within the workspace.",
            json!({"path":string()}),
        ),
        (
            "write_file",
            "Create or overwrite a UTF-8 file. Parent directory must exist; overwriting requires overwrite=true.",
            json!({"path":string(),"content":string(),"overwrite":{"type":"boolean"}}),
        ),
        (
            "edit_file",
            "Replace exact old_text with new_text only if it occurs exactly once.",
            json!({"path":string(),"old_text":string(),"new_text":string()}),
        ),
        (
            "list_directory",
            "List immediate entries; use path=\".\" for the workspace root.",
            json!({"path":string()}),
        ),
        (
            "glob",
            "Find files using a workspace-relative glob, for example **/*.rs. Skips .git and symlinks.",
            json!({"pattern":string()}),
        ),
        (
            "grep",
            "Search UTF-8 files using a Rust regular expression. Path may name a file or directory; skips .git, binary files, and symlinks.",
            json!({"path":string(),"pattern":string()}),
        ),
        (
            "shell",
            "Run a noninteractive /bin/sh command subject to the configured execution permission policy (default: ask). Fresh workspace cwd; returns bounded stdout, stderr, exit_code, timeout status, and explicit output coverage.",
            json!({"command":string()}),
        ),
    ];
    specs.into_iter().map(|(name,description,properties)| {
        let required = properties.as_object().expect("literal schema object").keys().cloned().collect::<Vec<_>>();
        json!({"type":"function","name":name,"description":description,"strict":true,"parameters":{"type":"object","properties":properties,"required":required,"additionalProperties":false}})
    }).collect()
}
