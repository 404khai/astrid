//! Optional metadata-only experiment. No model tool, VM connection, or execution authority.
use astrid::cancellation::Cancellation;
use clap::Subcommand;
use reqwest::{Client, header::HeaderValue};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

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
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum Error {
    #[error("InstaBox discovery denied; explicit --allow-discovery is required")]
    Denied,
    #[error("set INSTABOX_API_KEY to an InstaBox API key with machines:read scope")]
    MissingKey,
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
    #[error("InstaBox discovery timed out")]
    Timeout,
    #[error("InstaBox discovery cancelled")]
    Cancelled,
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
        let mut response = self
            .http
            .get(format!("{}{path}", self.origin))
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
    let Command::Discover {
        allow_discovery,
        machine,
    } = command;
    // Deny before even consulting the environment; never read local provider/CLI credentials.
    if !allow_discovery {
        return Err(Error::Denied.into());
    }
    if machine.as_deref().is_some_and(|id| !valid_id(id)) {
        return Err(Error::InvalidId.into());
    }
    let key = std::env::var("INSTABOX_API_KEY").map_err(|_| Error::MissingKey)?;
    let client = DiscoveryClient::new(&key)?;
    let cancel = Cancellation::default();
    let result = tokio::select! {
        biased;
        _ = tokio::signal::ctrl_c() => { cancel.cancel(); Err(Error::Cancelled) },
        result = client.discover(true, machine.as_deref(), &cancel, Duration::from_secs(15)) => result,
    }?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

#[cfg(test)]
#[path = "../tests/support/instabox_mock.rs"]
mod tests;
