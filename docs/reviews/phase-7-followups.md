# Phase 7 follow-up review — 2026-10-16

Disposition: requested follow-ups complete; recommend closure with explicit
limitations. Solo source/evidence review, not independent or adversarial-model
acceptance. Maintainer acceptance and any Phase 8 authorization remain separate.

Baseline: `6608e26b102e108d3d2cdd6cce417b5bca32a044` (merged PR #14), new branch
`feat/context-meter-routing-comparisons` in the existing checkout. No worktree.
[Plan](../plans/phase-7-followups.md), accepted
[ADR 0016](../adr/0016-persistent-routing-provenance.md),
[pinned checks](../evaluations/phase-7-followup-validation-2026-10-16.json).

## Exit evidence

| Ticket | Finding and evidence | Disposition |
| --- | --- | --- |
| P7-05 context usage | Default execution status and idle composer show percentage plus estimated used/configured input allowance; compact plain output does too. Tests cover the requested 57% example, compaction decrease, opaque partial data, missing metadata, reset, footer rendering and restarted idle status. | Met with estimation limits |
| P7-06 routing comparison | Eight live episodes on identical frozen coding and notes tasks, fixed Sol/low vs Sol→Terra/low; two counterbalanced repetitions per cell. Independent unchanged coding tests and artifacts, separate solo factual notes rubric review. Actual routed calls use both models. | Met as small empirical comparison; no benefit established |
| P7-07 resumability | Typed response dispatch origins and durable replay mode in snapshot v2; native same-source continuation retained, mixed/changed/unknown history visible after restart and routing-off. V1 load is read-only; successful save migrates. Original private state retained, no tool replay. | Met under accepted migration contract |

The meter denominator is the configured estimated-context budget minus response
reserve (default 65,536−4,096 = 61,440), not provider capacity. Its numerator is
non-opaque serialized JSON bytes/4. Opaque data makes it partial; even without
opacity this is a heuristic. Actual model-window occupancy is unknown. Idle
`last` describes the most recent prepared selection, not the entire stored
conversation or text being composed. Existing context byte ceilings and whole
exchange admission are unchanged. Detailed inspection remains `--show-context`.

## Live quality comparison

The [frozen manifest](../../tests/fixtures/evaluation/comparison.json) preceded
inference. The [runner](../../scripts/compare_routing.py) creates fresh synthetic
folders, keeps task/fixture hashes, common effort/budgets/permissions, observed
usage, CLI output and trace metadata. [Raw results and grading](../evaluations/phase-7-comparison-2026-10-16.json)
retain each episode separately. The live binary is pinned in that report; runs
occurred after the meter and before provenance implementation. These results do
not establish live acceptance of persisted routing.

| Task | Configuration | Rubric pass | Mean wall s | Mean input tokens | Mean output tokens |
| --- | --- | --- | --- | --- | --- |
| Notes | Fixed Sol/low | 2/2 | 15.19 | 3,345 | 280 |
| Notes | Sol→Terra/low | 2/2 | 21.80 | 3,313 | 281 |
| Coding | Fixed Sol/low | 2/2 | 32.53 | 13,573.5 | 512.5 |
| Coding | Sol→Terra/low | 2/2 | 32.69 | 35,583 | 507 |

All notes answers included the three outstanding actions with citations and
excluded the completed washer task; no notes fixture changes, shell or approvals.
All coding answers repaired whitespace and exact greeting bytes; unchanged tests
passed independently, with Cargo.toml/AGENTS unchanged. Models also invoked the
verification command themselves. Runs completed, but completion alone was not
the grading criterion. Scripted providers were used only for runtime invariants.

No general quality advantage or statistically reliable latency improvement is
established. Routed coding used substantially more observed input tokens here.
The intervention includes visible-history reconstruction as well as the stage
model; it does not isolate model strength. Two samples per cell, one provider,
two narrow tasks, cache/server scheduling and account load limit interpretation.
Subscription cost, true TTFT and decode throughput remain unavailable, not zero.

## Persistence and source review

The maintainer explicitly approved v2 provenance and visible-history migration.
Origin labels describe dispatch configuration, not a server-resolved model or a
proven default effort. Missing origins stay unknown. Same-source native sessions
can keep private continuation; model/provider changes, configured routing or
unverified history force durable visible replay, including later routing-off runs.
Visible projection excludes private reasoning and preserves complete tool
exchanges; the original session content remains intact.

V1 load does not write or invent provenance. Atomic idle publication writes v2;
older binaries reject v2. Validation/serialization/pre-publication failures retain
the old snapshot. As in ADR 0012, a directory-sync failure after rename can leave
the new snapshot published with durability uncertain; there is no false rollback
promise. Existing permissions, private filesystem bounds, exclusive lock, workspace
binding, run metadata and incomplete-tool-batch rejection remain enforced. No
active execution is recovered or replayed on load. Policy files are not persisted.

Solo review findings resolved in this slice:

- The context meter crowded slash-command feedback off the footer. Status and
  notices now use separate existing rows; render and real-PTY regressions pass.
- Inserting `/routing` into the existing slash menu changed navigation positions.
  Appending it preserves the existing selections and regression behavior.
- An intermediate provenance set used an ID type without `Hash`; it now uses the
  existing ordered ID contract. Strict clippy's nested-condition finding was fixed.
- Parsing a JSON value before typed deserialization could erase duplicate struct
  keys. Final typed decoding uses original bytes; duplicate version, missing fields,
  foreign IDs and v1 metadata smuggling reject without overwrite.
- Fixture providers now explicitly declare their known identity/capability rather
  than weakening production unknown-provenance handling.

## Validation and retrospective

Final locked Rust suite: **218 passed, 0 failed, 1 ignored** (the existing optional
recording-overhead experiment). Comparison runner: **2 Python tests passed**.
`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and
`git diff --check` passed. Migration/restart/tool/permission tests use scripted
inference; they establish contracts, not model quality. Local link/hash checks
are recorded with the pinned validation.

What we learned: the useful user-facing context number already existed in typed
admission metadata, but needed explicit units and unknowns. The comparison shows
routing can cost more input without improving these narrow outcomes. Which
assumptions were wrong: mixed private history cannot become resumable merely by
removing a rejection. Useful abstractions: model-call IDs, context ledger provenance,
request projections and atomic idle snapshots. Premature abstractions: automatic
classification, model capacity registries, worker recovery and universal provider
compatibility. Debt that could distort later work: exact token/capacity accounting
and broader quality evidence remain absent. Do not carry forward: treating an
estimated budget as a model window, scripted output as quality, or durable idle
history as background/crash recovery.

No unresolved consequential decision blocks these three tickets. Broader tasks,
more repetitions, verified provider accounting/capacity and cross-device trust
remain future scoped work. No Phase 8, harness adapter, paid infrastructure,
desktop, local inference or scheduler implementation. M02 Paused/unaccepted,
ADR 0014 Superseded, Phases 5/13 Deferred. Unrelated user files and recovery data
are preserved; current changes remain uncommitted on the requested new branch.
