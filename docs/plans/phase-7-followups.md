# Phase 7 follow-ups — context, comparison, resumable routing

Authorized 2026-10-16: maintainer requested all three follow-ups, context meter
first, on a new branch. Branch `feat/context-meter-routing-comparisons` created
from fetched `origin/main` at `6608e26b` after PR #14 merged. No worktree; local
untracked assets and recovery data retained. No Phase 8 authorization.

| Ticket | Contract | State |
| --- | --- | --- |
| P7-05 | Default visible percentage/used/input-budget estimate in execution and idle prompt; partial opaque counts and unknown model window explicit; plain/rich clients | Done; full deterministic checks and solo review passed |
| P7-06 | Frozen same coding and notes tasks for routed vs fixed inference; controlled effort, fixture hashes, artifact grading, observed metrics; no scripted quality claims | Done; 8 live episodes graded separately |
| P7-07 | Accepted v2 response dispatch provenance and durable replay mode; v1 unknown provenance follows visible-history migration; atomic saves, no tool replay | Done; accepted ADR 0016, restart/migration invariants passed |

The maintainer explicitly approved the proposed v2 migration in this thread:
per-response provider/model/effort describe dispatch configuration, not guessed
server resolution. Same-model native continuation is retained when provenance is
known; routed/model-changing follow-ups permanently use visible history. Legacy
v1 history has unknown provenance, retains private bytes, and follows visible
history on its next submission. Successful atomic publication migrates to v2 (post-publication sync errors retain
the existing durability-uncertain contract); old binaries
reject v2. [ADR 0016](../adr/0016-persistent-routing-provenance.md) records this accepted change
to ADRs 0012/0015.

Context denominator is the configured input admission allowance (total local
estimate budget minus response reserve), never an inferred provider capacity.
The estimate counts non-opaque JSON bytes/4, not total tokens. Existing exact byte
ceiling, permissions and private state remain authoritative. Idle values describe
the last prepared selection, not unsent text or a growing session total.

Required validation: meaningful meter rendering/state tests, deterministic comparison
runner failures and task rubrics, restart/provenance validation/failed-save invariants,
full locked tests, fmt, strict all-target clippy and solo review. Keep live evidence
separate; use existing auth and synthetic fixtures without new paid infrastructure.

## Completion and review — 2026-10-16

[Follow-up review](../reviews/phase-7-followups.md) records the exit matrix,
retrospective and limitations. Locked Rust tests: **218 passed, 0 failed, 1 ignored**;
Python comparison invariants: **2 passed**; fmt, strict all-target clippy and diff
whitespace passed. [Pinned validation](../evaluations/phase-7-followup-validation-2026-10-16.json).

[Live comparison](../evaluations/phase-7-comparison-2026-10-16.json) uses the same
frozen coding/notes task bytes and fixtures in all configurations; eight task
rubrics pass. All routed episodes dispatched both configured models. Two samples
per cell establish no general quality advantage; routed coding used substantially
more observed input tokens here. No invented costs or provider capacity.

All three authorized tickets are complete. No Phase 8 advancement; no new
infrastructure, harness delegation, desktop or worker implementation. Unrelated
untracked artifacts and recovery data remain preserved.

Publication — 2026-10-16: [PR #15](https://github.com/404khai/astrid/pull/15) opened
against `main`; implementation commit `cdc6e95c`, 33 scoped files. Publication
source hashes and 339 committed local Markdown targets verified; no new runtime
change or inference. Await maintainer review/acceptance; no Phase 8 implementation.
