# Follow-up context recovery and development direction — solo review

2026-10-18; baseline `8dea6765`, existing `feat/tui-redesign` checkout.
Scoped implementation authorized in the current request; maintainer separately
accepted [ADR 0020](../adr/0020-followup-context-recovery.md).

## Finding and repair

The latest exchange's containing historical group protected an entire previous
submission. Mandatory-fit validation therefore failed before optional eviction
or compaction. The new regression was run against the original HEAD selector:
exit 101 with `ProtectedOverflow`. Restoring the repaired selector permits the
same bounded request without changing any budget default.

The selector separates the latest actual historical exchange and its question
from the earlier prefix. Other historical submissions stay atomic. Event-state
validation permits exactly this additional shape while retaining whole-tool-batch
and protected-content checks. Original session messages and private continuation
are untouched; summary source IDs remain explicit. No persistence field changes.

New tests cover original and follow-up history, a private-reasoning sentinel,
actual mocked follow-up dispatch, retained original bytes, cancellation after
complete tool exchanges, no repeated committed edit and event replay. Existing
incomplete-cancelled-batch rejection remains binding and is revalidated.

CLI presentation now prints the configured model-call ceiling, response planning
reserve and exact request byte utilization/ceiling. The current local context
estimate remains labeled; model-window occupancy is unknown. Live provider usage
projection remains a separate RC-02 ticket; no unavailable count is filled with
zero or inferred from the context estimate.

## Scope review

The [direction plan](../plans/remote-mobile-cli.md) audits actual source/tests and
separates runtime recovery, remote execution, bounded computer use, Expo mobile,
authenticated Host and terminal infrastructure. It reuses the existing mobile app
and identifies unconditional SwiftUI imports as an Android blocker. The proposed
Host API is not implemented or an accepted remote-trust contract. InstaBox's merged
CLI is usable independently; model-tool/scheduler integration remains unimplemented.

Checked the patch for hidden permissions, changed budgets, tool replay, weakened
incomplete-batch checks, private-data summaries, changed v2 formats and implicit
remote capability: none introduced. Summary content remains intentionally
incomplete. The latest exchange, its question or mandatory instructions can still
exceed admission; aggregate multi-tool output bounds are a separate RC-03 issue.
This is not crash recovery or guaranteed remote-process cancellation.

No live inference, credential/private session read, VM operation, machine creation,
port exposure or secret transfer. No branch/worktree switch, staging, commit or
publication. Unrelated initial dirty files and recovery data preserved.

## Validation

Final commands/results are recorded in the
[recovery milestone](../milestones/runtime-recovery.md#validation-record) and
[source-pinned evaluation](../evaluations/runtime-recovery-2026-10-18.json).
Tests use synthetic credentials, local fixtures and mock HTTP, not real accounts.
One initial Python invocation from the repository root failed to import the
script-local `compare_routing` module; rerun from `scripts/` uses its supported
module layout. One new assertion initially assumed all other small history would
be omitted; corrected it to require the oversized exchange's omission while
allowing unrelated small history to fit. Neither failure changed runtime policy.

Disposition: RC-01 passed with explicit limitations: **279 Rust passed, 0 failed,
1 ignored; 2 Python passed; fmt and strict all-target Clippy passed**. RC-02's live
usage projection and RE-01's target-authority contract remain the next independent
work, rather than being silently included here. No phase closure is inferred.
