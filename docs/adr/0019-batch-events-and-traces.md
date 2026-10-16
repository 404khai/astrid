# ADR 0019 — Batch lifecycle delivery and metadata trace correlation

Date: 2026-10-16
Status: Accepted — 2026-10-16
Implementation: Implemented in the authorized thread; [review and evidence](../reviews/phase-8.md)
Decision evidence: Maintainer approved ADRs 0017–0019 and instructed “I approve begin implementation, new thread, new branch.”
Related: ADRs 0002, 0008, 0009;
[Phase 8 plan](../plans/phase-8.md), [ownership proposal](0017-bounded-task-dag.md)

## Context

A worker's core ExecutionEvent has a per-run sequence and may contain task text,
tool arguments and results. Existing bounded run channels deliberately apply
backpressure. Forwarding several such channels into a stalled UI channel can hold
all workers open during cancellation. Adding BatchId fields to persisted run records
would also change an accepted schema. Batch lifecycle observability needs an explicit
contract without silently changing existing run consumers or recording content.

## Options

1. Forward all child events through one lossless backpressured channel. Familiar,
   but upstream stalls can obstruct joins unless a larger delivery protocol is built.
2. Keep child run streams unchanged and drain them internally; expose a distinct
   bounded batch lifecycle projection with explicit detach on saturation, a complete
   bounded lifecycle log in BatchResult and optional metadata-only batch traces.
3. Persist full child transcripts and task artifacts for replay/recovery. Larger
   privacy, durability and schema scope; not needed for Phase 8.

## Accepted decision

The maintainer selected option 2, with these semantics explicit in API documentation and tests:

- Introduce BatchEvent/BatchState, not a synthetic parent ExecutionEvent. A single
  coordinator assigns batch sequence numbers at validated state transitions. Events
  identify BatchId, applicable TaskId/WorkerId and child RunId/SessionId once known.
  Child run sequence numbers remain local; timestamps do not establish global order.
- Public lifecycle events describe admission, readiness, assignment, run linking,
  cancellation requests, result publication, skipped work and terminal outcomes.
  Do not forward raw ModelTextDelta, tool arguments, ToolOutput or private state into
  this stream. Child runs keep their original checked events and traces unchanged.
- Internal collectors drain each bounded child channel concurrently until closure;
  they never await terminal rendering or batch subscriber delivery. Bound internal
  messages/results by graph and worker ceilings. A renderer cannot own cancellation
  propagation, child collection or worker-resource cleanup.
- The new batch live projection uses nonblocking bounded delivery. On full/closed
  receiver, detach the projection and report delivery incomplete/disconnected in
  BatchResult. Do not drop individual events and claim that the subscriber received
  a complete history. Closing a subscriber is not cancellation. This is a new API
  policy, not a change to existing run-channel backpressure guarantees.
- Retain the full bounded batch lifecycle log in BatchResult, independent of live
  delivery and optional recording. Bound its event count from validated task/edge
  ceilings and the enumerated state machine; repeated cancel requests are idempotent
  and do not generate unbounded events. No token/tool-output events in that log.
  Invalid transitions fail explicitly rather than growing past the bound or silently
  omitting lifecycle state. BatchState replay of that log must match returned state.
- Worker context progress, if displayed, is a separately bounded/coalesced numeric
  projection tagged with TaskId. It is not a full child transcript or global token
  occupancy. Reuse estimated/partial/unknown meter labels per worker; never sum
  context percentages across models. Full per-call metadata remains in child traces
  when recording succeeds. Distinguish progress delivery coverage from lifecycle
  recording completeness.
- When observability is enabled, add a distinct batch trace family in private user
  storage (proposed `traces/batches/<BatchId>.jsonl`), with an explicit batch-kind
  header/schema and terminal completeness footer. Do not modify existing schema-1
  run records or v2 idle sessions. Existing run readers retain their behavior;
  batch inspection has a separate reader/CLI path.
- Persist only a new allowlist: generated IDs, dependency edges between generated
  task IDs, effective capability actions, monotonic offsets, lifecycle/categorical
  outcomes, numeric counts/budgets/result lengths, and child trace references/coverage.
  Omit manifest paths, caller task labels, prompts, transferred/final text, content
  digests, file paths, tool arguments/outputs, arbitrary error text, credentials and
  provider continuation. Dependency topology is observable metadata and may itself
  reveal execution patterns; private files are not encryption.
- Link children even when their traces are disabled/incomplete/unavailable; never
  invent a complete linked execution. Batch trace completeness describes its own
  lifecycle recording, not child coverage or task correctness. Report unknown usage
  and cost with coverage counts; observed parallel wall time is not the sum of child
  durations. Do not count both child and aggregate usage in statistics.
- Reuse the accepted recorder's bounded queue/record/run/store/finalization principles.
  Both trace families must participate in the configured total byte/file quota and
  writer exclusion, including concurrent batch/child writers. Do not quietly grant
  another full store allowance per directory. No automatic deletion. Writer failure
  or saturation reports incomplete recording separately from execution outcome.

## Consequences

A stopped live consumer can lose the new batch projection, with explicit coverage,
while the returned bounded lifecycle log remains reconstructable. This trades live
lossless delivery for independent cancellation/joins. Callers requiring every batch
transition can inspect BatchResult; persistent recording remains optional and fallible.

This adds a new inspectable metadata format requiring acceptance, but no migration
of old traces and no active-task recovery. Existing single-run APIs, lossless attached
channels and snapshot v2 remain unchanged. Raw full child history is not implicitly
available through batch events; no new full transcript/content retention is authorized.

## Decision question

Approve a distinct batch lifecycle projection that explicitly detaches on saturation,
retains its bounded log in BatchResult, and records optional metadata-only correlation
in a separate trace family under shared store quotas?

## Validation

Stall and close external receivers during inference and cancellation: workers still
join, live delivery becomes explicitly incomplete, and returned lifecycle replay
matches state. Assert contiguous batch sequences, correct child links and no extra
terminal events. Use privacy sentinels in task labels, prompts, dependency text,
errors, paths and private continuation; none reach persisted metadata. Test old run
trace readers unchanged, malformed/new batch schemas, partial prefixes, concurrent
quota admission, writer saturation/failure, child trace coverage and no double-counted
usage. No new paid infrastructure or network-dependent critical tests.

## Resolution

Accepted on 2026-10-16 in response to the explicit decision question for ADRs
0017–0019. The maintainer also authorized implementation in a new thread/branch.
The selected recommendation above is binding for this implementation; prior
planning-only scope is superseded. Phase 7 administrative closure is not invented.
Implementation and actual validation must be recorded slice by slice; no Phase 9.

## Implementation note — 2026-10-16

Uses flat private `traces/batch-<BatchId>.jsonl` / `.partial` names rather than the
proposed subdirectory. This keeps both families under the identical existing store
lock and total byte/file quota without recursive accounting or a second allowance.
The family/header/reader remain distinct. Recording is fallible: the one bounded
live sample had complete batch metadata, two unavailable child traces and one
complete child trace; coverage is retained honestly, not inferred from parent
completeness. See the [review](../reviews/phase-8.md).
