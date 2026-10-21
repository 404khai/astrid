//! ADR 0021: host-owned foreground remote capability. No credentials in arguments,
//! grants, sessions or model schemas. Local shell presets never approve this tool.
use crate::{
    cancellation::Cancellation,
    events::{RunId, ToolCallId},
    instabox::{DiscoveryClient, Error, ExecGrant, ExecReport, ExecStatus},
    model::{ToolCall, ToolOutcome, ToolResult},
    tools::ToolExecution,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// Canonical tuple: JSON field order/spacing is irrelevant; string bytes are exact.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecArgs {
    pub command: String,
    pub cwd: String,
    pub timeout_seconds: u64,
}

/// Host principal and runtime identities, never deserialized from model input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub principal: String,
    pub run_id: RunId,
    pub tool_call_id: ToolCallId,
    pub call_id: String,
    pub machine_id: String,
    pub args: ExecArgs,
}
impl ApprovalRequest {
    fn wire(&self) -> ExecGrant {
        ExecGrant {
            machine_id: self.machine_id.clone(),
            command: self.args.command.clone(),
            cwd: self.args.cwd.clone(),
            timeout_seconds: self.args.timeout_seconds,
        }
    }
    /// Content-bearing approval display only, never persistent metadata.
    pub fn description(&self) -> String {
        format!(
            "instabox_exec {}\nPrincipal: {}\nRun: {}\nCall: {} ({})\nExact command (JSON string): {}\nExact cwd (JSON string): {}\nTimeout: {}s\nRunning check is point-in-time; POST may auto-wake a machine paused after GET.",
            self.machine_id,
            self.principal,
            self.run_id,
            self.tool_call_id,
            json!(self.call_id),
            json!(self.args.command),
            json!(self.args.cwd),
            self.args.timeout_seconds
        )
    }
}

/// Non-serializable and not Clone. Even shared references can admit only one POST.
/// Host brokers create grants only after their own explicit remote approval.
pub struct RemoteGrant {
    binding: ApprovalRequest,
    expires: Instant,
    consumed: AtomicBool,
}
impl RemoteGrant {
    pub fn approve(binding: ApprovalRequest, valid_for: Duration) -> Result<Self, Error> {
        binding.wire().validate()?;
        let expires = Instant::now()
            .checked_add(valid_for)
            .ok_or(Error::GrantExpired)?;
        Ok(Self {
            binding,
            expires,
            consumed: AtomicBool::new(false),
        })
    }
    pub(crate) fn check(
        &self,
        binding: &ApprovalRequest,
        cancel: &Cancellation,
    ) -> Result<(), Error> {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        if self.binding != *binding {
            return Err(Error::ExecDenied);
        }
        if Instant::now() >= self.expires {
            return Err(Error::GrantExpired);
        }
        if self.consumed.load(Ordering::Acquire) {
            return Err(Error::GrantConsumed);
        }
        Ok(())
    }
    fn consume(&self, binding: &ApprovalRequest, cancel: &Cancellation) -> Result<(), Error> {
        self.check(binding, cancel)?;
        self.consumed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| Error::GrantConsumed)
    }
}

#[async_trait]
pub trait RemoteApproval: Send + Sync {
    /// None denies. No authenticated metadata request occurs before a valid answer.
    /// No local PermissionHandler or preset is consulted by this broker.
    async fn approve(&self, request: &ApprovalRequest) -> Option<RemoteGrant>;
}

/// Opaque credential boundary: only the HTTP client holds a sensitive header.
/// No environment loading, serialization, Debug implementation or target mutation.
pub struct InstaboxTool {
    principal: String,
    machine_id: String,
    client: Arc<DiscoveryClient>,
    broker: Arc<dyn RemoteApproval>,
}
impl InstaboxTool {
    pub fn new(
        principal: String,
        machine_id: String,
        client: Arc<DiscoveryClient>,
        broker: Arc<dyn RemoteApproval>,
    ) -> Result<Self, Error> {
        if principal.trim().is_empty()
            || principal.len() > 256
            || principal.chars().any(char::is_control)
        {
            return Err(Error::ExecDenied);
        }
        if !crate::instabox::valid_id(&machine_id) {
            return Err(Error::InvalidId);
        }
        Ok(Self {
            principal,
            machine_id,
            client,
            broker,
        })
    }
    pub(crate) fn request(
        &self,
        run_id: RunId,
        tool_call_id: ToolCallId,
        call: &ToolCall,
    ) -> Result<ApprovalRequest, Error> {
        if call.name != "instabox_exec" {
            return Err(Error::InvalidExecRequest);
        }
        let args = serde_json::from_str::<ExecArgs>(&call.arguments)
            .map_err(|_| Error::InvalidExecRequest)?;
        let request = ApprovalRequest {
            principal: self.principal.clone(),
            run_id,
            tool_call_id,
            call_id: call.call_id.clone(),
            machine_id: self.machine_id.clone(),
            args,
        };
        request.wire().validate()?;
        Ok(request)
    }
    pub(crate) async fn approve(&self, request: &ApprovalRequest) -> Option<RemoteGrant> {
        self.broker.approve(request).await
    }
    pub(crate) async fn execute(
        &self,
        request: &ApprovalRequest,
        grant: &RemoteGrant,
        cancel: &Cancellation,
    ) -> ToolExecution {
        self.execute_waits(
            request,
            grant,
            cancel,
            (
                Duration::from_secs(15),
                Duration::from_secs(request.args.timeout_seconds + 5),
            ),
        )
        .await
    }
    async fn execute_waits(
        &self,
        request: &ApprovalRequest,
        grant: &RemoteGrant,
        cancel: &Cancellation,
        waits: (Duration, Duration),
    ) -> ToolExecution {
        let started = Instant::now();
        let wire = request.wire();
        let report = self
            .client
            .execute_guarded(&wire, cancel, waits.0, waits.1, &|consume| {
                if request.principal != self.principal || request.machine_id != self.machine_id {
                    return Err(Error::ExecDenied);
                }
                if consume {
                    grant.consume(request, cancel)
                } else {
                    grant.check(request, cancel)
                }
            })
            .await;
        let data = match report {
            Ok(report) => {
                let cancelled = report.status == ExecStatus::Cancelled;
                let data = report_data(request, report);
                if cancelled {
                    return ToolExecution::CancelledWithOutput { data };
                }
                data
            }
            Err(Error::Cancelled) => return ToolExecution::Cancelled,
            Err(error) => json!({"target":"instabox", "machine_id":request.machine_id,
                "run_id":request.run_id, "tool_call_id":request.tool_call_id, "call_id":request.call_id,
                "status":"not_dispatched", "remote_outcome":"not_dispatched", "successful":false,
                "failure":error.category(), "output_available":false, "exit_status":null,
                "post_admitted":false, "elapsed_ms":started.elapsed().as_millis()}),
        };
        // Keep the existing session v2 outcome envelope. Success here carries a
        // report; ToolResult::is_error checks its explicit `successful` field so
        // failed/nonzero/unknown reports produce ToolCallFailed, never success.
        // Original stdout/stderr/exit fields remain untouched.
        let outcome = if data["timed_out"] == true || data["status"] == "deadline_exceeded" {
            ToolOutcome::TimedOut { data }
        } else {
            ToolOutcome::Success { data }
        };
        ToolExecution::Finished(ToolResult {
            call_id: request.call_id.clone(),
            name: "instabox_exec".into(),
            outcome,
        })
    }
}
fn report_data(request: &ApprovalRequest, report: ExecReport) -> Value {
    let successful = report.status == ExecStatus::Completed
        && report.exit_status == Some(0)
        && !report.timed_out;
    let mut data = serde_json::to_value(report).expect("fixed report serializes");
    data["successful"] = json!(successful);
    data["run_id"] = json!(request.run_id);
    data["tool_call_id"] = json!(request.tool_call_id);
    data["call_id"] = json!(request.call_id);
    data["post_admitted"] = json!(true);
    data
}

pub(crate) fn definition() -> Value {
    json!({"type":"function","name":"instabox_exec","strict":true,
        "description":"Execute on the host-configured exact existing InstaBox machine. Requires separate exact-call remote approval, including possible auto-wake after running check. Unknown outcome does not prove termination and must not be retried automatically.",
        "parameters":{"type":"object","properties":{"command":{"type":"string"},"cwd":{"type":"string","description":"Absolute remote cwd"},"timeout_seconds":{"type":"integer","minimum":1,"maximum":600}},"required":["command","cwd","timeout_seconds"],"additionalProperties":false}})
}

#[cfg(test)]
#[path = "../tests/support/instabox_runtime_mock.rs"]
mod tests;
