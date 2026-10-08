# Phase 3 adversarial plan and first-slice review

Date: 2026-10-07
Baseline: `c62df7a` plus the implementation worktree
Reviewer: Separate adversarial agent `/root/phase3_adversarial`
Scope: Read-only plan/code review; no live-provider validation

The maintainer requested an adversarial critique followed by implementation.
The separate reviewer examined the plan, accepted ADRs, and runtime/provider
code. Review found that the proposed first slice mixed measurement with admission
before the admission contract was settled.

## Material plan findings

1. Encrypted reasoning byte length cannot establish its provider token cost.
   Measure serialized bytes, label heuristics, and retain token uncertainty.
2. Removal must keep a completed assistant response, its continuation, and all
   associated tool outcomes together, including denied/failed outcomes. Global
   reused-call-ID checks must survive removal.
3. Model-generated summaries need explicit invocation identity, ceiling
   accounting, cancellation, and failure semantics. Summaries must never execute
   tools. Failed/cancelled compaction must preserve the last valid selection.
4. A prompt budget does not bound Session history, response assembly, streamed
   text, or continuation memory. Metadata snapshots must not copy prompt bodies.
5. Accounting belongs to actual adapter preparation, including tool definitions
   and serialized outcomes, rather than a separate runtime approximation.
6. Snapshot events must be correlated and ordered before dispatch. A preparation
   record does not prove dispatch; rejected-request invocation counting remains
   an open admission-design question.

Disposition: Implement passive accounting/inspection as P3-02. Keep admission,
removal, compaction, and retained-memory contracts open for maintainer grilling.
No recommendation above is an accepted design answer by itself.

## First-slice implementation review

The reviewer inspected context/model/provider/runtime/events/CLI changes and
reported no material blocker. OpenAI counts and sends the same prepared Value;
aggregate metadata contains no private continuation contents; projection checks
model correlation, lifecycle, and duplicate metadata.

Reviewer-requested fixtures cover wire-size agreement with Unicode and escaped
outcomes, opaque continuation privacy, cancellation during stalled publication,
authentication failure after preparation, duplicate replay, and providers omitting
metadata. The root agent authored and ran the fixtures; the reviewer did not
independently execute them.

Explicit limitations: the heuristic excludes the whole assistant category when
it contains reasoning. Preparation/counting remain synchronous over unbounded
existing inputs. This is request measurement, not a memory bound, exact provider
token count, admission rule, context selection, or compaction implementation.

Acceptance evidence is owned by [P3-02](../plans/phase-3.md#p3-02-show-what-the-prepared-model-request-contains).
Root validation passed: 97 deterministic tests, formatting, Clippy with warnings
denied, CLI help, and local Markdown link targets. No live-model or interactive
terminal acceptance was performed for this slice.
This review does not close Phase 2 or complete Phase 3.

## Full context-engine implementation review

After the maintainer accepted ADRs 0006/0007, the reviewer challenged budget,
selection, compaction, event replay, and cancellation. Three material findings
were fixed before publication:

1. Replay now rejects duplicate selection per turn, protected eviction, split
   complete exchanges, invalid summary lineage/bounds, and prepared metadata that
   differs from accepted selection.
2. Summary lines preserve requested file identity, path-label truncation, result
   truncation, and coverage before bounded content excerpts.
3. Accounting checks cancellation after synchronous measurement and before error
   commit, preserving cancellation and previous selection even when accounting
   returns overflow or error.

The reviewer independently ran six `tests/context_budget.rs` tests and five
`context::tests` fixtures; all passed. Final read-only review reported no material
blocker. Full-suite/format/Clippy/CLI/link evidence remains root-owned in
[the Phase 3 review](phase-3.md) and [plan](../plans/phase-3.md).

Remaining limits: uncertain provider token fit, incomplete/stale summaries,
unbounded total retained history, cooperative cancellation between synchronous
measurements, and relevance comparison measuring retention rather than task
success. Earlier first-slice statements above are historical evidence, not the
scope of the final implementation. No live-provider verification is claimed.
