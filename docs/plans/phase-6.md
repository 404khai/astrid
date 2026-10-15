Current disposition (2026-10-15): **Closed with explicit limitations** after
main-based audit and actual configuration comparison. Fresh regression 203/0/1,
fmt, strict clippy and whitespace passed. See the current [review](../reviews/phase-6.md)
and [comparison](../evaluations/phase-6-comparison-2026-10-15.json).
The original completion record below remains historical (213 included M02 tests).
Phase 7 transition is authorized by the maintainer's new-thread instruction.

---

# Phase 6 — Evaluation

Updated: 2026-10-15
Authorization: Implementation authorized by maintainer's direction revision after prerequisite audit.
Implementation: Complete for scoped baseline
Acceptance: Exit evidence verified in solo review; maintainer closure disposition pending; no Phase 7 authorization.
Prerequisites: [audit](../reviews/direction-revision-2026-10-15.md); Phase 5 Deferred.

## Smallest useful suite

Use the existing Rust test harness, actual OpenAI adapter against local scripted
HTTP, real tools in temporary directories and frozen expected outcomes. No new
crate/provider/routing framework. A report distinguishes task success from run
outcome and records fixture/rubric identity, evidence kind, requested model,
wall duration, model/tool requested/dispatched counts, usage, compactions and cost.
Retain missing metrics as null; scripted token usage must not look live. Real
wall time is fixture/runtime time, not model latency. Keep transcripts private;
only synthetic fixture summaries enter committed records.

| Ticket | Slice / acceptance | State |
| --- | --- | --- |
| P6-01 | Audit prerequisites, critically review baseline, pause M02 and reconcile authority | Done; audit linked above |
| P6-02 | Reproducible coding repair, notes retrieval, denied mutation, cancellation and forced context-compaction tasks; machine-readable per-task metrics and fail on rubric violations | Done |
| P6-03 | Controlled live CLI evaluation on synthetic fixture using existing auth; compare rubric and record available metrics independently of scripted results | Done |
| P6-04 | Full regression/fmt/clippy, suite replay and solo phase review; checkpoint actual evidence and remaining limits | Done |

Frozen task criteria must inspect real artifacts/tool results and event invariants,
not just match a model's final claim. Context evaluation must force compaction and
check that protected task/latest exchange remains while removed content is labeled
incomplete. Cancellation must prevent a subsequent side effect. Permission denial
must hold even when a mocked client would approve. Coding must execute a real
verification command. Non-coding notes must preserve original fixture bytes and
expose the source to the provider; scripted answers are not a quality evaluation.

## Critique and decisions

M01 has useful live notes rubrics but no unified reproducible runtime metrics
suite. Existing invariant tests alone cannot compare model task success. Start
with deterministic execution baselines plus a separately labeled live quality
sample, not a leaderboard. A tiny one-provider suite cannot prove general model
capability or superiority. Use null cost without a known subscription billing
basis; do not consult a price list to invent per-request cost. Keep missing/invalid
usage distinguishable via trace summaries. Reuse existing trace schema rather
than introducing a competing accounting system. No consequential architecture
change requires a new ADR. Cancellation remains cooperative; shell retains
existing account-level authority. Existing fixture failures must be reported.

## Run and evidence

Run the deterministic suite with:

```sh
ASTRID_EVAL_REPORT=/tmp/astrid-phase-6.json cargo test --locked --test evaluation -- --nocapture
```

For the controlled live sample (existing auth, synthetic data only):

```sh
cargo build --locked
python3 scripts/evaluate_m01.py --model gpt-5.6-sol --scenario outstanding --output /tmp/astrid-live.json
```

Live grading remains an explicit rubric review, not a completion flag. Omitting
`--scenario` retains the original three-scenario M01 runner behavior.

Report output is optional and test-specific; no credentials or user notes are used
by the deterministic suite.
Frozen criteria live alongside the suite, reviewed before live runs. Later Phase 7
experiments should reuse fixtures/rubrics and report evidence/model/configuration
explicitly, adding comparable live runs without confusing mock performance with
inference. No routing, effort controls, local backend, worker or GPUI code here.

Final commands/results and retrospective belong to [Phase 6 review](../reviews/phase-6.md).

## Completion evidence — 2026-10-15

P6-01–04 implemented/verified. Root regression **213 passed, 0 failed, 1 ignored**;
format, strict all-target clippy and whitespace passed. Two deterministic suite
runs passed all five criteria, including forced compaction. One fresh controlled
live notes sample passed the frozen rubric in solo review, with observed usage
and no mutations; cost/true TTFT remain null. See [mock report](../evaluations/phase-6-mocked-2026-10-15.json),
[live report](../evaluations/phase-6-live-2026-10-15.json) and
[review](../reviews/phase-6.md) for commands, audit, limitations and closure recommendation.
No automatic Phase 7 advancement.
