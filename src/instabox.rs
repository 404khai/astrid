//! Optional InstaBox integration: explicit metadata discovery and permission-gated execution on an existing machine.
use astrid::cancellation::Cancellation;
use clap::Subcommand;
use reqwest::{Client, header::HeaderValue};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::{Duration, Instant},
};

const API: &str = "https://api.instabox.sh";
const MAX_BODY: usize = 262_144;

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

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum Error {
    #[error("InstaBox discovery denied; explicit --allow-discovery is required")]
    Denied,
    #[error(
        "set INSTABOX_API_KEY in .env or the environment to an InstaBox API key with machines:read for discovery or machines:write for explicitly granted execution"
    )]
    MissingKey,
    #[error(
        "cannot load InstaBox key from .env; use one INSTABOX_API_KEY=value assignment in a regular UTF-8 file (at most 16 KiB)"
    )]
    InvalidEnvFile,
    #[error("invalid InstaBox API key header")]
    InvalidKey,
    #[error("invalid machine ID; expected m_ followed by 26 lowercase letters or digits")]
    InvalidId,
    #[error("InstaBox authentication failed (expired, revoked or invalid key)")]
    Authentication,
    #[error("InstaBox access denied (scope, account activation or account policy)")]
    Forbidden,
    #[error("InstaBox machine unavailable or deleted")]
    NotFound,
    #[error("InstaBox quota exceeded; no automatic retry")]
    Quota,
    #[error("InstaBox service unavailable")]
    Unavailable,
    #[error("InstaBox request failed")]
    Transport,
    #[error("InstaBox response invalid or exceeds discovery bounds")]
    InvalidResponse,
    #[error("InstaBox request timed out")]
    Timeout,
    #[error("InstaBox request cancelled")]
    Cancelled,
    #[error("InstaBox execution denied; explicit --allow-execution is required")]
    ExecDenied,
    #[error(
        "InstaBox execution requires an existing machine in running state; Astrid will not start or resume it"
    )]
    MachineNotRunning,
    #[error("invalid execution request")]
    InvalidExecRequest,
    #[error("InstaBox execution did not succeed; inspect the JSON outcome (no automatic retry)")]
    ExecUnsuccessful,
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

fn valid_id(id: &str) -> bool {
    id.len() == 28
        && id.starts_with("m_")
        && id[2..]
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

#[derive(Deserialize, Serialize)]
struct Machine {
    id: String,
    state: String,
}
impl Machine {
    fn validate(&mut self) -> Result<(), Error> {
        if !valid_id(&self.id) {
            return Err(Error::InvalidResponse);
        }
        // Do not print arbitrary server strings, names, labels or unknown state text.
        if !matches!(
            self.state.as_str(),
            "creating"
                | "starting"
                | "running"
                | "stopping"
                | "stopped"
                | "suspending"
                | "suspended"
                | "resuming"
                | "deleting"
                | "error"
        ) {
            self.state = "unknown".into();
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    machines: Vec<Machine>,
    // Missing cursor must not be mistaken for proof of complete discovery.
    #[serde(deserialize_with = "serde_json::Value::deserialize")]
    next_cursor: serde_json::Value,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Limits {
    max_machines: u64,
    max_snapshots: u64,
    #[serde(rename = "maxStoredGiB")]
    max_stored_gi_b: u64,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Usage {
    machines: u64,
    snapshots: u64,
    stored_bytes: u64,
    limits: Limits,
    // Preserve absence of a storage measurement without printing free-form timestamps.
    #[serde(skip_serializing)]
    computed_at: Option<String>,
}
#[derive(Serialize)]
struct Discovery {
    target: &'static str,
    machines: Vec<Machine>,
    list_complete: bool,
    usage: Usage,
    storage_measurement_available: bool,
    elapsed_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ExecGrant {
    machine_id: String,
    command: String,
    cwd: String,
    timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecResponse {
    stdout: String,
    stderr: String,
    exit_code: i32,
    timed_out: bool,
    truncated: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ExecStatus {
    Completed,
    Cancelled,
    DeadlineExceeded,
    Failed,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RemoteOutcome {
    ObservedCompleted,
    Unknown,
}

#[derive(Debug, Serialize)]
struct ExecReport {
    target: &'static str,
    machine_id: String,
    status: ExecStatus,
    remote_outcome: RemoteOutcome,
    stdout: String,
    stderr: String,
    exit_status: Option<i32>,
    timed_out: bool,
    output_truncated: bool,
    output_available: bool,
    failure: Option<&'static str>,
    elapsed_ms: u128,
}

impl Error {
    fn category(&self) -> &'static str {
        match self {
            Self::Authentication => "authentication",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Quota => "quota",
            Self::Unavailable => "unavailable",
            Self::InvalidResponse => "invalid_response",
            _ => "transport",
        }
    }
}
impl ExecReport {
    fn unknown(
        request: &ExecGrant,
        status: ExecStatus,
        failure: &'static str,
        started: Instant,
    ) -> Self {
        Self {
            target: "instabox",
            machine_id: request.machine_id.clone(),
            status,
            remote_outcome: RemoteOutcome::Unknown,
            stdout: String::new(),
            stderr: String::new(),
            exit_status: None,
            timed_out: false,
            output_truncated: false,
            output_available: false,
            failure: Some(failure),
            elapsed_ms: started.elapsed().as_millis(),
        }
    }
}

impl ExecGrant {
    fn validate(&self) -> Result<(), Error> {
        if !valid_id(&self.machine_id) {
            return Err(Error::InvalidId);
        }
        if self.command.trim().is_empty()
            || self.command.contains('\0')
            || self.cwd.contains('\0')
            || self.command.len() > 16_384
            || self.cwd.is_empty()
            || self.cwd.len() > 4_096
            || !self.cwd.starts_with('/')
            || self.timeout_seconds == 0
            || self.timeout_seconds > 600
        {
            return Err(Error::InvalidExecRequest);
        }
        Ok(())
    }
}

// Private transport seam. Production origin is fixed, with no URL/env override.
struct DiscoveryClient {
    http: Client,
    origin: String,
    key: HeaderValue,
}
impl DiscoveryClient {
    fn new(key: &str) -> Result<Self, Error> {
        if key.is_empty() {
            return Err(Error::MissingKey);
        }
        if !key.starts_with("ibk_") {
            return Err(Error::InvalidKey);
        }
        let mut key = HeaderValue::from_str(key).map_err(|_| Error::InvalidKey)?;
        key.set_sensitive(true);
        let http = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| Error::Transport)?;
        Ok(Self {
            http,
            origin: API.into(),
            key,
        })
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        self.request(self.http.get(format!("{}{path}", self.origin)))
            .await
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, Error> {
        let mut response = request
            .header("X-API-Key", self.key.clone())
            .send()
            .await
            .map_err(|_| Error::Transport)?;
        match response.status().as_u16() {
            200 => (),
            401 => return Err(Error::Authentication),
            403 => return Err(Error::Forbidden),
            404 => return Err(Error::NotFound),
            429 => return Err(Error::Quota),
            503 => return Err(Error::Unavailable),
            _ => return Err(Error::Transport),
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Transport)? {
            if bytes.len().saturating_add(chunk.len()) > MAX_BODY {
                return Err(Error::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)
    }

    async fn execute(
        &self,
        granted: Option<&ExecGrant>,
        request: &ExecGrant,
        cancel: &Cancellation,
    ) -> Result<ExecReport, Error> {
        self.execute_bounded(
            granted,
            request,
            cancel,
            Duration::from_secs(15),
            Duration::from_secs(request.timeout_seconds.saturating_add(5)),
        )
        .await
    }

    // Private duration seam for deterministic stalled-server tests; production bounds are fixed above.
    async fn execute_bounded(
        &self,
        granted: Option<&ExecGrant>,
        request: &ExecGrant,
        cancel: &Cancellation,
        metadata_wait: Duration,
        response_wait: Duration,
    ) -> Result<ExecReport, Error> {
        request.validate()?;
        if granted != Some(request) {
            return Err(Error::ExecDenied);
        }
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let started = Instant::now();
        let machine_path = format!("/v1/machines/{}", request.machine_id);
        let mut machine: Machine = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(Error::Cancelled),
            _ = tokio::time::sleep(metadata_wait) => return Err(Error::Timeout),
            value = self.get(&machine_path) => value?,
        };
        machine.validate()?;
        if machine.id != request.machine_id {
            return Err(Error::InvalidResponse);
        }
        if machine.state != "running" {
            return Err(Error::MachineNotRunning);
        }
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            cmd: &'a str,
            cwd: &'a str,
            timeout_seconds: u64,
        }
        let post = self.request::<ExecResponse>(
            self.http
                .post(format!(
                    "{}/v1/machines/{}/exec",
                    self.origin, request.machine_id
                ))
                .json(&Payload {
                    cmd: &request.command,
                    cwd: &request.cwd,
                    timeout_seconds: request.timeout_seconds,
                }),
        );
        let response = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Ok(ExecReport::unknown(request, ExecStatus::Cancelled, "cancelled", started)),
            _ = tokio::time::sleep(response_wait) => return Ok(ExecReport::unknown(request, ExecStatus::DeadlineExceeded, "local_deadline", started)),
            value = post => match value {
                Ok(response) => response,
                Err(error) => return Ok(ExecReport::unknown(request, ExecStatus::Failed, error.category(), started)),
            },
        };
        Ok(ExecReport {
            target: "instabox",
            machine_id: request.machine_id.clone(),
            status: ExecStatus::Completed,
            remote_outcome: RemoteOutcome::ObservedCompleted,
            stdout: response.stdout,
            stderr: response.stderr,
            exit_status: Some(response.exit_code),
            timed_out: response.timed_out,
            output_truncated: response.truncated,
            output_available: true,
            failure: None,
            elapsed_ms: started.elapsed().as_millis(),
        })
    }

    async fn discover(
        &self,
        allowed: bool,
        id: Option<&str>,
        cancel: &Cancellation,
        deadline: Duration,
    ) -> Result<Discovery, Error> {
        if !allowed {
            return Err(Error::Denied);
        }
        if id.is_some_and(|id| !valid_id(id)) {
            return Err(Error::InvalidId);
        }
        let started = Instant::now();
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(Error::Cancelled),
            _ = tokio::time::sleep(deadline) => Err(Error::Timeout),
            result = async {
                let (mut machines, list_complete) = if let Some(id) = id {
                    let machine: Machine = self.get(&format!("/v1/machines/{id}")).await?;
                    if machine.id != id { return Err(Error::InvalidResponse); }
                    (vec![machine], false) // selected machine is not an account inventory
                } else {
                    let page: Page = self.get("/v1/machines?limit=100").await?;
                    if page.machines.len() > 100 || !(page.next_cursor.is_null() || page.next_cursor.is_string()) {
                        return Err(Error::InvalidResponse);
                    }
                    (page.machines, page.next_cursor.is_null())
                };
                for machine in &mut machines { machine.validate()?; }
                let usage: Usage = self.get("/v1/usage").await?;
                Ok(Discovery { target: "instabox", machines, list_complete,
                    storage_measurement_available: usage.computed_at.is_some(), usage,
                    elapsed_ms: started.elapsed().as_millis() })
            } => result,
        }
    }
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

fn invocation_key() -> Result<String, Error> {
    let environment = match std::env::var("INSTABOX_API_KEY") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err(Error::InvalidKey),
    };
    load_key(environment, Path::new(".env"))
}

#[cfg(test)]
#[path = "../tests/support/instabox_mock.rs"]
mod tests;
