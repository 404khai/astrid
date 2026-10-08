# ADR 0007: Deterministic context compaction before model summaries

Date: 2026-10-07
Status: Accepted
Implementation: Implemented
Related: Phase 3, P3-04 through P3-06; ADR 0006; ADR 0002 invocation ceiling
Decision evidence: Maintainer selected both recommendations on 2026-10-07: "use both recommendations, then open a pr for it on a new branch"

## Context

Removing complete older exchanges can lose useful evidence. A shorter replacement
can preserve selected facts, but any summary is incomplete. A model summary adds
inference, streaming, invocation-ceiling accounting, cancellation, and possible
tool requests. A deterministic summary cannot promise equivalent semantic recall.

## Options

1. Produce bounded deterministic condensation with source IDs and explicit omitted
   information. No additional inference; predictable failure and cancellation.
2. Generate summaries through separately identified calls to the current provider.
   Count every call against the existing ceiling. Never execute returned tools.
   More flexible summaries, but more complex lifecycle and failure handling.

## Recommendation

Choose option 1 for this phase. Preserve bounded facts from structured tool
outcomes and bounded excerpts of visible assistant text, with original item IDs,
observed file references, outcomes, and explicit omission markers. Do not interpret
or copy private reasoning into summaries. Treat a summary as task data, with no
authority to override instructions or permission policy.

Compare condensation against full-history and recency-only selection on fixed
fixtures. Record lost relevant evidence rather than claiming better task success.
Keep one rolling bounded summary with explicit lineage so summary history cannot
silently grow into another full prompt. Only commit a replacement selection after
the whole candidate passes budget validation and cancellation checks. Failed or
cancelled compaction preserves the previous valid selection.

## Consequences

Compaction consumes no model calls and dispatches no tools. Important information
may be omitted; inspection exposes that incompleteness and source lineage.
Original ephemeral history remains available to library callers. No persistence,
semantic search, model routing, or additional summary provider is introduced.
Model summaries remain a future replacement decision within Phase 3 if the
bounded comparison demonstrates a need; do not implement them automatically.

## Decision question

Start with deterministic, explicitly incomplete summaries, or use separately
tracked model-summary calls now?

## Validation

Fixtures cover long Unicode output, many older exchanges, multi-tool outcomes,
truncated results, private reasoning sentinels, fixed summary size, preserved
lineage, stale file-read labels, failed fit, and cancellation before commit.
Compare which task-relevant facts survive recency and file-reference selection.

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
