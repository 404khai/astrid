//! Fixed, typed X11 plans over the documented InstaBox exec/status protocol.
//! No model command/path/env parameters and no automatic retries or lifecycle calls.
use crate::{
    cancellation::Cancellation,
    desktop::{
        Action, ActionTransport, ApprovalRequest, Button, Direction, Modifier, TransportFailure,
    },
};
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Client, header::HeaderValue};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

pub(crate) const CAPTURE: &str = "set -o pipefail; export DISPLAY=:1; import -limit memory 64MiB -limit map 0 -limit disk 0 -window root -depth 8 -interlace none png24:- | head -c 1048577 | base64 | tr -d '\\n'";
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WireError {
    #[error("computer transport configuration invalid")]
    Configuration,
    #[error("exact machine is not running")]
    NotRunning,
    #[error("computer request cancelled")]
    Cancelled,
    #[error("computer HTTP request rejected (status {0})")]
    HttpStatus(u16),
    #[error("computer transport deadline exceeded")]
    RequestTimeout,
    #[error("computer response byte limit exceeded")]
    ResponseLimit,
    #[error("computer response invalid")]
    ResponseInvalid,
    #[error("computer command exited with status {0}")]
    CommandExit(i32),
    #[error("computer command timed out")]
    CommandTimeout,
    #[error("computer output truncated by provider")]
    Truncated,
    #[error("capture stdout is not valid base64")]
    CaptureEncoding,
    #[error("capture PNG byte limit exceeded")]
    CaptureLimit,
    #[error("computer transport failed; outcome may be unknown")]
    Unknown,
}
impl WireError {
    fn transport(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::RequestTimeout
        } else {
            Self::Unknown
        }
    }
    /// Fixed host categories only. Never include raw HTTP bodies or guest stderr.
    pub(crate) fn capture_failure(self) -> &'static str {
        match self {
            Self::HttpStatus(401 | 403) => "capture HTTP authentication or permission rejected",
            Self::HttpStatus(_) => "capture HTTP request rejected",
            Self::RequestTimeout => "capture transport deadline exceeded",
            Self::ResponseLimit => "capture HTTP response byte limit exceeded",
            Self::ResponseInvalid => "capture response invalid",
            Self::CommandTimeout => "capture command timed out (2-second limit)",
            Self::Truncated => {
                "capture output truncated by provider (base64 may exceed stdout limit)"
            }
            Self::CommandExit(127) => "capture command not found (exit 127)",
            Self::CommandExit(_) => "capture command exited unsuccessfully",
            Self::CaptureEncoding => "capture stdout is not valid base64",
            Self::CaptureLimit => "capture PNG byte limit exceeded",
            Self::Cancelled => "observation cancelled",
            _ => "capture connection or configuration failed; remote outcome unknown",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Plan {
    pub cmd: String,
    pub cwd: &'static str,
    pub timeout_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
}
impl Plan {
    pub(crate) fn capture() -> Self {
        Self {
            cmd: CAPTURE.into(),
            cwd: "/data/home",
            timeout_seconds: 2,
            stdin: None,
        }
    }
    pub(crate) fn action(action: &Action) -> Result<Self, WireError> {
        action
            .validate(2560, 2560)
            .map_err(|_| WireError::Configuration)?;
        let mut stdin = None;
        let args = match action {
            Action::Click { x, y, button } => format!(
                "mousemove {x} {y} click {}",
                match button {
                    Button::Left => 1,
                    Button::Middle => 2,
                    Button::Right => 3,
                }
            ),
            Action::Type { text } => {
                stdin = Some(text.clone());
                "type --delay 0 --file -".into()
            }
            Action::Scroll { direction, notches } => format!(
                "click --repeat {notches} --delay 30 {}",
                match direction {
                    Direction::Up => 4,
                    Direction::Down => 5,
                    Direction::Left => 6,
                    Direction::Right => 7,
                }
            ),
            Action::Key { key, modifiers } => {
                let base = match key.as_str() {
                    "Enter" => "Return",
                    "Backspace" => "BackSpace",
                    "PageUp" => "Prior",
                    "PageDown" => "Next",
                    "Space" => "space",
                    s => s,
                };
                let mut keys = modifiers
                    .iter()
                    .map(|m| match m {
                        Modifier::Ctrl => "ctrl",
                        Modifier::Alt => "alt",
                        Modifier::Shift => "shift",
                    })
                    .collect::<Vec<_>>();
                keys.push(base);
                format!("key -- '{}'", keys.join("+"))
            }
        };
        Ok(Self {
            cmd: format!("export DISPLAY=:1; xdotool {args}"),
            cwd: "/data/home",
            timeout_seconds: 2,
            stdin,
        })
    }
}

pub struct ComputerWire {
    http: Client,
    origin: String,
    key: HeaderValue,
}
impl ComputerWire {
    pub fn new(key: &str) -> Result<Self, WireError> {
        if !key.starts_with("ibk_") {
            return Err(WireError::Configuration);
        }
        let mut key = HeaderValue::from_str(key).map_err(|_| WireError::Configuration)?;
        key.set_sensitive(true);
        let http = Client::builder()
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| WireError::Configuration)?;
        Ok(Self {
            http,
            origin: "https://api.instabox.sh".into(),
            key,
        })
    }
    async fn json<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        cap: usize,
        cancel: &Cancellation,
    ) -> Result<T, WireError> {
        if cancel.is_cancelled() {
            return Err(WireError::Cancelled);
        }
        let fetch = async {
            let mut response = request
                .header("X-API-Key", self.key.clone())
                .send()
                .await
                .map_err(WireError::transport)?;
            if response.status() != 200 {
                return Err(WireError::HttpStatus(response.status().as_u16()));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(WireError::transport)? {
                if bytes.len().saturating_add(chunk.len()) > cap {
                    return Err(WireError::ResponseLimit);
                }
                bytes.extend_from_slice(&chunk);
            }
            serde_json::from_slice(&bytes).map_err(|_| WireError::ResponseInvalid)
        };
        tokio::select! {biased;_=cancel.cancelled()=>Err(WireError::Cancelled),value=fetch=>value}
    }
    pub(crate) async fn preflight(
        &self,
        machine: &str,
        cancel: &Cancellation,
    ) -> Result<(), WireError> {
        if !crate::instabox::valid_id(machine) {
            return Err(WireError::Configuration);
        }
        #[derive(Deserialize)]
        struct Machine {
            id: String,
            state: String,
        }
        let value: Machine = self
            .json(
                self.http
                    .get(format!("{}/v1/machines/{machine}", self.origin)),
                16_384,
                cancel,
            )
            .await?;
        if value.id != machine {
            return Err(WireError::Unknown);
        }
        if value.state != "running" {
            return Err(WireError::NotRunning);
        }
        Ok(())
    }
    pub(crate) async fn status(
        &self,
        machine: &str,
        cancel: &Cancellation,
    ) -> Result<(bool, bool), WireError> {
        #[derive(Deserialize)]
        struct Status {
            installed: bool,
            running: bool,
        }
        let value: Status = self
            .json(
                self.http
                    .post(format!("{}/v1/machines/{machine}/desktop", self.origin))
                    .json(&serde_json::json!({"action":"status"})),
                16_384,
                cancel,
            )
            .await?;
        if value.running && !value.installed {
            return Err(WireError::Unknown);
        }
        Ok((value.installed, value.running))
    }
    async fn exec(
        &self,
        machine: &str,
        plan: &Plan,
        cap: usize,
        cancel: &Cancellation,
    ) -> Result<Exec, WireError> {
        let result: Exec = self
            .json(
                self.http
                    .post(format!("{}/v1/machines/{machine}/exec", self.origin))
                    .json(plan),
                cap,
                cancel,
            )
            .await?;
        if result.timed_out {
            return Err(WireError::CommandTimeout);
        }
        if result.truncated {
            return Err(WireError::Truncated);
        }
        if result.exit_code != 0 {
            return Err(WireError::CommandExit(result.exit_code));
        }
        Ok(result)
    }
    pub(crate) async fn capture(
        &self,
        machine: &str,
        cancel: &Cancellation,
    ) -> Result<Vec<u8>, WireError> {
        let result = self
            .exec(machine, &Plan::capture(), 1_500_000, cancel)
            .await?;
        if result.stdout.len() > crate::computer_png::MAX_PNG.div_ceil(3) * 4 {
            return Err(WireError::CaptureLimit);
        }
        let bytes = STANDARD
            .decode(result.stdout)
            .map_err(|_| WireError::CaptureEncoding)?;
        if bytes.len() > crate::computer_png::MAX_PNG {
            return Err(WireError::CaptureLimit);
        }
        Ok(bytes)
    }
    #[cfg(test)]
    pub(crate) fn at(key: &str, origin: String) -> Self {
        let mut w = Self::new(key).unwrap();
        w.origin = origin;
        w
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Exec {
    stdout: String,
    exit_code: i32,
    timed_out: bool,
    truncated: bool,
}

pub(crate) struct TypedActionTransport {
    pub wire: Arc<ComputerWire>,
    pub cancel: Cancellation,
}
#[async_trait]
impl ActionTransport for TypedActionTransport {
    async fn act(&self, request: &ApprovalRequest) -> Result<(), TransportFailure> {
        let plan = Plan::action(&request.args.action).map_err(|_| TransportFailure)?;
        self.wire
            .exec(&request.owner.machine_id, &plan, 16_384, &self.cancel)
            .await
            .map(|_| ())
            .map_err(|_| TransportFailure)
    }
}
