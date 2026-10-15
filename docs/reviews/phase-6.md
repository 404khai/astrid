# Main-based closure audit — 2026-10-15

Disposition: **Closed with explicit limitations** under the maintainer's delegated evidence-based disposition.
Fresh main-based validation: `cargo test --locked` **203 passed, 0 failed, 1 ignored**;
`cargo fmt --check`, strict all-target clippy and `git diff --check` passed.
Phase 7 transition is authorized by the current request; ADR 0015 is explicitly
accepted. This disposition supersedes the historical pending statements below.
Original combined-checkout review below is preserved as historical evidence.

The earlier review overstated the comparison criterion: five scripted tasks plus
one live quality sample showed a baseline, not an exercised comparison. This
thread closes that gap with [two-policy execution replay](../evaluations/phase-6-policy-comparison-2026-10-15.json)
and a [controlled comparison record](../evaluations/phase-6-comparison-2026-10-15.json).
Both configurations pass all five frozen criteria. The runner/rubric/fixture hashes
and configuration are recorded. Scripted success remains execution evidence only.

One concrete new live validation was justified: replay the outstanding-notes task
with recency instead of file-reference selection. [Candidate evidence](../evaluations/phase-6-live-recency-2026-10-15.json)
passes solo frozen-rubric review: three cited actions, washer excluded, boiler date
unknown, observed complete two-file coverage, zero mutations and approvals. Existing
file-reference evidence is reused. Both fit the budget without compaction; latency
and token differences are observations, **not causal policy gains**. One sample per
configuration cannot establish general quality or statistical superiority. Existing
M01 conflict/incomplete samples remain supplementary historical evidence.

The empirical-comparison exit criterion is satisfied at this deliberately small
scope: actual configurations can be replayed, graded and compared with explicit
metrics and evidence kinds. Not every axis needs a new live run to establish that
mechanism. Broader model/prompt/policy comparisons are future experiments, not
claims of this closure. Unavailable cost/true TTFT/decode metrics remain null.

Main lacks the desktop host; restored only the headless read-only policy helper.
Earlier 213-pass evidence belongs to `8baf6af` plus dirty files and includes ten
M02 host tests. It is not this branch's regression evidence. M02 links to research
absent here now point at the verified preserved `8baf6af` source on GitHub.
Recovery stash/snapshot and dirty/untracked user files remain untouched.

Retrospective supplement: comparison execution was the missing assumption in the
original review; frozen fixtures and existing telemetry were sufficient to repair
it without a benchmark platform. Do not carry mock quality scores, desktop test
counts or timing improvements into routing claims. Existing shell authority,
cooperative cancellation, context heuristics and private retention limits remain.
Phase 5/13 Deferred; M02 Paused; no Phase 8 advancement.

---

# Phase 6 — Evaluation review

Date: 2026-10-15
Review: Solo source/diff and evidence review, not an independent adversarial model.
Implementation: Complete for the scoped baseline.
Disposition: Recommend closure with limitations; maintainer closure/Phase 7 transition pending.
Baseline: `8baf6af` plus preserved M01/planning/user changes and this Phase 6/CLI work;
original `feat/m02-tauri-desktop` checkout. No clean standalone commit claim.

## Exit criteria and results

| Contract | Evidence | Result / limitation |
| --- | --- | --- |
| Small reproducible suite | [runner](../../tests/evaluation.rs), [frozen rubric](../../tests/fixtures/evaluation/rubric.json) | Five tasks use temporary fixtures, scripted adapter HTTP and real tools |
| Coding | Read/edit greeting and real `cargo test --offline` exits 0, reports 2 passed, expected repaired bytes | Execution baseline, not autonomous coding-quality score |
| Non-coding | Both frozen household sources reach the provider with complete coverage; bytes unchanged | Scripted output quality unscored; separate live sample below |
| Permissions | Two denied writes/shell requests, zero dispatches, approving handler cannot override Deny | Existing trusted-local policy semantics retained |
| Cancellation | First read triggers cancellation; second tool skipped, no side effect/final answer, state replays | Deterministic cancellation boundary, not all possible OS races |
| Context | Four budgeted requests, eviction, two selections with incomplete summaries, original task and latest read preserved | Existing heuristic/byte and incomplete summary contracts retained |
| Metrics | [machine-readable mock report](../evaluations/phase-6-mocked-2026-10-15.json), actual metadata traces | Wall time, requested/dispatched/outcome counts, per-call usage and summary-selection counts; missing usage/cost null |
| Controlled live task | [live record](../evaluations/phase-6-live-2026-10-15.json) | Frozen outstanding-notes rubric passes in solo review, one model/sample |
| Future comparisons | Report carries task criteria, fixture/rubric/runner hashes and model/policy/context config | Later routing experiments can replay inputs and distinguish evidence kinds; no model ranking claim |

The live CLI used existing subscription authentication and `gpt-5.6-sol`, a copied
synthetic two-file notes folder, read Allow/write Deny/execute Deny, eight-call
ceiling and explicit observability. All three actions and source citations were
correct; completed washer replacement was excluded; boiler date stayed unknown.
Listing/read coverage supported the two-file completeness statement. Wall time
27.04s; 3 model calls, 3 tool calls, 0 summary selections, 0 approvals and 0 changed
paths. Observed usage totals: 3,219 input, 374 output, 0 cached tokens. These are
provider-reported counts, not context estimates. Cost, true TTFT and decode rate
remain unavailable. No user notes or new infrastructure were used.

## Validation

- `cargo test --locked`: **213 passed, 0 failed, 1 ignored** (optional historical
  recorder-overhead experiment). Includes new evaluation and PTY regressions,
  existing M01 tests and preserved Rust host tests. No fresh desktop build claimed.
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `git diff --check`: passed. Local Markdown file-target validation checked
  152 links across 14 authoritative/navigation files with no missing targets;
  runner and rubric SHA-256 values match the recorded mock report.
- Deterministic suite passed twice, including a report-producing replay after the
  full regression run. Five rubric passes on each; timing differences are local
  scheduling/compilation observations, not inference performance improvements.
- Initial focused evaluation emitted one unused-import warning; removed before
  full strict validation. No failing runtime tests observed in this revision.
- `cargo install --path . --locked --debug --offline`: replaced the installed CLI.
  Actual installed `astrid` in a temporary workspace with `TERM=dumb` and an
  explicit model reached `> `, accepted `/quit` and exited 0 without model inference.
  Real-PTY regression also verifies task input, choice selection, no ANSI escapes
  and restored terminal state. Rich composer tests remain green.

## Findings and retrospective

What did we learn? Runtime completion, task rubric success and model ability need
separate labels. The suite reports outcome independently of success; scripted
responses never establish model quality. Cancellation/denial are successful safety
tasks despite cancellation or denied tool outcomes.

Which assumptions were wrong? A usable interactive terminal need not support ANSI
or an 8×8 viewport. Startup was gated on rich rendering. The CLI now uses canonical
line input when rich composition is unavailable, retaining noninteractive rejection.
No user shell setting was changed; this addresses the observed overstrict gate,
not a claim that the user's own terminal environment was inspected.

Which abstractions were useful? Existing runtime events/state replay, real Tools,
local provider HTTP fixtures, pure request accounting and private metadata traces
already provide the necessary seams. No new evaluation crate or runtime abstraction.

Which were premature? A benchmark platform, universal provider interface, cost
estimator without billing evidence, desktop replacement ADR and routing policy
would add unsupported scope. None was added.

What debt would distort the next phase? One live provider/model and one fresh notes
sample cannot establish cross-model coding success, routing superiority or broader
personal-agent reliability. Future routing experiments need comparable live coding
and notes runs/configurations, unsupported-effort handling, and provider continuation
compatibility tests. Existing foundation memory, shell authority and missing telemetry
limits still apply. Missing usage must not become zero in comparisons.

What should not carry forward? Desktop-first ordering, Tauri IPC/embedded-host and
Liquid Glass assumptions, mock timings treated as inference benchmarks, and summary
counts treated as lossless memory. Preserve M02 implementation as paused evidence,
not the future desktop contract. GPUI capabilities await Phase 11 investigation.

## Closure recommendation

The bounded Phase 6 baseline meets its implementation exit criteria: reproducible
execution tasks plus an explicitly limited live quality sample and inspectable
metrics. Recommend close with these limitations. This review does not record a
maintainer closure decision or authorize Phase 7; stop here. Phase 5/13 remain
Deferred, M02 remains Paused and unaccepted. No GPUI, routing, workers or other
successor implementation was begun.
