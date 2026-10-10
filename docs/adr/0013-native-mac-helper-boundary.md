# ADR 0013: Native Mac client and bundled Rust helper boundary

Date: 2026-10-10
Status: Accepted
Implementation: Not Started
Decision evidence: Maintainer's explicit "Decision: Phase closure and ADR 0013"
on 2026-10-10 accepts the architectural direction with the clarifications below.
Implementation authorization: Pending; acceptance does not advance the active phase
Related: [roadmap reconciliation](../plans/personal-agent-roadmap.md), ADRs 0002–0004, 0009, 0012

## Context

The maintainer establishes native SwiftUI/AppKit as the first application client,
with authoritative Rust execution and an independently supported CLI. A bundled
helper is preferred subject to inspection. Runtime APIs already execute headlessly;
they do not implement a service or wire protocol. Transport, client authority,
session ownership and shutdown behavior affect future clients and require a
documented contract. This ADR applies only to the first local Mac integration.

## Existing constraints

- Runtime event IDs/sequences, permission enforcement and cancellation are
  authoritative; Swift cannot invent runtime transitions.
- Attached bounded event channels require draining. Receiver closure detaches
  observation but does not automatically cancel the run.
- ADR 0012 permits one workspace-store writer and idle snapshots only; permissions
  and auth are not restored. No background recovery or tool replay exists.
- Session/model serialization includes private continuation; public events include
  sensitive content. Neither is an automatically suitable UI/history wire schema.
- Current shell authority is the local account, not a process sandbox. A sandboxed
  Foundation Process child inherits the app sandbox.
  [Apple Process documentation](https://developer.apple.com/documentation/foundation/process)

## Options

1. **Bundled helper with private inherited pipes:** minimal local integration,
   independent Rust execution and bounded versioned framing; app-owned lifetime,
   no independent reconnection after app exit.
2. **Local socket service:** supports independent clients and service lifetime;
   immediately requires endpoint access control, service supervision, reconnection,
   approval routing and ownership decisions absent from today's runtime.
3. **Rust–Swift FFI:** avoids message transport but couples ABI, async callbacks,
   memory ownership and runtime lifecycle to the native process; it does not solve
   background execution and is harder to reuse for other clients.

## Decision

Use option 1 for Mac A, using the existing library from a separate helper
executable. Local development packaging is unsandboxed under existing account
authority; this is not a release distribution decision or a new grant. The CLI
continues using its existing runtime entry points independently.

Accepted contract:

- Use native SwiftUI for application views and AppKit for macOS-specific behavior.
  Keep the existing CLI independently supported; do not substitute Tauri,
  Electron or a web-based desktop shell.
- Rust remains authoritative for runtime execution, session state, tool invocation,
  permissions, cancellation and persistence. Swift owns presentation and local
  interaction intents, not runtime state transitions.
- Swift launches the fixed executable from its own bundle using Foundation
  Process, not a shell command or an executable path from task content.
- Private inherited stdin/stdout pipes carry a fixed-width length prefix and UTF-8
  JSON. Enforce a frame ceiling before allocation, schema validation, bounded
  queues and an initial supported-version handshake. Fail incompatible versions
  before actions; no best-effort fallback to text/CLI parsing. Stderr contains
  bounded diagnostics and never credentials or private continuation.
- Commands carry connection-scoped request IDs; replies identify the request and
  runtime RunId when available. Event messages preserve runtime sequence/parent
  IDs. Define a wire DTO/version rather than freezing Rust serde layouts as public
  compatibility. Duplicate submissions are rejected within the connection;
  never retry uncertain mutations automatically after connection loss.
- One helper owns one selected workspace store and one active run. Rust owns
  session state, idle saves and policy; Swift owns presentation/selection intents.
  Existing CLI/store contention is an explicit error, not bypassed or silently
  isolated into a conflicting copy of the same sessions.
- Bind requesting identity to the launching local connection/account and target
  to this helper/selected canonical workspace. Do not accept a user-supplied
  principal/target assertion as authority. Preserve existing policies and expose
  their effective grants; an unsandboxed process is not blanket delegated consent.
  This local binding provides no remote identity or hostile-local-user protection.
- Provider authentication and local client authorization are conceptually separate.
  Provider sign-in does not authorize a client action or elevate execution policy.
  Never expose credentials or opaque provider continuation through general
  UI/history events; browser sign-in interaction must preserve this boundary.
- Permission replies bind a one-use pending request to the active run/tool.
  Stale/duplicate/unknown replies cannot grant execution. Cancellation wins races
  according to existing runtime rules. No terminal input is used.
- Export paged bounded visible conversation/history DTOs; continuation and auth
  stay in Rust. Preserve truncation/coverage. Never export raw Session/RunResult.
  Optional traces stay metadata-only under ADR 0009.
- Keep command reading and cancellation independently polled from event writing.
  A bounded writer retains order; do not silently drop lifecycle events. A broken
  or persistently stalled transport terminates the attachment and invokes the
  helper's graceful-shutdown policy rather than hanging cleanup indefinitely.
  Exact frame/queue/deadline values are reversible implementation limits.
- Closing the application window must not terminate an active run; the app/helper
  remains alive and window state can reopen onto the existing application state.
  Explicit application quit or irrecoverable helper connection loss (including
  stdin EOF or unusable protocol) may initiate graceful cancellation, cleanup,
  terminal result and an
  idle save, then helper exit. Hard death may lose active work; show interruption,
  never claim rollback or terminal completion without evidence. Save/cleanup
  failure remains explicit. Shutdown does not change runtime detach semantics.
- The first integration is local-only. Do not introduce a network listener, XPC
  service, background daemon, reconnectable service, remote device authority,
  login item, task crash recovery or screen/control access.

## Native UI specification

The maintainer's supplied `references/desktop/desktop-ui-concept.png` is the primary
UI layout specification, not loose aesthetic inspiration. Preserve its information
hierarchy, positioning, proportions, navigation and component structure while
adapting appropriately to native macOS behavior. Inspect the image before preparing
the Mac A implementation plan or implementing views. Do not redesign its layout
or add speculative features from later milestones. Resolve routine visual and
implementation details autonomously when they do not affect runtime contracts.

## Why

This meets task submission, streaming, approvals, cancellation and idle history
with today's guarantees. Pipe ownership limits the connection to the launched
helper without introducing a socket authentication contract. FFI and service
supervision do not improve the first vertical slice enough to justify their costs.

## Consequences and compatibility

The protocol is versioned from inception; app/helper updates ship together.
No current persisted schema or public runtime event meaning changes. Shared
headless session coordination can be extracted from CLI code without importing
terminal presentation. AppKit window management can later host a floating panel
using the same client state; it grants no observation/control capability.

A later persistent service needs a separate ADR superseding only affected
transport/lifetime clauses. Background survival, reconnect and recovery require
new evidence. App Sandbox, signing/notarization, distribution and deployment-target
choices remain separate release work; this ADR does not promise App Store
compatibility.

## Cost of changing later

Medium: protocol DTOs and launch adapter need migration, while execution and
persisted conversation contracts remain reusable. Do not expose this first local
protocol as a remotely trusted API.

## Accepted decision

The maintainer accepted the Mac A direction above: versioned private-pipe bundled helper, one
workspace writer/active run, existing local authority, unsandboxed development
packaging, and graceful shutdown on explicit app quit/irrecoverable protocol loss,
with the explicit native UI, privacy, authority and local-only clarifications.

## Planned validation

Deterministic helper tests must cover handshake rejection, split/oversized frames,
duplicate submit and stale approvals, lossless ordered event delivery, independently
polled cancellation under output backpressure, EOF during permission/tool work,
committed edits during shutdown, store locks, idle-save failure and history privacy.
Native acceptance must cover sign-in, session restore/follow-up, visible streams,
approval denial and cancellation, window reopen and graceful app quit. An abruptly
killed helper reports uncertain active outcome and never replays a stored tool.
Existing locked tests, fmt and clippy remain required; no tests are claimed yet.

## Resolution

Accepted on 2026-10-10 by the maintainer's explicit decision and ten clarifications.
Implementation remains Not Started. Prepare the smallest Mac A implementation
plan and identify genuine blockers in a new thread, grounded in the desktop image.
Implementation requires separate authorization, then vertical slices with
deterministic protocol tests and native application validation. No helper, native
client, service or protocol implementation is authorized by this ADR acceptance
alone. The active phase remains Phase 4.
