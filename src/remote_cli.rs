//! Invocation-only host configuration and explicit foreground remote approvals.
//! No target discovery, persisted authority, worker installation or automatic grant.
use astrid::{
    instabox::{DiscoveryClient, Error},
    remote::{ApprovalRequest, InstaboxTool, RemoteApproval, RemoteGrant},
    runtime::PermissionHandler,
    tools::{PermissionRequest, Tools},
};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};

pub(crate) fn machine_id(value: &str) -> Result<String, String> {
    if astrid::instabox::valid_id(value) {
        Ok(value.to_owned())
    } else {
        Err("expected exact existing InstaBox machine ID (m_...), never a name or URL".into())
    }
}

pub(crate) struct ForegroundRemote {
    machine: String,
    client: Arc<DiscoveryClient>,
    computer_wire: Option<Arc<astrid::computer_wire::ComputerWire>>,
}
impl ForegroundRemote {
    pub(crate) fn load(machine: String) -> Result<Self, Error> {
        Self::load_kind(machine, false)
    }
    pub(crate) fn load_computer(machine: String) -> Result<Self, Error> {
        Self::load_kind(machine, true)
    }
    fn load_kind(machine: String, computer: bool) -> Result<Self, Error> {
        if !astrid::instabox::valid_id(&machine) {
            return Err(Error::InvalidId);
        }
        // Local loading only. Selection never performs authenticated discovery.
        let key = crate::instabox::invocation_key()?;
        Ok(Self {
            machine,
            client: Arc::new(DiscoveryClient::new(&key)?),
            computer_wire: if computer {
                Some(Arc::new(
                    astrid::computer_wire::ComputerWire::new(&key)
                        .map_err(|_| Error::InvalidKey)?,
                ))
            } else {
                None
            },
        })
    }

    pub(crate) fn install(
        &self,
        tools: Tools,
        requests: tokio::sync::mpsc::Sender<crate::DecisionRequest>,
    ) -> Result<Tools, Error> {
        if let Some(wire) = &self.computer_wire {
            let computer = astrid::computer::Computer::new(
                format!("local-euid:{}", unsafe { libc::geteuid() }),
                self.machine.clone(),
                wire.clone(),
                Arc::new(astrid::computer_png::DecodeWorker::new(
                    std::env::current_exe().map_err(|_| Error::Transport)?,
                )),
            )
            .map_err(|_| Error::InvalidExecRequest)?;
            return Ok(tools.with_computer(computer));
        }
        Ok(tools.with_instabox(InstaboxTool::new(
            // Host assertion for the invoking local account, never model/user text.
            format!("local-euid:{}", unsafe { libc::geteuid() }),
            self.machine.clone(),
            self.client.clone(),
            Arc::new(TerminalRemoteApproval { requests }),
        )?))
    }
}

struct TerminalRemoteApproval {
    requests: tokio::sync::mpsc::Sender<crate::DecisionRequest>,
}
#[async_trait]
impl RemoteApproval for TerminalRemoteApproval {
    async fn approve(&self, request: &ApprovalRequest) -> Option<RemoteGrant> {
        // Share the foreground answer transport, not local policy or its grants.
        // The runtime emits the complete private description before arming input.
        let mut terminal = crate::TerminalPermission {
            requests: self.requests.clone(),
        };
        let answer = terminal
            .decide(&PermissionRequest {
                command: request.description(),
                workspace: format!("instabox:{}", request.machine_id).into(),
            })
            .await;
        if matches!(answer, Ok(true)) {
            RemoteGrant::approve(request.clone(), Duration::from_secs(60)).ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astrid::{
        events::{RunId, ToolCallId},
        remote::ExecArgs,
    };
    const ID: &str = "m_0123456789abcdefghijklmnop";
    fn request() -> ApprovalRequest {
        ApprovalRequest {
            principal: "local-euid:test".into(),
            run_id: RunId::default(),
            tool_call_id: ToolCallId::default(),
            call_id: "provider-call".into(),
            machine_id: ID.into(),
            args: ExecArgs {
                command: "printf 'hello\\n'; pwd".into(),
                cwd: "/data/home".into(),
                timeout_seconds: 2,
            },
        }
    }
    #[test]
    fn approval_display_preserves_control_bytes_as_visible_json_escapes() {
        let mut binding = request();
        binding.args.command = "printf \"x\"\n\u{1b}[2J\r".into();
        binding.args.cwd = "/data/home/\tpath".into();
        let display = binding.description();
        assert!(display.contains(&serde_json::json!(binding.args.command).to_string()));
        assert!(display.contains(&serde_json::json!(binding.args.cwd).to_string()));
        assert!(!display.contains('\u{1b}'));
        assert!(!display.contains('\r'));
        assert_eq!(crate::console::printable(&display), display);
    }
    #[tokio::test]
    async fn remote_broker_requires_a_fresh_exact_answer_and_closed_ui_denies() {
        for answer in [Ok(false), Ok(true), Err(std::io::Error::other("closed"))] {
            let (tx, mut rx) = tokio::sync::mpsc::channel(1);
            let broker = TerminalRemoteApproval { requests: tx };
            let binding = request();
            let expected = matches!(answer, Ok(true));
            let approval = broker.approve(&binding);
            let user = async {
                let (display, reply) = rx.recv().await.unwrap();
                assert_eq!(display.command, binding.description());
                assert!(display.command.contains(ID));
                assert!(
                    display
                        .command
                        .contains(&serde_json::json!(binding.args.command).to_string())
                );
                assert!(display.command.contains("auto-wake"));
                assert_eq!(
                    display.workspace.to_str().unwrap(),
                    format!("instabox:{ID}")
                );
                reply.send(answer).unwrap();
            };
            let (grant, ()) = tokio::join!(approval, user);
            assert_eq!(grant.is_some(), expected);
            drop(rx);
            assert!(broker.approve(&binding).await.is_none());
        }
    }
    #[tokio::test]
    async fn dropping_pending_approval_closes_its_answer_and_cannot_grant_next_call() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let broker = TerminalRemoteApproval { requests: tx };
        let binding = request();
        let mut approval = Box::pin(broker.approve(&binding));
        let (_, reply) = tokio::select! {
            _ = &mut approval => panic!("approval without answer"),
            request = rx.recv() => request.unwrap(),
        };
        drop(approval);
        assert!(reply.send(Ok(true)).is_err());
        let next = request();
        let (grant, ()) = tokio::join!(broker.approve(&next), async {
            let (display, reply) = rx.recv().await.unwrap();
            assert_eq!(display.command, next.description());
            reply.send(Ok(false)).unwrap();
        });
        assert!(grant.is_none());
    }
}
