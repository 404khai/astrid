//! Bounded InstaBox HTTP executor. No environment, CLI parsing or key discovery.
use crate::cancellation::Cancellation;
use reqwest::{Client, header::HeaderValue};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
const API: &str = "https://api.instabox.sh";
const MAX_BODY: usize = 262_144;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
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
    #[error("remote execution grant expired")]
    GrantExpired,
    #[error("remote execution grant already consumed")]
    GrantConsumed,
    #[error(
        "InstaBox execution requires an existing machine in running state; Astrid will not start or resume it"
    )]
    MachineNotRunning,
    #[error("invalid execution request")]
    InvalidExecRequest,
    #[error("InstaBox execution did not succeed; inspect the JSON outcome (no automatic retry)")]
    ExecUnsuccessful,
}

pub fn valid_id(id: &str) -> bool {
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
pub struct Discovery {
    target: &'static str,
    machines: Vec<Machine>,
    list_complete: bool,
    usage: Usage,
    storage_measurement_available: bool,
    elapsed_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExecGrant {
    pub machine_id: String,
    pub command: String,
    pub cwd: String,
    pub timeout_seconds: u64,
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
pub enum ExecStatus {
    Completed,
    Cancelled,
    DeadlineExceeded,
    Failed,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteOutcome {
    ObservedCompleted,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct ExecReport {
    pub target: &'static str,
    pub machine_id: String,
    pub status: ExecStatus,
    pub remote_outcome: RemoteOutcome,
    pub stdout: String,
    pub stderr: String,
    pub exit_status: Option<i32>,
    pub timed_out: bool,
    pub output_truncated: bool,
    pub output_available: bool,
    pub failure: Option<&'static str>,
    pub elapsed_ms: u128,
}

impl Error {
    pub(crate) fn category(&self) -> &'static str {
        match self {
            Self::Authentication => "authentication",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Quota => "quota",
            Self::Unavailable => "unavailable",
            Self::InvalidResponse => "invalid_response",
            Self::ExecDenied => "permission_denied",
            Self::GrantExpired => "grant_expired",
            Self::GrantConsumed => "grant_consumed",
            Self::MachineNotRunning => "machine_not_running",
            Self::InvalidId | Self::InvalidExecRequest => "invalid_arguments",
            Self::Timeout => "local_deadline",
            Self::Cancelled => "cancelled",
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
    pub fn validate(&self) -> Result<(), Error> {
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
pub struct DiscoveryClient {
    http: Client,
    origin: String,
    key: HeaderValue,
}
impl DiscoveryClient {
    pub fn new(key: &str) -> Result<Self, Error> {
        if key.is_empty() {
            return Err(Error::MissingKey);
        }
        if !key.starts_with("ibk_") {
            return Err(Error::InvalidKey);
        }
        let mut key = HeaderValue::from_str(key).map_err(|_| Error::InvalidKey)?;
        key.set_sensitive(true);
        let http = Client::builder()
            // Disable even reqwest's default safe protocol-NACK retries.
            .retry(reqwest::retry::never())
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

    pub async fn execute(
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
        self.execute_guarded(request, cancel, metadata_wait, response_wait, &|_| {
            if granted == Some(request) {
                Ok(())
            } else {
                Err(Error::ExecDenied)
            }
        })
        .await
    }

    // Called only by the typed remote boundary. `admit(true)` atomically consumes
    // its grant after preflight; no await separates consumption from POST admission.
    pub(crate) async fn execute_guarded(
        &self,
        request: &ExecGrant,
        cancel: &Cancellation,
        metadata_wait: Duration,
        response_wait: Duration,
        admit: &(dyn Fn(bool) -> Result<(), Error> + Sync),
    ) -> Result<ExecReport, Error> {
        request.validate()?;
        admit(false)?;
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
        // Final binding/expiry/cancellation check and single atomic consumption.
        admit(true)?;
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

    pub async fn discover(
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

#[cfg(test)]
#[path = "../tests/support/instabox_mock.rs"]
pub(crate) mod tests;
