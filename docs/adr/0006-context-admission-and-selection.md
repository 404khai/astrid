# ADR 0006: Estimated context admission and complete-exchange selection

Date: 2026-10-07
Status: Accepted
Implementation: Implemented
Related: Phase 3, P3-03, P3-06; ADR 0002 conversation and invocation lifecycle
Decision evidence: Maintainer selected both recommendations on 2026-10-07: "use both recommendations, then open a pr for it on a new branch"

## Context

The adapter measures exact serialized request bytes, but opaque encrypted reasoning
does not expose its model-context token cost. Visible-text accounting alone misses
tool definitions, JSON-encoded outcomes, instructions, and private continuation.
A complete assistant response can request several tools. Removing it separately
from its outcomes can produce an invalid provider conversation.

## Options

1. Require exact provider accounting before any budget-based removal. Honest,
   but this blocks selection while opaque accounting remains unavailable.
2. Use a clearly labeled non-opaque JSON size heuristic and an independent exact
   serialized-byte ceiling. Admit requests under that local policy while keeping
   full provider token fit unknown. Remove older complete exchanges as units.

## Recommendation

Choose option 2. Represent ContextBudget and ContextSelection explicitly. Reserve
a configured allowance for a response by subtracting it from the estimated
context allowance; this is planning headroom, not an enforced provider output
limit or a claim about model capacity. Validate positive input allowance and byte
ceiling before work starts. Numeric defaults are reversible configuration choices.

Protect all operating/repository instructions, the original task, and the latest
complete assistant/tool exchange. Each other selectable exchange contains the
whole assistant response/continuation and every associated terminal tool outcome,
including denial/error/timeout. Never split an exchange to fit. Global reused-call
ID checks remain independent of selected history.

Measure through the adapter. If protected content cannot pass the configured
local policy, fail before provider authentication/dispatch. Selection does not
authorize more reads or modify file/permission state. Record retained/removed
item IDs and reasons. Retain original history separately in the ephemeral Session
so removal from the next request does not erase observed execution evidence.

## Consequences

Request admission has explicit estimates and an exact byte ceiling. Provider
context overflow remains possible; do not display an exact total or guaranteed
fit. Unknown accounting stays unknown. Prompt selection is bounded by configured
policy; retained conversation/runtime evidence and provider response assembly do
not acquire a total-memory bound. No persisted context or trace store is added.

This extends ADR 0002's full-history request construction and Phase 1 no-context
scope. Do not silently change its invocation ceiling: rejected preflight must
not count as an inference invocation. Cancellation before selection commit leaves
the previous selection unchanged; accepted events remain ordered under existing
backpressure rules.

## Decision question

Use estimated admission plus an exact byte ceiling and whole-exchange removal,
or require exact provider token accounting first?

## Validation

Fixtures must prove: oversized protected instructions/task/latest multi-tool
exchange prevents dispatch; older denied/failed outcomes stay with their request;
Unicode/escaped outputs and tool definitions enter accounting; private reasoning
stays unknown/private; removed call IDs cannot be reused; cancellation before
commit preserves selection; event inspection agrees with the dispatched request.

## Resolution

Accepted on 2026-10-07 by the maintainer's explicit selection of both
recommendations. Retain the alternatives above as decision history.

## Implementation evidence

The context engine performs pure adapter preflight, complete-exchange selection,
and bounded deterministic compaction; runtime events inspect/replay committed
selections. Source provenance and global tool IDs remain independent of selected
history. `context::tests`, `tests/context.rs`, and `tests/context_budget.rs` cover
wire accounting, privacy, protection, summary coverage, failure, cancellation,
replay, and retained evidence. The [Phase 3 review](../reviews/phase-3.md) owns
final validation results and limits.


## Follow-up session extension

[ADR 0010](0010-interactive-sessions.md) extends selection for multiple user
submissions: protect original/current tasks and the latest exchange; keep each
older follow-up submission with its entire response history. Current-submission
exchanges remain indivisible assistant/tool units. Request accounting and
admission semantics are unchanged.
