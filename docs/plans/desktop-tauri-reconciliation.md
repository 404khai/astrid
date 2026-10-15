# Historical direction notice — 2026-10-15

Desktop-first sequencing is superseded. M02 is paused; GPUI is the future
Phase 11 direction pending design. Use [canonical roadmap](../roadmap.md) for current scope. The original proposal below is retained as history.

# Tauri desktop and personal-agent architecture assessment

Current navigation (2026-10-12): [canonical roadmap](../roadmap.md),
[checkpoint](../CURRENT.md) and [nearest desktop plan](../milestones/02-desktop.md).
This assessment retains its original evidence and proposed sequence. The canonical
roadmap owns current delivery order; ADR 0014 acceptance remains pending.

Date: 2026-10-10
Status: Assessment and proposed milestones; no implementation authorization
Repository baseline: `5e922f13b2e7438e8e07700e37367a081bef4a88`
Active phase marker: Phase 4; Phases 2–4 administratively closed with limitations

The maintainer selects **Tauri 2**, Rust execution and web desktop presentation,
superseding the native SwiftUI/AppKit direction. Astrid owns its agent harness;
neither a coding-agent shell nor an external computer service replaces it.

## 1. Current implementation and the five boundaries

The repository has one Rust library/CLI package, one hosted model adapter, seven
workspace tools, ordered checked events, context selection, permission policy,
cancellation, metadata traces and private idle conversation snapshots. The
[foundation closure audit](../reviews/foundation-closure.md) records closure and
189 passing tests at its audit baseline. This assessment freshly ran the relevant
runtime/observability/session suites: **45 passed, one overhead test ignored**.
No live inference, Tauri build, Noodle deployment or cross-platform test was run.

Untracked `apps/macos/.build` and `build/macos/Astrid.app` are native build remnants;
no corresponding desktop sources or Tauri dependency/configuration were found.
They were preserved, not treated as a verified client implementation. The mobile
Expo/React Native project remains a demo interface. `references/desktop` is absent
in this checkout; the previous layout contract cannot be verified from that path.
An untracked React/Vite `web/` project appeared during assessment and was also
preserved. Its inspected manifest/entry point contains no Tauri backend/runtime
connection; this evolving frontend is not evidence of completed desktop execution.
Frontend component/tooling choices remain reversible local decisions rather than
a reason to request another architecture approval.

| Boundary | Evidence and current distinction | Required change / timing |
| --- | --- | --- |
| Model provider vs agent harness | `model::ModelProvider::generate` returns a validated response/tool batch; `openai.rs` performs inference and the Astrid runtime executes tools. There is no existing coding-harness delegation integration | Keep inference here. An external coding harness would be an explicitly delegated task executor, with its own side effects/permissions/telemetry; never fabricate its internal tool calls as Astrid's or treat it as a token provider |
| Client vs runtime host | `lib.rs` exports `runtime::run`/`run_in_session`, `PermissionHandler`, sessions and storage; terminal rendering/input live in CLI modules. `main.rs` and CLI-only `sessions.rs` still coordinate run/session ownership | Extract only needed headless orchestration into a Rust host coordinator. Tauri is initially both a UI adapter and the process hosting this coordinator; no Tauri type belongs in core runtime APIs |
| Execution target vs UI | `ToolExecutor::workspace()` requires a local Workspace and native tools/shell act on this machine. A webview is not an execution target | Initial target remains this Mac and selected directory. Browser/computer/remote execution needs explicit target identity and capability mapping when introduced |
| Workspace vs session | `runtime::Session` persists a workspace path; `run_in_session` rejects another workspace. Instructions and Git/change evidence are collected unconditionally | Keep the contract for first desktop/file tasks. Separate conversation identity from optional execution scopes only when an actual multi-scope/non-filesystem task requires it, with migration for ADR 0012 |
| Conversations vs recoverable jobs | `session_store.rs` saves idle snapshots under a one-writer/workspace lock. Metadata traces omit content. Disconnected event consumers do not cancel the running future, but it still needs a live host polling it | Retain idle history. No scheduler, durable accepted-job queue, reconnectable service, active checkpoint or verified side-effect recovery exists. A later background milestone must specify these separately |

Additional couplings are `agent::SYSTEM_PROMPT` naming one coding task, the fixed
native tool definitions in `openai.rs`, and `permissions::capability` mapping known
names to read/write/execute. `PermissionRequest` still carries command/workspace,
not a remote principal or target. These are concrete future extension points;
none justifies a universal device, tool or provider framework today.

Keep guarantees from ADRs 0002–0012: validated completion before dispatch,
runtime-owned sequence/parent IDs, permission checks, cancellation/cleanup,
committed mutations remaining committed, bounded output with coverage, whole
context exchanges, honest missing metrics, independent recorder failures, private
idle saves and rejection of incomplete tool-batch continuation. Normal completion
remains distinct from verified task success.

## 2. Proposed first desktop boundary

```text
Tauri web presentation (main window; future mascot window)
        ↕ restricted commands / authorized ordered projections
Tauri Rust adapter + application-lifetime host coordinator
        → Astrid Rust library → model inference
                             → current local tools
                             → sessions / permissions / context / traces

CLI → same Astrid library, independent client
```

Recommend **embedding the existing Rust library in Tauri's Rust core process**
first. A helper no longer avoids a language boundary: both sides are Rust.
A sidecar adds framing, packaging, version negotiation and cleanup without making
it an independently durable service. A separate service becomes justified when
execution must survive desktop-process exit or multiple independent clients need
one active host. Tauri itself separates the webview from its privileged Rust core.
[Tauri process model](https://v2.tauri.app/concept/process-model/)

The host owns run futures, cancellation handles, pending approvals and idle saves;
views own presentation. Use restricted typed commands for submission, history,
permission replies and cancellation. Version visible DTOs instead of exporting
Rust serde internals. Use authorized Tauri channels for ordered streaming, with
Rust-owned subscriber identity, bounded batches/acknowledgements and explicit
overflow/detachment. Channel delivery alone does not prove bounded memory or
durable replay. Keep runtime draining/cancellation independent of renderer work;
reattach in the same living process through a bounded snapshot with explicit gaps.
[Tauri frontend communication](https://v2.tauri.app/develop/calling-frontend/)

Tauri capability permissions govern UI IPC; Astrid policy governs agent actions.
Neither replaces the other or constitutes OS confinement. App-defined commands
are available to all views by default unless explicitly restricted. Register them
in the app manifest/permission system and validate the invoking view in Rust.
Do not expose generic shell/filesystem/HTTP plugin access to the renderer. Render
file/model/tool text as untrusted content with sanitization, CSP and navigation
restrictions; provider secrets and continuation stay in Rust. Approval IDs bind
to one pending run/tool action and cannot be reused or supplied as claimed authority.
[Tauri capabilities](https://v2.tauri.app/security/capabilities/),
[Tauri CSP](https://v2.tauri.app/security/csp/)

Initial lifecycle recommendation: hiding/closing windows does not drop runs.
Explicit app quit gracefully cancels, awaits cleanup and saves idle state. Renderer
loss detaches its subscription and leaves the host running; unresolved Ask actions
remain blocked until an authorized view answers or the run is cancelled. Host
death loses active work; do not claim rollback/recovery. A bounded host-side
projection may omit historical live data with a visible gap, never fabricate a
complete transcript. This differs from ADR 0013's helper-connection-loss policy
and therefore requires acceptance of [proposed ADR 0014](../adr/0014-tauri-desktop-runtime-boundary.md).

Tauri is selected, not awaiting another framework decision. ADR 0013 was actually
Accepted in Git; the latest instruction explicitly withdraws that implementation
direction. Preserve its history as Superseded, retain applicable runtime invariants,
and propose ADR 0014 for the changed integration/authority/lifetime boundary.
Do not label either the old direction or the replacement newly Accepted here.

Cross-platform path: keep core Rust APIs free of Tauri and platform UI types.
Tauri uses WKWebView, WebView2 and WebKitGTK, requiring platform-specific testing.
Astrid currently uses Unix file permissions/locks, metadata and process-group
signals in auth, tools, changes and session storage. Windows support therefore
requires real platform implementations/tests, not just a Tauri target switch.
Linux shell/process behavior also needs validation. Do not weaken current macOS
cleanup/security guarantees for nominal portability.
[Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/)

### Floating Astrid feasibility and limits

Research snapshot: current Tauri Rust API rendered as **2.12.3**, CLI latest release
metadata `tauri-cli-v2.12.2`; no version was installed or added to Astrid. Choose
and lock actual core/plugin versions during authorized implementation.

| Need | Documented surface | Implication / evidence still needed |
| --- | --- | --- |
| Transparent mascot | Transparent window plus transparent HTML canvas; decorations can be omitted | Transparency is visual, not per-pixel pointer pass-through. Current configuration docs make `macOSPrivateApi` a no-op in 2.12.1+; do not repeat the obsolete requirement to enable it. Validate the chosen release and packaging rather than assume store eligibility |
| Above ordinary windows | `set_always_on_top` | Suitable candidate; not a promise to cover every full-screen, protected or system surface. Test macOS Spaces/full-screen and Windows/Linux window-manager behavior separately |
| Positioning | Window position APIs, monitor enumeration and physical/logical coordinates | Bound position to monitor work areas; test scaling, negative monitor coordinates, hot-plug and restored positions. Webview-relative positioning is not native window positioning |
| Pointer interactions | `set_ignore_cursor_events` toggles window cursor handling; drag and focus controls exist | Whole-window click-through can make the mascot unclickable. Start with a small clickable window and separate compact conversation window; irregular hit regions/toggling need measured platform behavior, not CSS assumptions |
| Summon shortcut | Official global-shortcut plugin | Add only a configured shortcut with conflict/error handling. Linux display-server/global shortcut support and OS focus behavior need actual tests |
| Ambient availability | App-owned windows/tray and hide/reopen lifecycle | Works only while the app host is alive initially; autostart or service lifetime is not implied |

Sources: [Tauri configuration](https://v2.tauri.app/reference/config/),
[Rust window API](https://docs.rs/tauri/latest/tauri/window/struct.Window.html),
[global shortcut plugin](https://v2.tauri.app/plugin/global-shortcut/).

The documented all-workspaces setting is unsupported on Windows, and macOS
skip-taskbar behavior is not the Windows implementation. Linux compositor/window
manager differences prevent a universal overlay guarantee. Native AppKit adapters
may become appropriate for proven macOS focus/panel gaps; keep them below the
presentation boundary, not as primary views.
[Platform window configuration](https://v2.tauri.app/reference/config/#windowconfig)

Animation can project existing facts: idle, active model call, executing tool,
pending approval, cancelled/failed/completed. Do not label animation as observed
private reasoning. Progress/approval views use the same authorized host commands;
expansion opens another view of that state, not another runtime. A feasibility
experiment must test transparent pixels, focus return, pointer interaction,
shortcuts and multi-monitor rendering before committing to a complete overlay.
No screen reading or desktop-control authorization is part of this proposal.

## 3. Minimum noncoding workflow

**Task:** “Read my plain-text household notes in this folder and produce a checklist
of outstanding actions, citing the source filenames and marking uncertainties.”

Use a user-selected folder containing `.txt`/`.md` notes; Git is unnecessary.
Existing list/glob/read/grep tools and final-response text are sufficient. Set
read=allow, write=deny, execute=deny in Rust for this workflow. The grant covers
the selected folder, not an exact-file list unless a separately enforced file
allowlist is added. No email, browser, rename/delete, PDF parsing or new tool is
needed. The selected inference provider receives retrieved content; the UI must
make the chosen folder/provider and effective authority visible before submission.

Minimum implementation changes, not performed here:

1. Generalize the hardcoded operating prompt to delegated tasks while preserving
   trust, inspection, bounded-output and validation rules. Keep coding-specific
   instructions when relevant; no planning framework or semantic index.
2. Wire this read-only workflow through existing immutable RunConfig policies
   and workspace validation. A prompt prohibition alone is insufficient.
3. In the desktop client, show streamed activity, visible conversation, source
   coverage and final checklist. Preserve idle session scoping and explicit
   inherited-history labels; the workflow is not a personal-memory subsystem.
4. Add deterministic fixtures with known notes, conflicting/stale instructions,
   attempted writes/shell and truncated reads. Verify no mutation/dispatch, event
   invariants and honest coverage; separately assess factual checklist quality
   with a bounded live acceptance task.

If saving a checklist is later desired, add an explicitly approved write step;
do not silently widen this workflow. No persisted-state redesign is required now.

## 4. Noodle complementarity

Research identifies `pdparchitect/noodle`, not unrelated projects named Noodle.
Pinned upstream: `cb26ecf632e13ceda316d8b49ce8e0246a1dd70a`; see the
[source-grounded evaluation](../research/noodle-execution-environments.md).

| Project | Complement to Astrid | Proposed disposition |
| --- | --- | --- |
| Computer | VM-backed execution environment, explicit transfers, external CLI/MCP | Candidate typed subprocess/protocol adapter later; do not embed its Swift VM stack in core Rust |
| Browser | Persistent signed-in browser execution, external CLI/MCP | Candidate local action adapter later; account actions require Astrid's own scope/approval and result semantics |
| Hub | Noodle bot/harness hosting, pairing and remote communication | Learn concepts and consider a separately bounded upstream interoperability investigation; no confirmed general Astrid-hosting API |

These are not model providers. An external coding harness is a fourth, distinct
delegation integration. No provider credentials, harness auth copies, Noodle bot
access presets or automatic approval policy should be imported into Astrid.
Do not build replacement virtualization/browser infrastructure while a concrete
adapter investigation can answer the same need. No integration or outreach is
authorized by this assessment.

## 5. Revised vertical milestones

| Order | Reviewable outcome and failure evidence |
| --- | --- |
| T1 | Accept ADR 0014 and authorize first desktop slice separately; embed headless host, one directory writer/active run, CLI preserved; mock submission/events/approval/cancel/history end to end |
| T2 | Deliver notes-to-checklist as the first personal workflow and a small known-answer evaluation; show permissions/provider/coverage and prove mutation attempts denied |
| T3 | Complete desktop application usability: sign-in via existing Rust auth, session restore, window reopen, explicit quit/cleanup, auth failure and storage contention; one separately identified live task |
| F1 | Bounded floating-window feasibility experiment: transparency, topmost, focus/click-through, shortcut, DPI/monitors/Spaces and animation from existing runtime state; no complete overlay or observation/control permissions |
| B1 | Independently supervised runtime host, authenticated local clients, result/state reconnect and orphaned approvals; service death explicitly interrupts jobs unless recovery is separately verified |
| X1 | One justified Noodle Browser or Computer adapter investigation, then scoped implementation only on accepted authority/cancellation/result contracts; no blanket MCP rollout |
| A1 | Expo/React Native Android thin client with pairing/revocation and incremental history; Kotlin for one authorized device capability; validate on low-memory hardware |
| Later | Personal memory, scheduled tasks, local inference/routing from evidence, harness delegation, parallel workers/worktrees, broader ADE and research |

Phases 0–4 contracts/closure remain unchanged; Phase 11 desktop delivery moves
forward; Phase 6 evaluation begins with the first personal workflow; Phase 5 local
inference is optional; Phase 8 lifecycle work precedes worker DAGs; Phases 7/9/12/13
remain deferred. A single adapter may draw from Phase 10 only through explicit
scope authorization. This sequence is a proposal, not a phase-marker change.

Interaction, inference, orchestration and execution are four placements. Initially
they are webview, existing hosted provider, Rust in the Mac app, and local Mac
tools respectively. Future Android interaction can reach a Mac runtime and a
separate authorized execution target without running a large model on the phone.
Do not copy desktop credentials or raw session continuation to mobile.

## 6. Decisions and genuine blockers

**Immediate decision:** ADR 0014's embedded host, restricted web IPC, renderer-loss
and quit policy. Framework selection is already resolved. Implementation requires
separate authorization because this task explicitly requests assessment only.

**Later ADRs:** independent-host lifetime/reconnect; optional execution scopes and
session-schema migration; scoped external actions and cancellation/uncertain
outcomes; remote principals/targets/pairing/revocation; delegated harness custody
and authority; personal-memory retention/correction/deletion. Do not write generic
contracts before their triggering workflow.

No new Phase 2–4 closure gate, Noodle partnership or full memory engine blocks the
first Mac workflow. Desktop code and host DTOs are planned work, not unexplained
architectural blockers. Missing desktop reference assets block fidelity to the old
image, not this assessment. Windows porting and Noodle release/auth compatibility
are blockers only to those later deliveries. Release sandbox/distribution and any
private native API usage require their own decisions when release/overlay evidence
creates a real trade-off.

The recommendation is T1 with T2 as its first product acceptance task. Resolve
the concrete ADR 0014 boundary before implementing; preserve all existing runtime
guarantees and keep the active phase marker unchanged during this assessment.
