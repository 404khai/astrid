//! Explicit per-run authority. General shell execution is broad account authority.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum PermissionAction {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionPolicy {
    pub read: PermissionAction,
    pub write: PermissionAction,
    pub execute: PermissionAction,
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            read: PermissionAction::Allow,
            write: PermissionAction::Allow,
            execute: PermissionAction::Ask,
        }
    }
}

impl PermissionPolicy {
    pub fn action(self, capability: Capability) -> PermissionAction {
        match capability {
            Capability::Read => self.read,
            Capability::Write => self.write,
            Capability::Execute => self.execute,
        }
    }
}

pub fn capability(tool: &str) -> Option<Capability> {
    match tool {
        "read_file" | "list_directory" | "glob" | "grep" => Some(Capability::Read),
        "write_file" | "edit_file" => Some(Capability::Write),
        "shell" => Some(Capability::Execute),
        _ => None,
    }
}
