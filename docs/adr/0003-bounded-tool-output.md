# ADR 0003: Bounded tool output and cleanup under backpressure

Date: 2026-10-06
Status: Accepted
Implementation: Implemented; validated 2026-10-07
Related: Phase 2; P2-02 through P2-04; ADR 0002 event transport and cancellation
Decision evidence: Maintainer explicitly selected the recommended contract in
conversation on 2026-10-07.

## Context

Shell execution currently drains stdout/stderr into unbounded vectors and returns
them only after completion. Timeout output is explicitly unavailable. Streaming
these pipes into the existing lossless bounded event channel introduces a hazard:
a stalled observer must not prevent timeout/cancellation from stopping a process.
File/search results also need output bounds, independently of Phase 3 context policy.

## Options

1. Preserve blocking delivery throughout tool execution. Simple and consistent
   with current transport, but cleanup can become coupled to consumer progress.
2. Bound retained output and delivery buffers; explicitly discard excess output;
   keep process cleanup independent of delivery. More coordination, but bounded
   resources and honest missing-output reporting.
3. Spool all output to disk for later delivery. Retains more evidence but introduces
   disk quotas, artifact ownership, and persistence ahead of the trace phase.

## Accepted decision

Choose option 2. Extend the tool/runtime seam with structured stdout/stderr chunks
correlated to ToolCallId. The runtime owns public event identity and sequencing;
tools do not acquire a second public event publisher. Preserve byte order within
each stream; do not claim a true total order between stdout and stderr.

Use separate, explicit limits for read chunks, queued output, retained result
bytes, and total output eligible for live delivery per call. Initial proposed
defaults: 8 KiB read chunks, 64 KiB queued bytes per active shell call, and
64 KiB retained/deliverable prefix per stream. Retention and delivery need distinct
counters: consumer pressure can discard a live chunk while its prefix remains
in the final result. Limits count raw bytes, not Unicode characters. These are
starting limits to validate, not performance claims.

Continue draining after a capture limit so the child cannot block on a full pipe.
Drop only output not yet accepted as a public event; account for it in a bounded
summary. Already-assigned event sequences remain lossless and ordered. Once a
live-output queue overflows, stop further live chunks for that call and report
the omitted byte counts in its terminal output summary. No silent gaps.

Cancellation/timeout must trigger process-group cleanup without waiting on an
observer or a delivery queue. Final lifecycle delivery and the run's return may
still wait for an attached consumer to drain or detach, as in ADR 0002. Tests must
prove cleanup independently of receiving the terminal event. A cleanup failure
remains a run failure. Specify ownership and joining of any reader/cleanup tasks;
no background tasks may silently outlive the tool.

Return bounded partial output after timeout/cancellation where captured, with
truncation/omission metadata and truthful exit status or unavailability. Counters
cover observed bytes only: unread output after pipe closure is unavailable, not
an invented byte count. Retain distinct TimedOut and Cancelled lifecycle outcomes.
Do not inject new conversation results for cancelled/skipped tools.

Apply bounded result production to read_file, grep, glob, and list_directory too;
avoid reading an entire large file just to truncate its result. Use a proposed
64 KiB result-content limit with explicit truncation metadata. Specify whether
an oversized indivisible record is omitted or clipped and preserve valid text.
This bounds individual tool output, not total conversation/state memory.

## Consequences

Amends ADR 0002 to define which raw tool bytes enter its lossless event contract.
The contract for accepted lifecycle events remains unchanged. Terminal tool
payloads must retain output metadata even for cancellation. Serialization,
projection, CLI rendering, and headless consumers need coordinated updates.
The CLI should avoid printing streamed bytes again as a final-output duplicate.
No persisted traces, output spool, token budgeting, or compaction is introduced.

## Decision question

Accept bounded output with explicit loss reporting and cleanup independent of
delivery, while terminal notification may still wait for an attached observer?

## Validation

Flood both pipes beyond every limit; stall a receiver; request cancellation;
verify the process group stops before draining the receiver. Then resume delivery
and verify ordered events, accurate omission metadata, and one terminal outcome.
Also cover timeout partial output, split UTF-8, no trailing newline, pipe errors,
receiver disappearance, exit without an exit code, and large file/search results.

## Resolution

Accepted on 2026-10-07. ADR 0002 event transport is amended for raw tool
output admission; accepted lifecycle events retain its lossless ordering contract.

## Implementation evidence

`output.rs` retains raw stream chunks and incremental UTF-8 decoding.
`tools.rs` drains both pipes with fixed local buffers and nonblocking admission;
`runtime.rs` polls public delivery concurrently with execution/cleanup. Cancelled
terminal events retain structured partial output without conversation results.
Output summaries distinguish capture omissions, live omissions, unread completion,
and bytes unavailable as an incomplete text suffix. No-reader execution has final
results even without a public output consumer.

`native.rs` bounds actual JSON results to 64 KiB, selects at most 1,024
lexicographically smallest candidate records with a 32 KiB returned record budget,
and caps grep at 1 MiB/file, 16 MiB aggregate, and 16 KiB/line. Oversized inputs
and depth/name limitations carry incomplete coverage. Inspections run in joined
blocking jobs with cooperative cancellation. Mutation inputs and aggregate
conversation/state memory are outside these individual-output bounds.

Evidence: `tests/environment.rs`, `tests/native_bounds.rs`, `tests/phase2.rs`,
and decoder regressions in `tests/phase2_adversarial.rs`. The stalled-consumer
fixture proves shell cleanup before public delivery resumes.
