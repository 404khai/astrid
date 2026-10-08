# ADR 0009: Bounded local metadata traces

Date: 2026-10-08
Status: Accepted
Implementation: Implemented
Related: [Phase 4](../plans/phase-4.md), P4-04/P4-05; extends ADR 0002's historical no-persistence scope
Decision evidence: Maintainer instruction on 2026-10-08: "Implement the observability and its related adrs 8 & 9".

## Context

Current public events contain task text, arguments, outputs, file labels, summaries
and workspace reports. Serializing them directly would retain repository content
and potentially tool-observed secrets. A trace must reconstruct where execution
went without conflating persistence with session recovery or allowing disk failure
to relabel committed work.

## Options

1. Bounded metadata-only local traces; incomplete recording is explicit and
   separate from the run outcome.
2. Full public event transcripts; better content reconstruction but significant
   sensitive-content retention and a larger storage/redaction contract.
3. Mandatory durable recording; storage failure stops execution, changing the
   meaning of a successful runtime operation and requiring stronger commit rules.

## Decision

Use option 1. Persist a versioned allowlist projection: IDs, sequence, run-relative
monotonic offsets/durations, wall-clock start, provider/model, validated native tool
names, lifecycle outcomes/categorical failure codes, available usage, numeric
context sizes/budgets/counts and completeness diagnostics. Omit tasks, message
text, arguments, results, shell commands, paths, summary text, arbitrary error
messages, credentials and private continuation. Metadata still reveals execution
patterns and model identity; local private files are not encryption.

Capture before client rendering with a bounded recorder independent of terminal
lifetime. Bound queue, individual records, per-run bytes, total store bytes and
finalization wait. At overload/storage failure stop recording, retain whatever
valid prefix exists and report trace incomplete/unavailable separately from run
outcome. Do not block forever or drop records while claiming completeness.
No automatic deletion of old traces; a full store rejects new recording.

Use versioned JSONL framing with a header and terminal completeness footer. Sequence
gaps, truncated records, missing footer and invalid suffixes make a trace incomplete;
read the valid prefix without pretending the run completed. Unsupported schema
is an explicit error. A full trace records terminal outcomes, including failure
and cancellation. Complete means the metadata recording is complete, not that
task success or transcript fidelity is established.

## Consequences and compatibility

The first persisted schema is execution inspection, not event sourcing, full
ExecutionState replay, session persistence, request reproduction or recovery.
No migration of ephemeral sessions is needed. Future readers dispatch on schema
version; changing stored semantics requires explicit versioning/migration decisions.
Do not reinterpret old records silently. Local path and numeric limit defaults
can be decided in the ticket. Existing event-channel ordering/backpressure is
preserved; optional recorder failure has an independent diagnostic path.

This deliberately limits the roadmap's reconstruction promise to lifecycle and
measurements. Content-bearing trace modes would require a separate accepted
retention/privacy decision. No remote export, database or tracing SDK is needed.

## Accepted decision question

Accept bounded metadata-only local traces whose recording failures are reported
without changing run outcomes, rather than full transcript persistence or mandatory
durable logging?

## Validation

Privacy sentinels in task/tool/summary/error/private continuation never reach disk;
headless/disconnected execution records; writer failure after a committed edit does
not change the tool outcome; blocked writer/queue saturation cannot stall cleanup;
byte ceilings and malformed/crash prefixes are deterministic; incomplete statistics
expose coverage; private permissions and run-ID lookup cannot escape the trace store.

## Resolution

Accepted on 2026-10-08 by the maintainer's explicit implementation instruction.
Implementation and validation are recorded in [the Phase 4 review](../reviews/phase-4.md).
Cost of changing later: High (persisted semantics and
the relationship between storage failure and execution become durable contracts).
