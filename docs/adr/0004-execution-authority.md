# ADR 0004: Permission policy and execution authority

Date: 2026-10-06
Status: Accepted
Implementation: Implemented; validated 2026-10-07
Related: Phase 2; P2-05 and P2-06; ADR 0001 execution authority; ADR 0002 permissions
Decision evidence: Maintainer explicitly selected the recommended contract in
conversation on 2026-10-07.

## Context

Native tools validate workspace paths. Shell runs with account authority after
confirmation; cwd does not confine it. PermissionRequest currently contains only
command/workspace. A configurable policy must not imply that parsing a command
string can reliably identify network access or destructive behavior.

## Options

1. Keep fixed per-command shell approval and automatic native tools. Smallest
   surface, but does not fulfill configurable permissions.
2. Configure allow/ask/deny for typed native read/write and broad shell execution;
   explicitly acknowledge shell's wider authority.
3. Add an OS-enforced sandbox with filesystem/network/process restrictions.
   Stronger boundary, but substantially larger platform and lifecycle work.

## Accepted decision

Choose option 2 for trusted local repositories. Default to native reads/writes
allowed within validated paths and every shell call asking, preserving today's
behavior. A deny rule wins; an ask without a usable permission handler denies.
Cancellation still wins before dispatch, including after an approval race.

Use explicit per-run library configuration and CLI arguments initially; do not
load repository-controlled permission settings. Validate the whole policy before
model/tool execution. Do not add layered config precedence or remembered approvals.
An explicit shell-allow policy grants broad account-level execution authority;
surface that fact in effective configuration and decision events.

Every tool decision records capability, effective action, and reason through
runtime events, including automatic allow/deny. Human permission waiting remains
distinct from policy evaluation and ToolCallStarted. Keep existing terminal
invariants. Denied reads/writes must not invoke their executor or mutate files.

Do not infer enforceable read-only, network-denied, or non-destructive shell
subsets from command spelling. Network/destructive capability classes are not
independently enforceable for the general shell under this proposal; document
that limitation and reject configuration that claims otherwise. Native tools
with such explicit capabilities can be classified when introduced. No new delete
tool is required. Path validation remains mandatory even when policy says allow.

## Consequences

Changes ADR 0001's fixed shell approval rule only when the user explicitly selects
another policy; extends ADR 0002's permission representation. It is not protection
against hostile repository code, detached descendants, concurrent filesystem
swaps, or exfiltration by approved shell commands. Existing workspace and hard-link
checks remain valuable but must not be advertised as a complete security sandbox.

If hostile-code confinement is required to satisfy Phase 2's exit criterion,
choose option 3 and re-scope the tickets before implementation. Do not describe
option 2 as equivalent protection.

## Decision question

Is Phase 2 scoped to trusted local repositories with explicit broad shell
authority, or must it enforce containment of hostile code before completion?

## Validation

Exercise allow/ask/deny for each supported capability with an executor spy.
Verify deny has no side effects, missing input denies, policy is visible headlessly,
repo files cannot elevate authority, approval racing cancellation cannot spawn,
and invalid or unenforceable policies fail before execution.

## Resolution

Accepted on 2026-10-07. ADR 0001 fixed shell confirmation is amended only
for explicitly configured policy. Defaults retain ask for shell execution.

## Implementation evidence

`PermissionPolicy` exposes typed read/write/execute actions in `RunConfig`;
CLI flags are `--read-policy`, `--write-policy`, and `--shell-policy`. The runtime
emits effective policy and per-tool decision reasons, then asks, denies, or
dispatches. Native asks show tool arguments; shell asks retain the full command.
Unknown/invalid CLI policies and unsupported flags are rejected before execution.
No repository-controlled permission configuration or shell command classifier is
introduced. Denial bypasses execution; cancellation is checked again after approval
and immediately before dispatch.

Evidence: the allow/ask/deny executor-spy matrix and approval/cancellation tests
in `tests/environment.rs`, and CLI parsing coverage in `src/main.rs`.
