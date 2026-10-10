# Personal-agent roadmap reconciliation

Date: 2026-10-10
Status: Proposed delivery sequence; product direction established by maintainer
Audit baseline: `cdebe18` on `experiment/mobile`, initially clean worktree
Implementation authorization: Planning/documentation only; Phase 4 remains active

Decision update (2026-10-10): the maintainer approved administrative closure of
Phases 2–4 conditional on verified criteria and accurately documented limitations;
the [foundation audit](../reviews/foundation-closure.md) records the evidence and
resolution. ADR 0013 is separately Accepted with clarifications; implementation
remains Not Started and requires separate authorization. Prepare the smallest
Mac A plan in a new thread using references/desktop as the primary UI layout
specification. The delivery sequence below remains a proposal, not implementation
authorization or automatic phase advancement.

Astrid is an inference-aware personal AI agent runtime that autonomously carries
out delegated tasks across applications, devices, and compute environments. The
harness supplies execution guarantees. Coding and a possible ADE are capabilities
and client experiences. Native macOS is the next recommended application delivery.

## Current architecture and phase audit

One Cargo package exposes a Rust library and an independent CLI. There is no native
Mac client or helper protocol. The Expo/React Native mobile application is an
isolated, iOS-first UI concept with demo state, not a connected agent or validated
Android capability provider.

| Foundation | Actual implementation | Disposition |
| --- | --- | --- |
| Phases 0–1: loop/runtime | Typed run/session/turn/model/tool IDs, checked ordered events, sequential tools, cancellation, headless APIs | Retain; Phase 1 closure recorded |
| Phase 2: execution environment | Path validation, atomic edits, bounded native results, shell timeout/process cleanup, separate output, allow/ask/deny, bounded Git/change evidence | Verified; administratively closed with documented limitations 2026-10-10 |
| Phase 3: context | Provenance, admission heuristic plus exact byte ceiling, whole-exchange selection, deterministic incomplete summaries | Verified; administratively closed with documented limitations 2026-10-10 |
| Phase 4: observability | Optional bounded metadata traces, measured timing, available provider usage, trace/stats inspection | Verified within availability limits; administratively closed 2026-10-10; current phase marker retained |
| ADRs 0010–0012: conversations | Headless follow-ups and private workspace-scoped idle snapshots; writer lock and schema/size checks | Retain; not an active-run checkpoint |

ADRs 0001–0012 are accepted. Historical review headers describe their original
authorization dates; the current AGENTS.md phase marker and later authorization
records control scope. No missing required implementation ticket is identified in
the Phase 2–4 reviews. Closure dispositions were subsequently approved explicitly,
not assumed from implementation completion.
Unavailable true TTFT, prefill/decode metrics, or monetary cost are documented
limits, not grounds to invent metrics. Phase 4 lacks live subscription telemetry
acceptance; this remains an accurately documented closure limitation, not verified
live evidence.

Fresh targeted validation at this baseline: `cargo test --locked --test runtime
--test observability --test session_store --test sessions` passed **45 tests**,
with one intentionally ignored overhead experiment. This verifies relevant current
invariants, not a full-suite phase closure or a Mac integration. The host has
Xcode 27.0 and Swift 6.4; that does not establish a deployment target or release
distribution contract. Review was solo; no independent agent review is claimed.

Preserve completed guarantees: validated model completion before tool dispatch,
checked permission/event ordering, honest cancellation and committed mutations,
bounded output and explicit omissions, protected context exchanges, unavailable
metrics distinct from zero, and no tool replay during session restoration.

## Proposed milestone order

Keep existing phase IDs for historical references. Use named milestones for the
revised delivery sequence instead of renumbering accepted contracts.

| Order | Milestone and acceptance outcome | Original roadmap treatment |
| --- | --- | --- |
| 0 | Foundation disposition completed 2026-10-10: verified Phase 2/3/4 criteria and documented limitations, recorded administrative closure | Phases 0–4 contracts unchanged; Phase 4 marker retained |
| 1 | Mac A: native SwiftUI/AppKit client plus bundled Rust helper; submit a scoped task, stream events, approve/deny, cancel, restore idle history | Bring client portion of Phase 11 forward |
| 2 | Background B: runtime service survives GUI exit; reconnect to truthful state/results and pending approvals; explicitly handle service failure | Bring a small single-run lifecycle slice forward from long-running-task/scheduler/recovery direction; no worker DAG required |
| 3 | Personal C: one noncoding workflow with scoped execution context and explicit principal/target/grants | Add personal execution milestone; bring one application adapter forward only if the chosen workflow needs it |
| 4 | Evaluation D: small reproducible personal and coding task suite; completion, unwanted effects, approval burden, latency, tokens, cost when known | Bring Phase 6 ahead of routing and broader autonomy; keep deterministic tests in every earlier milestone |
| 5 | Android E: connected Expo/React Native thin client; authenticated Mac delegation, reconnect, approvals, bounded incremental history; one Kotlin capability when needed | Add remote/device trust boundary before enabling cross-device execution |
| 6 | Inference F: one practical local backend, then measured routing if tasks justify it | Phase 5 scope unchanged but no longer prerequisite to desktop/mobile; Phase 7 follows evaluation and backend evidence |
| Later | Scheduled delegation, personal memory, more adapters, worker DAGs, worktrees, runtime experiments and Phalanx | Phase 8 split: lifecycle earlier, scheduling/parallel workers later; Phase 9 follows concurrent coding; Phases 10/12/13 remain deferred except an explicitly scoped adapter |

Local inference can move earlier if measurements establish an immediate privacy,
availability, cost, or latency need. Low-memory phones do not require large local
models: interaction, inference, and action execution are separate placements.

## Assumptions to generalize when required

| Current assumption and evidence | Necessary future boundary | First-Mac treatment |
| --- | --- | --- |
| Coding-only `agent::SYSTEM_PROMPT` | Task policy separate from execution environment | Present current repository/file capability honestly; do not claim general personal autonomy |
| `Session` requires a Workspace; `ToolExecutor::workspace()` is mandatory | Conversation identity independent of optional filesystem execution scope | Retain selected-directory sessions; a Git repository is already optional |
| Runtime always captures workspace/Git evidence and loads root AGENTS.md | Environment-specific guidance and evidence sources | Preserve existing evidence and instruction semantics |
| OpenAI adapter publishes fixed native tool definitions; runtime maps names to three capability classes | Available typed capabilities supplied by execution configuration | Retain seven tools; evolve only for the first actual added capability |
| `PermissionRequest` carries command/workspace; trust is implicit local account authority | Explicit requesting principal, execution target and scoped grant, with observable decision/result | Bind helper connection to launching local client/account and current workspace; do not accept arbitrary claimed identities or elevate presets |
| CLI owns session list/selection and run setup | Small shared headless session coordination plus client selection | Reuse store/runtime; extract reusable Rust coordination without exporting terminal modules |
| Original history/provider assembly can grow despite request budgets | Bounded client history projection now; total runtime memory work before mobile-hosted execution | Bound protocol frames, output queues and history pages; do not claim whole-runtime memory bounded |

Full provider continuation and credentials stay inside Rust. UI history is a
bounded projection of visible messages, tool outcomes and coverage, never a raw
serialized Session or RunResult. Metadata traces remain distinct from conversation
storage and are insufficient for content replay or task recovery.

## Smallest complete native Mac slice: Mac A

Use SwiftUI for conversation, activity, sessions and approvals; AppKit for the
window, folder selection and desktop-specific behavior. A Swift-owned window
coordinator can later present the same state in an NSPanel. No floating panel,
screen observation, accessibility control, or global shortcut permission is
required in this slice. NSPanel is Apple's panel mechanism; exact cross-app focus,
Spaces and full-screen behavior require later validation, not an all-windows claim.
[Apple NSPanel documentation](https://developer.apple.com/documentation/appkit/nspanel)

Accepted ADR 0013 integration direction: bundled Rust executable launched by Foundation Process,
with versioned, length-prefixed JSON over inherited private stdin/stdout pipes;
stderr is bounded diagnostics. This is machine protocol I/O in an adapter, not
terminal I/O in the runtime. See [accepted ADR 0013](../adr/0013-native-mac-helper-boundary.md).

Deliver one selected directory, multiple idle sessions, and one active run:

1. Launch helper, negotiate protocol, report model/auth status. Use existing Rust
   sign-in URL/callback support and credentials; Mac opens the browser, Rust
   validates credentials. No token transfer to Swift or new auth backend.
2. Select directory and model, inspect effective policy, restore/list/create/select
   sessions and fetch paged visible history. Report a CLI-held writer lock clearly.
3. Submit a task, stream correlated runtime events and tool output, show honest
   truncation/outcome/context information. IDs and ordering originate in Rust.
4. Resolve one correlated approval or deny it; cancel while permission/model/tool
   work is active. Unknown, stale or duplicate replies cannot authorize dispatch.
5. Save at the existing idle boundary. Restart restores completed conversation
   without replaying tools; an incomplete tool batch remains non-continuable.

Window closure can leave the Mac process/helper alive. App quit or protocol loss
requests graceful cancellation, cleanup and idle save for this first milestone.
Abrupt helper death can lose active work; report interruption and retain the last
successful idle snapshot. This client policy does not change headless runtime
detach behavior. Reopening a window in the same process is not service reconnection.

Acceptance requires Rust helper tests and native UI verification: real event
streaming; allow/deny; cancellation races; restore without replay; unavailable
auth/model; protocol mismatch/oversized frames; stalled output with independently
read cancellation; helper death after a committed edit; store contention; and CLI
regression. Use mocked inference for deterministic integration, then a separately
identified live end-to-end task. Existing locked test/fmt/clippy gates remain.

## ADRs, blockers and recommendation

| Decision | When needed | Status |
| --- | --- | --- |
| Helper transport, versioning, local trust, session ownership and foreground lifetime | Before Mac A implementation | ADR 0013 accepted 2026-10-10; implementation authorization still required |
| Persistent service lifecycle, reconnect, approval ownership with no client, crash outcomes and result retention | Before Background B | New ADR; must explicitly distinguish service survival from crash recovery and supersede affected foreground clauses |
| Bounded delegation and non-workspace execution semantics | Before Personal C changes authority/persisted session meaning | New ADRs as concrete workflow requires; preserve ADR 0004 and explicitly migrate ADR 0012 if needed |
| Pairing, authenticated principals/targets, revocation, remote retries and duplicate side effects | Before Android E can execute remotely | Security/device-trust ADR; local helper protocol is not remote authorization |
| Personal memory provenance, correction, deletion, retention and isolation | Before persistent personal memory | Separate ADR; context compaction is not personal memory |

Foundation closure and ADR 0013 acceptance are now recorded. The remaining
preimplementation gate is a smallest Mac A plan and separate implementation
authorization, including local development packaging under existing account
authority. App Sandbox/release distribution is unresolved: Process children inherit
the parent's sandbox, so a sandboxed helper must not be assumed to preserve current
shell authority. Defer release packaging to its own decision; recommend an
unsandboxed local development app for Mac A, with existing explicit policies.
[Apple Process documentation](https://developer.apple.com/documentation/foundation/process)

There is no demonstrated core rewrite or dependency blocker to Mac A. Implementing
its helper/protocol, session coordinator, safe history projection and native views
is planned work. Background service supervision, reconnectable state and crash
recovery do not exist yet. A future service can investigate SMAppService launch
agents, which require their own lifecycle/user-approval design.
[Apple SMAppService documentation](https://developer.apple.com/documentation/servicemanagement/smappservice)

Keep Expo/React Native for mobile. Add Kotlin modules for individually authorized
Android capabilities, overlays and services. Android overlay access is a separate
user grant, and foreground services have launch/lifecycle restrictions. Validate
memory/scrolling/reconnect on actual low-memory hardware; existing iOS demo evidence
does not establish Android readiness. Do not assume cross-app overlay parity on iOS.
[Android overlay settings](https://developer.android.com/reference/android/provider/Settings#ACTION_MANAGE_OVERLAY_PERMISSION),
[Android foreground services](https://developer.android.com/develop/background-work/services/fgs)

Defer generic device registries, universal providers, multi-agent execution,
automatic routing, semantic indexing, broad desktop control, always-on memory,
ADE editors and release-store distribution until a scoped task justifies them.

**Recommendation:** plan **Mac A** in a new thread under accepted ADR 0013, then
obtain separate implementation authorization. Build a working native client of today's runtime, then establish
Background B before promising delegated work that survives app exit. No next-phase
implementation is authorized by this proposal. Closure comes from the separate
maintainer decision and verified foundation review.
