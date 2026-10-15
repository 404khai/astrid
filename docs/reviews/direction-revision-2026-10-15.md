# Direction reconciliation and M02 pause — 2026-10-15

Maintainer instruction: pause desktop, supersede ADR 0014, select GPUI for future
scoped design, defer Phases 5/13, audit prerequisites then implement Phase 6.
This is explicit Phase 6 authorization, not automatic authorization for Phase 7.

## Checkout and preserved work

Inspected `feat/m02-tauri-desktop` at `8baf6af` (draft PR #13, base
`feat/m01-personal-tasks`). Existing checkout, no worktree. Dirty/untracked M01,
planning, native prototype, build, reference and household files were already
present. No reset, cleanup, staging, commit, branch switch or deletion performed.
The original checkout/branch is retained to avoid separating uncommitted M01
prerequisites or obscuring the desktop handoff. New Phase 6 files are separately
named; historical M02 acceptance evidence is unchanged.

Inspected runtime/model/tools/context/trace interfaces, M01 tests and live records,
M02 review, host/lookup source and adapter layout. Reusable work:

- `src/host.rs`: framework-independent single-workspace/run ownership, session
  locks, approvals, cancellation, bounded/private projections and request receipts.
  Ten historical deterministic tests; potential reuse requires future design review.
- `src/file_lookup.rs`: shared bounded filename lookup independent of terminal/UI.
- Existing headless runtime, permissions, context, trace, sessions and CLI remain
  the foundation. M01 notes guidance/tests/live rubric remain available.

Paused framework work: `apps/desktop` Tauri/web/native adapter and
`experiments/m02-glass` probes. Historical host/projection slices remain implemented;
M02 is not accepted. Final visual/accessibility, active renderer refresh and live
active-run quit acceptance remain gaps per [M02 review](m02-desktop.md). No further
Tauri build is needed for this runtime-only revision. Existing `apps/macos` and
other untracked user artifacts remain untouched.

## Prerequisite audit

| Prerequisite | Evidence / disposition |
| --- | --- |
| Earlier phases | [Foundation closure](foundation-closure.md), phase 1–4 reviews: accepted with explicit limitations; no reopened closure or invented telemetry |
| Current authorized milestone | M01 complete with limitations; M02 explicitly paused by the new instruction, not falsely completed |
| Non-coding runtime | M01 seven deterministic tests and three frozen-rubric live notes scenarios; direct runtime/CLI, no new capability needed |
| Coding | Existing greeting fixture and real tool/edit/test verification; reuse in evaluation |
| Context | Existing pure provider measurement, whole-exchange selection and deterministic incomplete compaction; preserve ADRs 0006/0007 |
| Metrics | Existing event IDs/lifecycles and optional metadata traces; unavailable usage/cost stays null |
| Local inference | Phase 5 explicitly deferred by maintainer; not a Phase 6 prerequisite |
| Authority | 2026-10-15 immediate task explicitly authorizes implementation after this audit and plan; no unresolved consequential contract needed |

No standalone Phase 6 plan existed at audit. The original AGENTS deliverables and
M01 baseline are useful but do not constitute Phase 6 closure. Critique: normal
completion is not success; scripted answers cannot score model ability; local HTTP
metrics are not live inference performance; tiny suites cannot justify broad
routing claims. The [new plan](../plans/phase-6.md) addresses these limits with
frozen criteria, explicit evidence labels, observed runtime metrics and separate
controlled live evidence. No new provider abstraction, daemon or schema is needed.

## Future desktop direction

GPUI is the maintainer's selected framework, motivated by Rust learning. Its
capabilities remain unverified here. At Phase 11 investigate upstream/API stability,
platform support, accessibility, streaming, animation, overlays and host/client
lifecycle against Astrid events/permissions. No replacement ADR now. Superseding
ADR 0014 does not erase its past acceptance or establish a GPUI embedded-host,
web IPC, Liquid Glass or independent-service requirement.
