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
