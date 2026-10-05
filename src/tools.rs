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
use walkdir::WalkDir;

use crate::{
    model::{ToolCall, ToolOutcome, ToolResult},
    workspace::Workspace,
};

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("workspace path denied: {0}")]
    PathDenied(String),
    #[error("edit requires exactly one match; found {0}")]
    EditAmbiguous(usize),
    #[error("shell command was not approved")]
    PermissionDenied,
    #[error("{0}")]
    Io(#[from] io::Error),
}

impl ToolError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidArguments(_) => "invalid_arguments",
            Self::PathDenied(_) => "path_denied",
            Self::EditAmbiguous(_) => "edit_ambiguous",
            Self::PermissionDenied => "permission_denied",
            Self::Io(_) => "io_error",
        }
    }
}

/// Every shell invocation requires its own explicit decision.
#[async_trait]
pub trait ShellConfirmation: Send {
    async fn confirm(&mut self, command: &str, workspace: &Path) -> io::Result<bool>;
}

pub struct Tools {
    workspace: Workspace,
    shell_timeout: Duration,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
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
struct GlobArgs {
    pattern: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrepArgs {
    path: String,
    pattern: String,
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

    pub async fn execute(
        &self,
        call: &ToolCall,
        confirmation: &mut dyn ShellConfirmation,
    ) -> ToolResult {
        let outcome = match self.invoke(call, confirmation).await {
            Ok(data) => ToolOutcome::Success { data },
            Err(error) => ToolOutcome::Error {
                code: error.code().into(),
                message: error.to_string(),
            },
        };
        ToolResult {
            call_id: call.call_id.clone(),
            name: call.name.clone(),
            outcome,
        }
    }

    async fn invoke(
        &self,
        call: &ToolCall,
        confirmation: &mut dyn ShellConfirmation,
    ) -> Result<Value, ToolError> {
        match call.name.as_str() {
            "read_file" => {
                let args: PathArgs = parse(call)?;
                let path = self.workspace.resolve(&args.path)?;
                regular_file(&path)?;
                Ok(json!({"path": args.path, "content": fs::read_to_string(path)?}))
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
            "list_directory" => {
                let args: PathArgs = parse(call)?;
                let path = self.workspace.resolve(&args.path)?;
                let mut entries = fs::read_dir(path)?.map(|entry| {
                    let entry = entry?;
                    let kind = entry.file_type()?;
                    Ok(json!({"name": entry.file_name().to_string_lossy(), "kind": if kind.is_symlink() { "symlink" } else if kind.is_dir() { "directory" } else if kind.is_file() { "file" } else { "other" }}))
                }).collect::<Result<Vec<_>, io::Error>>()?;
                entries.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
                Ok(json!({"path": args.path, "entries": entries}))
            }
            "glob" => {
                let args: GlobArgs = parse(call)?;
                if Path::new(&args.pattern).is_absolute()
                    || args.pattern.split('/').any(|part| part == "..")
                {
                    return Err(ToolError::PathDenied(args.pattern));
                }
                let pattern = globset::GlobBuilder::new(&args.pattern)
                    .literal_separator(true)
                    .build()
                    .map_err(|e| ToolError::InvalidArguments(e.to_string()))?
                    .compile_matcher();
                let files = self
                    .files(self.workspace.root())?
                    .into_iter()
                    .map(|path| self.workspace.relative(&path))
                    .filter(|path| pattern.is_match(path))
                    .collect::<Vec<_>>();
                Ok(json!({"files": files}))
            }
            "grep" => {
                let args: GrepArgs = parse(call)?;
                let path = self.workspace.resolve(&args.path)?;
                let pattern = regex::Regex::new(&args.pattern)
                    .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
                let mut matches = Vec::new();
                for file in self.files(&path)? {
                    let bytes = fs::read(&file)?;
                    // Binary files are deliberately excluded from textual search.
                    if bytes.contains(&0) {
                        continue;
                    }
                    let Ok(content) = String::from_utf8(bytes) else {
                        continue;
                    };
                    for (index, line) in content.lines().enumerate() {
                        if pattern.is_match(line) {
                            matches.push(json!({"path": self.workspace.relative(&file), "line": index + 1, "text": line}));
                        }
                    }
                }
                Ok(json!({"matches": matches}))
            }
            "shell" => {
                let args: ShellArgs = parse(call)?;
                if args.command.trim().is_empty() {
                    return Err(ToolError::InvalidArguments(
                        "command must not be empty".into(),
                    ));
                }
                if !confirmation
                    .confirm(&args.command, self.workspace.root())
                    .await?
                {
                    return Err(ToolError::PermissionDenied);
                }
                self.shell(&args.command).await
            }
            _ => Err(ToolError::InvalidArguments(format!(
                "unknown tool {}",
                call.name
            ))),
        }
    }

    fn files(&self, path: &Path) -> Result<Vec<PathBuf>, ToolError> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.is_file() {
            return Ok(vec![path.to_path_buf()]);
        }
        if !metadata.is_dir() {
            return Err(ToolError::PathDenied(path.display().to_string()));
        }
        let mut files = Vec::new();
        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| entry.file_name() != ".git")
        {
            let entry = entry.map_err(|error| ToolError::Io(io::Error::other(error)))?;
            if entry.file_type().is_file() {
                // Revalidate every discovered path through the same boundary.
                files.push(
                    self.workspace
                        .resolve(&self.workspace.relative(entry.path()))?,
                );
            }
        }
        files.sort();
        Ok(files)
    }

    async fn shell(&self, command: &str) -> Result<Value, ToolError> {
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
        let out = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).await?;
            Ok::<_, io::Error>(bytes)
        });
        let err = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).await?;
            Ok::<_, io::Error>(bytes)
        });
        let waited = tokio::time::timeout(self.shell_timeout, async {
            let status = child.wait().await?;
            let stdout = out.await.map_err(io::Error::other)??;
            let stderr = err.await.map_err(io::Error::other)??;
            Ok::<_, io::Error>((status, stdout, stderr))
        })
        .await;
        match waited {
            Ok(result) => {
                let (status, stdout, stderr) = result?;
                Ok(
                    json!({"stdout": String::from_utf8_lossy(&stdout), "stderr": String::from_utf8_lossy(&stderr), "exit_code": status.code(), "timed_out": false}),
                )
            }
            Err(_) => {
                // The timeout covers pipe draining as well as the shell itself.
                // Descendants inherit this fresh process group unless they detach.
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
                let _ = child.kill().await;
                let _ = child.wait().await;
                Ok(
                    json!({"stdout": null, "stderr": null, "exit_code": null, "timed_out": true, "timeout_seconds": self.shell_timeout.as_secs_f64()}),
                )
            }
        }
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
            "Run a noninteractive /bin/sh command after explicit user confirmation. Fresh workspace cwd; returns stdout, stderr, exit_code, and timeout status.",
            json!({"command":string()}),
        ),
    ];
    specs.into_iter().map(|(name,description,properties)| {
        let required = properties.as_object().expect("literal schema object").keys().cloned().collect::<Vec<_>>();
        json!({"type":"function","name":name,"description":description,"strict":true,"parameters":{"type":"object","properties":properties,"required":required,"additionalProperties":false}})
    }).collect()
}
