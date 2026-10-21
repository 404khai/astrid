//! Standalone CLI and local credential loading; never part of the model adapter.
use astrid::{cancellation::Cancellation, instabox::*};
use clap::Subcommand;
use std::{io::Read, os::unix::fs::OpenOptionsExt, path::Path, time::Duration};

#[derive(Subcommand)]
pub enum Command {
    /// Read one page of machine metadata (or one explicit ID) and account usage.
    Discover {
        /// Separate network discovery grant; local shell/read policy does not grant it.
        #[arg(long)]
        allow_discovery: bool,
        /// Exact m_ ID; names, auto-selection, connection and execution are unsupported.
        #[arg(long)]
        machine: Option<String>,
    },
    /// Execute one exact command on one existing running machine.
    Exec {
        /// Exact existing machine ID. Machines are never created, started, resumed or selected implicitly.
        #[arg(long)]
        machine: String,
        /// Exact command string sent to InstaBox.
        #[arg(long)]
        command: String,
        /// Remote working directory.
        #[arg(long, default_value = "/data/home")]
        cwd: String,
        /// Provider-side command timeout in seconds (1..=600).
        #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u64).range(1..=600))]
        timeout_seconds: u64,
        /// Explicit grant for this exact request, including possible provider auto-wake after the status check.
        #[arg(long)]
        allow_execution: bool,
    },
}

// Only the invocation directory's key assignment is read. Never source shell code,
// expand variables, search parent directories, or alter the process environment.
fn load_key(environment: Option<String>, path: &Path) -> Result<String, Error> {
    if let Some(key) = environment.filter(|value| !value.trim().is_empty()) {
        return Ok(key);
    }
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::MissingKey);
        }
        Err(_) => return Err(Error::InvalidEnvFile),
    };
    if !file
        .metadata()
        .map_err(|_| Error::InvalidEnvFile)?
        .is_file()
    {
        return Err(Error::InvalidEnvFile);
    }
    let mut bytes = Vec::new();
    file.take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::InvalidEnvFile)?;
    if bytes.len() > 16_384 {
        return Err(Error::InvalidEnvFile);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| Error::InvalidEnvFile)?;
    let mut key = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
        let Some((name, value)) = line.split_once('=') else {
            if line.starts_with("INSTABOX_API_KEY") {
                return Err(Error::InvalidEnvFile);
            }
            continue;
        };
        if name.trim() != "INSTABOX_API_KEY" {
            continue;
        }
        if key.is_some() {
            return Err(Error::InvalidEnvFile);
        }
        let value = value.trim();
        let value = if value.starts_with(['\'', '"']) {
            let quote = value.as_bytes()[0] as char;
            if value.len() < 2 || !value.ends_with(quote) {
                return Err(Error::InvalidEnvFile);
            }
            &value[1..value.len() - 1]
        } else {
            value
        };
        key = Some(value.to_owned());
    }
    key.filter(|value| !value.is_empty())
        .ok_or(Error::MissingKey)
}

pub async fn run(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Command::Discover {
            allow_discovery,
            machine,
        } => {
            // Deny before consulting environment or .env; never read provider/CLI credentials.
            if !allow_discovery {
                return Err(Error::Denied.into());
            }
            if machine.as_deref().is_some_and(|id| !valid_id(id)) {
                return Err(Error::InvalidId.into());
            }
            let key = invocation_key()?;
            let client = DiscoveryClient::new(&key)?;
            let cancel = Cancellation::default();
            let result = tokio::select! {
                biased;
                _ = tokio::signal::ctrl_c() => { cancel.cancel(); Err(Error::Cancelled) },
                result = client.discover(true, machine.as_deref(), &cancel, Duration::from_secs(15)) => result,
            }?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Exec {
            machine,
            command,
            cwd,
            timeout_seconds,
            allow_execution,
        } => {
            let request = ExecGrant {
                machine_id: machine,
                command,
                cwd,
                timeout_seconds,
            };
            request.validate()?;
            // Deny before reading credentials or making even a metadata request.
            if !allow_execution {
                return Err(Error::ExecDenied.into());
            }
            let key = invocation_key()?;
            let client = DiscoveryClient::new(&key)?;
            let cancel = Cancellation::default();
            let granted = request.clone();
            let execution = client.execute(Some(&granted), &request, &cancel);
            tokio::pin!(execution);
            let result = tokio::select! {
                biased;
                result = &mut execution => result?,
                _ = tokio::signal::ctrl_c() => {
                    cancel.cancel();
                    execution.await?
                },
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
            if result.status != ExecStatus::Completed
                || result.timed_out
                || result.exit_status != Some(0)
            {
                return Err(Error::ExecUnsuccessful.into());
            }
        }
    }
    Ok(())
}

pub(crate) fn invocation_key() -> Result<String, Error> {
    let environment = match std::env::var("INSTABOX_API_KEY") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err(Error::InvalidKey),
    };
    load_key(environment, Path::new(".env"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dotenv_key_loading_is_local_literal_and_environment_takes_precedence() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(".env");
        assert_eq!(load_key(None, &path).err(), Some(Error::MissingKey));
        for assignment in [
            "INSTABOX_API_KEY=ibk_synthetic",
            "export INSTABOX_API_KEY = 'ibk_synthetic'",
            "INSTABOX_API_KEY=\"ibk_synthetic\"",
        ] {
            std::fs::write(&path, format!("# comment\nOTHER=ignored\n{assignment}\r\n")).unwrap();
            assert_eq!(load_key(None, &path).unwrap(), "ibk_synthetic");
            assert_eq!(
                load_key(Some(String::new()), &path).unwrap(),
                "ibk_synthetic"
            );
            assert_eq!(
                load_key(Some("ibk_environment".into()), &path).unwrap(),
                "ibk_environment"
            );
        }
        std::fs::write(&path, "INSTABOX_API_KEY=$(touch forbidden)\n").unwrap();
        assert_eq!(load_key(None, &path).unwrap(), "$(touch forbidden)");
        assert!(!root.path().join("forbidden").exists());
        assert_eq!(
            load_key(None, &root.path().join("child/.env")).err(),
            Some(Error::MissingKey)
        );
    }

    #[test]
    fn dotenv_failures_do_not_echo_secrets_or_follow_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(".env");
        for text in [
            "INSTABOX_API_KEY='SECRET",
            "INSTABOX_API_KEY=SECRET\nINSTABOX_API_KEY=duplicate",
            "INSTABOX_API_KEY SECRET",
        ] {
            std::fs::write(&path, text).unwrap();
            let error = load_key(None, &path).unwrap_err();
            assert_eq!(error, Error::InvalidEnvFile);
            assert!(!format!("{error:?} {error}").contains("SECRET"));
        }
        std::fs::write(&path, "x".repeat(16_385)).unwrap();
        assert_eq!(load_key(None, &path).err(), Some(Error::InvalidEnvFile));
        std::fs::write(&path, [0xff]).unwrap();
        assert_eq!(load_key(None, &path).err(), Some(Error::InvalidEnvFile));
        std::fs::write(&path, "INSTABOX_API_KEY=\n").unwrap();
        assert_eq!(load_key(None, &path).err(), Some(Error::MissingKey));
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert_eq!(load_key(None, &link).err(), Some(Error::InvalidEnvFile));
        assert_eq!(
            load_key(None, root.path()).err(),
            Some(Error::InvalidEnvFile)
        );
    }
}
