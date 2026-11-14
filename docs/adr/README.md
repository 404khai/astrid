# Architecture decisions

Accepted ADRs describe contracts. Proposed ADRs contain recommendations awaiting
maintainer resolution; they do not authorize implementation or advance a phase.
See the [development process](../development.md) for the lifecycle and template.

| ADR | Decision | Status | Implementation |
| --- | --- | --- | --- |
| [0001](0001-phase-0-execution-contract.md) | Phase 0 execution contract | Accepted | Implemented; Phase 1 changes are in 0002 |
| [0002](0002-phase-1-runtime-and-event-model.md) | Runtime and event model | Accepted | Implemented |
| [0003](0003-bounded-tool-output.md) | Bounded tool output and cleanup under backpressure | Accepted | Implemented |
| [0004](0004-execution-authority.md) | Permission policy and execution authority | Accepted | Implemented |
| [0005](0005-workspace-change-evidence.md) | Git baseline and workspace change evidence | Accepted | Implemented |
| [0006](0006-context-admission-and-selection.md) | Estimated context admission and whole-exchange selection | Accepted | Implemented |
| [0007](0007-context-compaction.md) | Deterministic compaction before model summaries | Accepted | Implemented |
| [0008](0008-observability-control.md) | Optional observability and settings | Accepted | Implemented |
| [0009](0009-persistent-trace-contract.md) | Bounded local metadata traces | Accepted | Implemented |
| [0010](0010-interactive-sessions.md) | Interactive in-memory sessions | Accepted | Implemented |
| [0011](0011-permission-modes.md) | Permission modes and client display | Accepted | Implemented |
| [0012](0012-local-session-persistence.md) | Workspace-scoped local session persistence | Accepted | Implemented; deterministic validation passed |
| [0013](0013-native-mac-helper-boundary.md) | Native Mac client and bundled Rust helper boundary | Superseded; native direction withdrawn | Not Started |
| [0014](0014-tauri-desktop-runtime-boundary.md) | Tauri desktop and Astrid runtime hosting boundary | Superseded 2026-10-15; accepted 2026-10-14 retained | Paused; partial implementation preserved |
| [0015](0015-visible-history-routing.md) | Explicit visible-history model handoff | Accepted 2026-10-15; one-shot restriction superseded by 0016 | Visible handoff implemented; final checks passed |
| [0016](0016-persistent-routing-provenance.md) | Persistent dispatch provenance, conservative v1 migration, resumable visible-history routing | Accepted 2026-10-16 | Implemented; final checks passed |
| [0017](0017-bounded-task-dag.md) | Bounded process-local DAG, exclusive task ownership and scoped cancellation | Accepted 2026-10-16 | Implemented; [review/evidence](../reviews/phase-8.md) |
| [0018](0018-worker-authority-and-results.md) | Read-only worker grants and attributed visible-result reconciliation | Accepted 2026-10-16 | Implemented; [review/evidence](../reviews/phase-8.md) |
| [0019](0019-batch-events-and-traces.md) | Explicit batch projection delivery and metadata trace correlation | Accepted 2026-10-16 | Implemented; [review/evidence](../reviews/phase-8.md) |
| [0020](0020-followup-context-recovery.md) | Follow-up selection without whole-transcript protection | Accepted 2026-10-18 | Implemented; scoped amendment to 0006/0010; [evidence](../milestones/runtime-recovery.md) |
| [0021](0021-remote-target-authority.md) | Foreground exact-target remote execution authority | Accepted 2026-10-21 | Implemented; [review/evidence](../reviews/instabox-tool-adapter-2026-10-21.md) |
| [0022](0022-screenshot-observation.md) | Separate screenshot observation and ephemeral image handling | Accepted 2026-10-21 with explicit gates | Fixtures implemented; scoped runtime/transport/vision amendment in 0024; image persistence/live trial gated |
| [0023](0023-bounded-desktop-actions.md) | Separate desktop action grants and native-frame admission | Accepted 2026-10-21 with clarifications | Controller implemented; foreground integration amendment in 0024; live trial gated |
| [0024](0024-foreground-computer-use-integration.md) | Real foreground CU and ephemeral per-frame image disclosure | Accepted 2026-10-21 | Foreground integration implemented; mock validation, exact live trial pending |
| [0025](0025-image-upload-response-deadlines.md) | Separate bounded image upload and visual response deadlines | Accepted 2026-10-23 | Implemented; ten-second upload/image and sixty-second response boundaries mock-validated; other gates unchanged |
| [0026](0026-presence-host-and-protocol.md) | Native macOS presence host and presence protocol v1 (v2 D1 a) | Accepted 2026-11-11 for the presence host | V2-01 implemented in `apps/presence-mac`; 29 Swift tests; live overlay behaviour pending maintainer verification |
| [0027](0027-local-look-and-presence-tools.md) | Local Mac look (one combined capture+disclosure approval), host-side capture and presentation tools | Proposed 2026-11-11 | Not started; V2-02 blocked on decision |
| [0028](0028-read-only-mail.md) | Read-only mail: Apple Mail (Spotlight) and Gmail (gmail.readonly OAuth, Keychain) | Accepted 2026-11-11 | Implemented with fixtures/mocks; live sign-in pending maintainer |
| [0029](0029-resident-presence-timers.md) | Resident presence host owns timers and alarms (D5 a) | Accepted 2026-11-11 | Timers/alarms implemented; EventKit reminders/calendar next |
| [0036](0036-live-canvas.md) | Live canvas: streamed, validated shape block drawn by the presence app (D3 b, general vocabulary) | Accepted 2026-11-13 | V2-04 first slice; parser tests and browser render checked; live voice→canvas pending maintainer |
| [0038](0038-operator-walkthroughs.md) | Operator: guided walkthroughs, one look per press (D2 a), "Show me" and bounded "Do it" on this Mac; Claude via Agent SDK, plain-ask handoff and voice (amended) | Accepted 2026-11-14 | V2-05; mock inference, fake/synthetic capture; live screen, model and input trials pending maintainer |
| [0037](0037-native-desktop-app.md) | Native desktop app (SwiftUI, Liquid Glass) beside presence; chats on disk with diagrams; bounded ask history | Accepted 2026-11-13 (test run) | `apps/astrid-desktop`; Radian-style sidebar, Rex-style settings and themes (amended); 19 Swift + 2 Rust tests; window captures and coexistence checked with a fake runtime child; live model asks pending maintainer |
| [0039](0039-phone-companion.md) | Phone companion: Android overlay with intro wave and canvas; QR-paired, sealed LAN link to the desktop app with chat sync; phone asks without mail | Accepted 2026-11-14 | V2-06; `mobile/` Android + `modules/astrid-overlay`, `PhoneLink` in the desktop app; live phone asks pending maintainer |
