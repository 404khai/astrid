# Runtime recovery before remote execution

Date: 2026-10-18
Authorization: Implementation authorized by the current maintainer request;
[ADR 0020](../adr/0020-followup-context-recovery.md) separately accepted in this session.
Implementation: RC-01 Complete; budget presentation Complete; RC-02 live usage projection Complete.
Acceptance: RC-01 Passed with the explicit limitations below.
This scoped milestone does not advance Phase 9 or resume paused M02.

## First implementation slice — RC-01

Repair follow-up context admission at the selection boundary. A short explanation
request after a large completed transcript must dispatch under the unchanged
allowance. Protect the original/current request, latest actual complete exchange
and its historical question; permit the earlier prefix to be omitted/condensed.
Preserve full original history, v2 provenance, original continuation, complete
request/result units, call-ID uniqueness, permissions and cancellation barriers.

The existing unmatched-tool cancellation policy remains binding: inspect the old
session and start `/new`; do not replay tools or manufacture outcomes. A cancelled
model request with no committed incomplete batch may accept a follow-up. This
slice fixes context stranding, not active-job/crash recovery.

Tests: oversized original and follow-up submissions; private reasoning excluded
from summary; source history unchanged; actual mocked dispatch and no repeated
edit; event replay; protected overflow remains a failure; existing cancellation,
permission, persistence and routing suites. No real session or credential reads.

## Independently reviewable visibility follow-up — RC-02

The small CLI presentation change in this slice prints the configured model-call
ceiling, planning response reserve and exact request bytes/ceiling alongside the
existing local utilization estimate. These are configuration/admission values,
not an enforced provider output-token limit or known model-window occupancy.

RC-02 implementation now exposes provider-reported input/output/cached counts in
the normal run projection, with explicit unavailable/invalid states and per-metric
reporting-call coverage. `observability::Usage` validates counts; traces/stats
inspect retained/recorded coverage via `astrid trace <run-id>` and `astrid stats`.
`ExecutionEvent::ModelUsageReported` supplies bounded headless metadata to the
console without changing `ModelResponse` or private session v2. The existing
observability opt-in remains binding.
[Compatibility and review](../reviews/live-token-usage-2026-10-18.md).
Test absent, zero, malformed, partial and cancelled-call telemetry; cached is a
subset of input. Never derive account quota percentage or cost without a basis.

## Validation record

Baseline inspected: `8dea6765`, existing `feat/tui-redesign` checkout. Unrelated
`.gitignore`, earlier CURRENT entries, user assets and recovery files preserved.
Initial serial `cargo test --locked --offline -- --test-threads=1` passed before
the repair. Final validation on the pinned source:

- `cargo test --locked --offline -- --test-threads=1`: **279 passed, 0 failed, 1 ignored**.
- `cargo fmt --check`: passed.
- `cargo clippy --locked --offline --all-targets -- -D warnings`: passed.
- `python3 -m unittest test_compare_routing.py` from `scripts/`: **2 passed**.
- Scoped whitespace and local Markdown link checks passed.

The ignored case is the existing optional recording-overhead experiment.
[Source hashes and command record](../evaluations/runtime-recovery-2026-10-18.json),
[solo review and limits](../reviews/runtime-recovery-2026-10-18.md). No live inference, cloud action, provisioning, port exposure or secret
transfer is part of acceptance.

[Dependency-ordered direction](../plans/remote-mobile-cli.md).


## RC-02 validation — 2026-10-18

Separate branch `feat/live-token-usage`, parent recovery commit `dc7cbfbc` / PR #19.
**283 Rust passed, 0 failed, 1 ignored; 2 Python passed; fmt and strict all-target
Clippy passed.** The ignored case remains the optional recording-overhead test.
[Commands/source hashes](../evaluations/live-token-usage-2026-10-18.json),
[compatibility and solo review](../reviews/live-token-usage-2026-10-18.md).
No live inference/InstaBox acceptance or installed binary refresh. Usage is opt-in;
missing or invalid values remain unavailable. RE-01 still needs target authority.

Publication: implementation `36ccc11d`, [PR #20](https://github.com/404khai/astrid/pull/20)
stacked on recovery PR #19. Neither PR was merged by this task.


## RC-03 — Aggregate exchange admission, 2026-10-21

Implementation authorized by the maintainer's new branch/thread request;
`feat/aggregate-tool-admission`, parent `fd419fb6`, same preserved checkout.
Implemented and validated. Check the mandatory exchange prefix before more tool execution and
after the final result; stop pending work on overflow without rewriting results,
replaying effects or changing budgets. Incomplete batches remain inspection-only;
a result that crosses the allowance is retained, so this is not a guaranteed-fit
or total-memory bound. [Review/contract](../reviews/aggregate-tool-admission-2026-10-21.md).
**288 Rust passed, 0 failed, 1 ignored; 2 Python passed; fmt/strict all-target
Clippy passed.** [Pinned evidence](../evaluations/aggregate-tool-admission-2026-10-21.json).
The original-runtime regression executes the forbidden later mutation. RE-01 follows
this slice and still needs its target-authority contract.


RC-03 publication: `b87218d4`, [PR #21](https://github.com/404khai/astrid/pull/21)
targeting main after #19/#20 merged. GitHub PR state records its final merge.
Tauri draft #13 was closed at the maintainer's request, preserving source/branch.
Next: RE-01 authority contract, then mocked adapter; no live remote grant yet.

## RE-01 follow-up — 2026-10-21

RC-01/02/03 are merged (#19/#20/#21). The maintainer authorized the next
foreground mocked remote-tool slice on `feat/instabox-tool-adapter` from main
`cc0fa964`. [ADR 0021](../adr/0021-remote-target-authority.md) records the concrete
exact-target authority contract, explicitly Accepted by the maintainer with atomic
consumption, local OS scope and future auditability clarifications. Independent
HTTP extraction retains the existing standalone CLI, key-loading tests and bounded
executor tests. The host-installed foreground typed tool now follows that contract;
[implementation review](../reviews/instabox-tool-adapter-2026-10-21.md).
No live trial is authorized.

Extraction validation: **288 Rust passed, 0 failed, 1 ignored; 2 Python passed**;
fmt, strict all-target Clippy and whitespace passed. All existing 15 synthetic
HTTP/key tests plus the standalone CLI preflight remain green. This is extraction
evidence only, not remote-tool acceptance.

RE-01 final mocked validation: **302 Rust passed, 0 failed, 1 ignored; 2 Python
passed**; fmt, strict all-target Clippy and whitespace passed. The initial parallel
run had one existing Git-observation assertion failure; isolated and complete
serialized reruns passed without changing the Git observer. Explicitly disable
reqwest protocol-NACK retries as well as application retries.
[Evidence](../evaluations/instabox-tool-adapter-2026-10-21.json).
Stop at this slice and its unmerged PR; propose RE-02 only after separate exact
live authorization. Normal CLI sessions do not implicitly install a remote broker.

Published implementation `5ec7fdd6` in [PR #22](https://github.com/404khai/astrid/pull/22)
against main; left unmerged as instructed. No successor implementation started.

## RE-02 foreground connection — 2026-10-21

Latest maintainer instruction separately authorizes CLI connection, mocks and a
reviewable live plan on existing `feat/instabox-cli-approvals` from `2f8f9358`,
superseding only RE-01's stop boundary. Explicit invocation-only target selection
now installs a separate exact-call broker in one-shot and interactive foreground
runs. No preset/resume/worker authority, implicit target discovery, persistent
permission or schema change. Remote review/attribution and cancellation uncertainty
are visible in the CLI; metadata traces retain ADR 0009's omission contract.

[RE-02 review and bounded trial plan](../reviews/instabox-cli-approvals-2026-10-21.md)
and [deterministic evidence](../evaluations/instabox-cli-approvals-2026-10-21.json).
Live trial remains pending: no exact target/provider/action scope was supplied or
approved. No authenticated metadata, cloud, inference or VM operations performed.
Read-only recheck found #22 already merged upstream at `56880270`; publish RE-02
against main after diff verification, without merging any PR. Stop at this slice.

**308 Rust passed, 0 failed, 1 ignored; 2 Python passed; formatting, strict all-target Clippy and whitespace passed.** No live acceptance claim follows from these mocked checks.
