# Phase 7 — Deterministic direct-inference routing

2026-10-16 follow-up: this document preserves the initial implementation checkpoint.
The one-shot/v1-only restriction below is superseded by accepted ADR 0016. See the
[active follow-up plan](phase-7-followups.md) and [current review](../reviews/phase-7-followups.md).

Authorization: Maintainer's 2026-10-15 new-thread instruction, conditional on
Phase 6 evidence closure. ADR 0015 handoff accepted explicitly in this thread.
Implementation: Complete for the accepted one-shot contract.
Review: Recommend closure with explicit limitations; [review](../reviews/phase-7.md).
Phase 8 remains unauthorized.

## Review and grilling

Phase 6 now compares two configurations on frozen fixtures with separate live
quality grading. It does not demonstrate a better model/policy. Reuse those
fixtures; keep null cost/missing telemetry and do not rank scripted models.

| Failure question | Disposition |
| --- | --- |
| Can opaque reasoning follow another model? | No assumption; accepted [ADR 0015](../adr/0015-visible-history-routing.md) uses visible history |
| Does switching replay a mutation? | Only request history is rebuilt; real tool dispatch remains exclusively in runtime |
| Does a route imply task success? | No; per-call terminal outcome differs from independently graded task success |
| Can provider silently ignore unsupported effort? | Validate exact supported model/effort before dispatch; no fallback |
| Can inference call a coding-agent harness? | No; existing OpenAI Responses adapter is direct inference only |
| Can routing alter resumed session schema? | Per-run config and request view only; idle serialization unchanged |
| Does a new model grant authority? | Same per-run policy/handler, workspace and cancellation boundaries |

## Small slices

| Ticket | Acceptance | State |
| --- | --- | --- |
| P7-01 | Typed capability, effort, route/policy/config validation; exact adapter effort serialization and measurement; unsupported effort rejects before auth | Done |
| P7-02 | Runtime configured stage boundaries, visible-history adapter view; route event/trace metadata with reasons and per-call outcomes; CLI flags | Done |
| P7-03 | Multi-model scripted real-tool acceptance; private-state exclusion, budget agreement, denied operations/cancellation, snapshot compatibility; bounded live smoke if needed | Done |
| P7-04 | Required cargo checks; solo diff/invariant review; document evidence, limitations and exit disposition | Done |

Configuration is explicit: initial model remains required; optional stages select
another configured model from a specified model-call number onward. Classes are
caller-supplied intent labels, not inferred competence. No model invocation solely
for classification and no new automatic step after a final response. Multiple
model classes must be exercised by one genuine tool-loop run in acceptance.

Provider effort support starts narrow from verified official model documentation;
unknown model identifiers retain legacy provider-default behavior when no explicit
effort is requested. No future backend, broad universal provider trait, ML policy,
worker, scheduler, daemon, desktop or harness adapter is part of this phase.

## Slice checkpoint — 2026-10-15

Typed routes and exact effort validation implemented, plus core per-call route
publication, trace metadata/summary projection and CLI `--routing`/`--reasoning-effort`.
Five focused scripted/real-tool acceptance tests pass. The first attempt failed a
test assumption that missing usage meant no Usage object; fixed the assertion to
check null counters, retaining production unavailable semantics. No model-quality
claim or live routed evidence yet. Current full regression remains the Phase 6
203/0/1 baseline until post-routing checks run.

Solo review found a consequential unresolved edge: routed original history retains
private responses without source-model provenance in the existing snapshot schema.
A later native follow-up cannot safely resume that history. Maintainer question
pending: one-shot inspection-only routed results with explicit save/resume rejection
(recommended), or separately approve provenance migration. Do not claim resumable
routing until resolved. No dependent persistence change implemented yet.

## Accepted compatibility refinement — 2026-10-15

Maintainer explicitly approved one-shot routing with inspection-only returned
history. Guard now rejects routed save/serialization and routed/native continuation;
`run_in_session` rejects new multi-stage configuration. Native/effort-only snapshot
schema stays unchanged. Focused acceptance now checks rejected save retains prior
snapshot and original private messages survive inspection. This supersedes the
pending compatibility question above. Final post-refinement validation pending.

One fresh live synthetic notes task passed with Sol/low followed by Terra/low:
3 model calls, 4 tools, zero changes/approvals; usage 3313 input/278 output/0 cached.
[Evidence](../evaluations/phase-7-live-2026-10-15.json). This path was already fresh
and does not persist history, so the guard adds no live inference validation gap.

## Final checkpoint

Final regression **211 passed/0 failed/1 ignored**, fmt, strict all-target clippy and
whitespace passed; [pinned evidence](../evaluations/phase-7-validation-2026-10-15.json).
Both backpressure test synchronizations were corrected for the added route event;
a reproduced duplicate-descriptor lock lifetime gap was fixed with explicit owner
unlock and a deterministic invariant test. Full native session/permission/context
regressions pass. Six routed acceptance tests and ten evaluation execution cases
pass. Earlier pending entries above are historical slice checkpoints. Implementation
is complete and solo review recommends closure with the stated limitations. Stop;
no automatic Phase 8 advancement.

## PR publication — 2026-10-15

[PR #14](https://github.com/404khai/astrid/pull/14), implementation `ef1dbb8`, targets
`main` from the existing branch. An exported exact staged snapshot passes 211/0/1,
fmt and strict clippy; 208 local documentation targets resolve. Paused desktop code
and unrelated local artifacts are excluded. Compact CLI context usage is a
recommended follow-up, with estimated budget/provider usage/model capacity kept
explicit; it is not implemented by this PR. No Phase 8 transition is inferred.
