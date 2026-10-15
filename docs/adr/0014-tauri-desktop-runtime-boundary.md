# ADR 0014: Tauri desktop and Astrid runtime hosting boundary

Date: 2026-10-10
Status: Superseded — 2026-10-15 (accepted 2026-10-14)
Implementation: Paused; partial implementation preserved
Decision evidence: After the precise host/IPC/lifecycle recommendation was presented,
the maintainer instructed “So what’s the way forward start implementation” on
2026-10-14. This accepts option 1 and the contract below.
Related: ADR 0013 withdrawn native direction; ADRs 0002–0012; [assessment](../plans/desktop-tauri-reconciliation.md)

## Context

Astrid is its own personal-agent harness. The maintainer selects Tauri 2, Rust
authoritative execution and web presentation, superseding SwiftUI/AppKit as the
primary desktop UI. The existing Rust library can already run headlessly. No
Tauri integration, independent service or recoverable-job subsystem exists.
Tauri's Rust core can host the library directly, so a bundled Rust helper must
be justified anew rather than inherited from the prior Swift language boundary.

## Existing constraints

- Core runtime remains usable by the CLI and other hosts without Tauri types.
- Model completion, tool lifecycle/permissions, IDs/events, cancellation, context
  selection, recording and idle persistence retain accepted meanings.
- Public events can include sensitive task/tool content; raw sessions/results
  include private continuation. Neither is a safe automatic frontend DTO.
- Existing session storage is workspace-scoped, locked and idle-only; a second
  desktop/CLI writer must fail clearly. UI reload is not crash recovery.
- Renderer authorization, provider authentication and tool authority are separate.
  Tauri plugin access controls are not Astrid's tool permission policy or an OS
  sandbox. Existing shell Allow remains broad account-level authority.

## Options

1. **Embed the Astrid library in Tauri Rust core:** reuse Rust interfaces directly;
   one app-lifetime host owns runs and snapshots, independently of windows.
   No execution after host-process exit or host crash recovery.
2. **Bundled Rust sidecar:** separate host process/crash boundary, at the cost of
   packaging, wire schemas, version negotiation and supervisor design. Being a
   child process does not establish durable background execution.
3. **Independent local service immediately:** natural multiple-client/background
   destination, but requires authenticated transport, service supervision,
   reconnect/result retention and orphaned approval contracts now.

## Recommendation

Choose option 1 for the initial macOS slice, with an application-lifetime Rust
coordinator and a narrow Tauri adapter. Keep CLI/library independence. Do not add
sidecar, socket listener, daemon, universal device model or speculative scheduler.

Accepted contract:

1. Tauri hosts the runtime, not model inference in JavaScript. The host starts and
   continues polling Rust run futures independently of view/component lifetime;
   never hold a global host mutex across model/tool/permission awaits. Keep
   blocking filesystem/accounting work away from the UI event loop as practical
   without claiming stronger cancellation than the existing runtime provides.
2. Expose versioned visible DTOs through a small typed command surface. Submission
   creates a host-owned request/run association; duplicate request IDs do not
   start another run. An uncertain side effect is never automatically retried.
   One selected workspace store and one active run suffice initially.
3. Authorize commands with an explicit app command manifest and narrow window/
   webview capabilities, plus host-side invoking-view checks. App commands are
   otherwise exposed broadly by default. Use bundled local UI, sanitized task
   content, restricted navigation and CSP. No renderer generic shell/fs/network
   plugins or remote page with Astrid command authority.
   [Tauri capability documentation](https://v2.tauri.app/security/capabilities/)
4. Bind the requester to the authorized local view/account and execution target
   to the host's workspace handle. Renderer-supplied principal/target/policy text
   is not authority. Rust validates client intents and resolves actual RunConfig
   permissions; provider login never elevates them. Model/file/tool content cannot
   create IPC intents or permission grants.
5. Bind each approval reply to one pending run/tool request and authorized client.
   Reject stale, duplicate and wrong-run replies. Cancellation retains existing
   race behavior. No browser-side approval auto-grant or terminal fallback.
6. Stream authorized ordered UI projections through Tauri channels, retaining
   Rust event parent IDs/sequences. Channels do not by themselves bound queues,
   authorize every subscriber or persist replay. Use bounded batches and a finite
   outstanding-data allowance/acknowledgement; detach a stalled subscriber and
   label coverage gaps. Keep the host draining authoritative events and processing
   cancellation regardless of UI backlog. Never silently relabel projection loss
   as a complete runtime history or drop core lifecycle transitions.
   [Tauri channel documentation](https://v2.tauri.app/develop/calling-frontend/)
7. History/export DTOs omit auth credentials and opaque continuation; Rust retains
   provider replay privately. Paginate/cap history and tool text with explicit
   omissions; do not serialize Session/RunResult wholesale. Do not store secrets
   or authoritative conversations in web localStorage. ADR 0009 metadata trace
   privacy and ADR 0012 snapshot schema/writer guarantees remain unchanged.
8. Closing/hiding a window keeps the host and run alive. Renderer loss detaches
   its stream; a reopened authorized view can obtain bounded current state while
   the host lives. Pending Ask requests remain blocked until answered or cancelled.
   Explicit application quit requests cancellation, polls cleanup to terminal
   outcome and attempts an idle save before exit. Display cleanup/save failures
   honestly; forced process death can lose active work and committed actions may
   remain. No survival after app exit, active-job recovery or automatic tool replay.
9. A future mascot is another client window, not a runtime instance. Window
   positioning, focus, transparent rendering, shortcuts and animations belong to
   the adapter. Approval commands require explicit authorization for that view
   when introduced; never grant it all main-window commands by wildcard. No
   screen observation or desktop action capability follows from window creation.
10. Target macOS first, using web presentation rather than SwiftUI primary views.
    Native macOS integrations can remain narrow adapters. Tauri offers a Windows/
    Linux UI path; Astrid's Unix-specific tools/auth/storage still need portability
    work and tests before claiming support. Platform changes must preserve current
    path, persistence and cancellation guarantees.

## Why

Tauri already provides a privileged Rust core and separate webview presentation.
Direct library hosting avoids an unnecessary Rust-to-Rust protocol while retaining
the runtime boundary. The host coordinator can later move behind a service
transport when real background/multiple-client requirements justify it.
[Tauri process model](https://v2.tauri.app/concept/process-model/)

## Consequences and compatibility

The Tauri technology choice is established; this ADR does not request it again.
Acceptance concerns the host, UI authority and lifecycle semantics. No accepted
runtime event meaning, permission preset or persisted conversation schema changes.
Web IPC adds a trust surface absent from the Swift/private-pipe plan; explicit
view authorization and untrusted-content rendering require deterministic tests.
Application exit still ends execution. Service survival/reconnect and crash/job
recovery require a later ADR and measured failure evidence, not rebranding the
desktop process as a daemon. No framework-inferred speed/memory claim is made.

## Cost of changing later

Medium: the coordinator and DTOs can transfer to a service, but transport,
authentication, approval ownership and lifecycle need a deliberate migration.

## Decision required

Accept the initial embedded Rust host with the restricted local web IPC/privacy
contract above, continuing across renderer/window loss but gracefully cancelling
on explicit app quit, while deferring independent service/recovery?

## Planned validation

Mocked end-to-end tests cover submission, ordered stream projection, event/state
correlation, forged/stale approvals, duplicate submission, cancellation during
permission/model/tool execution and stalled renderer output, view authorization,
untrusted HTML/links, secret/continuation sentinels, lock contention and idle save
failure. Close/reopen and renderer reload leave the same run alive; explicit quit
preserves honest cleanup/committed-action outcomes. Host death must not imply
rollback/recovery. Retain locked Rust tests/fmt/clippy, frontend checks and actual
macOS application acceptance. Current implementation evidence is recorded in
[the M02 review](../reviews/m02-desktop.md).

## Original proposed resolution (historical)

Pending at proposal time. Tauri selection supersedes ADR 0013's native/helper direction by explicit
maintainer instruction. This proposed replacement boundary and its implementation
require separate acceptance/authorization. The Phase 4 marker stays unchanged.

### Acceptance — 2026-10-14

The maintainer answered the explicit recommended-contract decision by instructing
implementation to start. Option 1 and clauses 1–10 are accepted for scoped M02.
The former pending resolution above remains historical. No independent service,
recovery, M03, cross-device trust or new OS tool authority follows from acceptance.
Material/desktop implementation and product acceptance are separate evidence.

## Supersession — 2026-10-15

The maintainer paused M02 and withdrew Tauri as the intended desktop technology.
The accepted contract above is retained as history, not a future GPUI host/IPC
requirement. GPUI is selected pending Phase 11 investigation and scoped design;
no replacement desktop ADR or implementation is authorized. Runtime authority,
CLI independence and ADRs 0001–0012 remain. See [direction record](../reviews/direction-revision-2026-10-15.md).
