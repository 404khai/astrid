# RC-03 — Aggregate tool-exchange admission: solo review

2026-10-21, `feat/aggregate-tool-admission`, parent `fd419fb6` (RC-02).
Maintainer authorized a new branch for RC-03, publication, merging the open
implementation PRs and closing the paused Tauri draft. A separate conversation
thread cannot be created through this session's tools; the branch and checkpoint
record the requested new scope without claiming a new UI thread was created.

## Failure and smallest repair

Per-tool retained-output caps do not constrain the sum of results in one model
response's multi-tool batch. Previously the runtime executed every requested tool
before checking the next model request's mandatory context. Later mutations could
therefore run after preceding results had already made admission impossible.

Measure the original/current user requests, instructions/tool definitions and
new assistant response with its accumulated actual results before the first tool
dispatch and between terminal tool outcomes. Exclude older optional exchanges,
which normal selection can prune. The first response's full tool-call arguments
are included, so oversized request batches stop before any tool effect.
After the final result, check again before another model call or a ceiling result.

Use adapter accounting with the next deterministic route's effective model,
effort and existing replay mode; at the model-call ceiling use the current route.
Peeking the policy creates no route event, model invocation or authentication.
This is measurement of an incomplete prefix, never a request sent to a provider
or a committed context selection. Actual complete requests still pass normal
selection and exact preflight. Missing accounting fails closed. No inference is
spent measuring, and cancellation wins over an interrupted measurement/error.

When admission fails, keep all original responses/results and committed mutations.
Mark still-pending tools skipped with the existing `run_failed` lifecycle, fail
the turn/run as `context`, and execute no later tool or model request. Do not
truncate results, compact the latest exchange, fabricate terminal tool outcomes
or rerun side effects. Original incomplete-batch continuation rejection remains:
a skipped batch has no invented matching results and requires `/new`. A fully
completed oversized batch remains retained and may still require a fresh session
or an explicitly different user budget. No resumability guarantee is added.

## Bounds and compatibility

This is a guard against further execution after mandatory content exceeds local
admission, not a promise that every completed exchange fits. The result that first
crosses the threshold is retained unchanged; its pre-existing tool-specific cap
still governs that result. Custom executors can have other caps. Model response
assembly, total session retention and total process memory gain no new hard bound.
A prefix that fits does not prove that future results or opaque provider tokens
will fit. The local byte/heuristic limits, response reserve, per-tool caps and
provider-fit unknowns are unchanged.

No new ADR is needed: this is earlier enforcement of ADR 0006's accepted local
admission policy with ADR 0003's original retained results and ADR 0010/0020's
unchanged complete-exchange/incomplete-batch rules. No synthetic tool outcomes,
permission change, persistence format, trace schema, event family or remote grant
is introduced. Unbudgeted library callers retain their existing behavior.
Current request selections are not overwritten by the prefix measurement.
Public diagnostics contain sizes/limits; metadata traces still retain only the
existing categorical `context` failure, not result text or private continuation.

## Acceptance and validation

Fixtures exercise many individually admissible results whose aggregate exceeds
both byte and heuristic limits, escaped Unicode output, oversized tool arguments,
retained completed mutations, a final completed exchange overflow, unchanged
under-budget/unbudgeted runs, cancellation during accounting (including error),
checked event replay and no continuation/replay of the skipped batch. The wire
accounting assertion independently measures one actual result versus their sum.
The same aggregate regression is run against the original runtime to demonstrate
that the later mutation executes there. All data/HTTP/auth are synthetic.

Final commands, source hashes, red/green evidence and limits are in the
[evaluation](../evaluations/aggregate-tool-admission-2026-10-21.json).
No live inference/InstaBox operation, provider credential or private-session read.
GitHub authentication is used only for the explicitly authorized PR operations.
Unrelated local history, user files, recovery data and paused desktop source stay
preserved. Closing a GitHub draft does not delete its historical PR record.

Disposition: passed with explicit limits. **288 Rust passed, 0 failed, 1 ignored;
2 Python passed; fmt and strict all-target Clippy passed.** The ignored case is
the existing optional recording-overhead experiment. Stop after RC-03 and the
authorized publication/merge cleanup. Next: RE-01's concrete target-authority ADR and smallest mocked
InstaBox adapter; no remote implementation or live grant is inferred here.


## Publication and dependency cleanup

Implementation `b87218d4` is [PR #21](https://github.com/404khai/astrid/pull/21).
#19 and #20 merged in order with merge commits, retaining the reviewed source
ancestry. #21 was retargeted to main after those merges. Draft #13 was closed,
with its branch head/source verified unchanged. No unrelated files were staged.
Final #21 merge disposition is the linked GitHub record; all three pinned code
hashes remain unchanged by this publication-only checkpoint.
