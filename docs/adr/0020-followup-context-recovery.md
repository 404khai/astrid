# ADR 0020 — Protect the latest exchange without pinning its whole submission

Date: 2026-10-18
Status: Accepted — maintainer selected “Approve the scoped context-selection amendment” in this session.
Implementation: Implemented; 279 Rust tests passed, fmt/strict Clippy passed.
Source-pinned validation and limitations are recorded in the linked milestone.
Amends only the historical-group/latest-exchange interaction in
[0006](0006-context-admission-and-selection.md) and [0010](0010-interactive-sessions.md).

## Context

A short follow-up can fail before authentication because historical grouping turns
“protect the latest complete exchange” into “protect the previous submission's
entire transcript.” The previous run can fit successive requests yet strand its
next question. Increasing allowances retains the cause. The
[regression evidence](../evaluations/runtime-recovery-2026-10-18.json) records
the reproduction against the original selector; the earlier local incident
report remains preserved separately.

## Decision

Protect operating/repository instructions, original task, current request and the
actual latest complete assistant response with every corresponding tool outcome.
When that exchange belongs to a historical follow-up, also retain its question as
an anchor. Separate that historical submission's earlier complete exchanges into
an optional prefix, eligible for existing bounded deterministic compaction. Other
historical submissions remain atomic. Within the current submission, existing
whole-exchange selection remains unchanged. A question or latest exchange that
alone exceeds admission still fails explicitly before inference.

Never split a multi-tool exchange, fabricate tool results or replay effects.
Invalid/incomplete histories still reject continuation unchanged. Cancellation
before selection commit preserves the prior selection. Original messages, private
continuation, global call IDs, provenance, session v2 schema and permission policy
remain unchanged. Omission decisions and explicitly incomplete summary source IDs
identify what was excluded. Private reasoning never enters a summary.

Apply the same rule to execution-event validation so live and reconstructed state
agree. Existing valid traces with whole-submission retention remain valid; this
adds an allowed selection shape without changing event or stored-session fields.
No rewriting of historical snapshots or metadata traces occurs.

## Alternatives considered

- Raise budgets: postpones the failure and resends expensive history.
- Drop the latest historical exchange entirely: weakens continuity unnecessarily.
- Permit arbitrary historical exchange selection: larger semantic change, with
  more opportunities to detach an answer from its question.
- Manufacture completion for cancelled tool batches: violates the completion
  barrier and may conceal uncertain side effects; rejected.

## Consequences and compatibility

The latest historical submission may now be represented by its question, latest
exchange and an incomplete summary of earlier work. Relevant earlier detail can
be lost; original history remains retained. This is safe follow-up selection,
not mid-run crash recovery, remote process termination or automatic resume.
No model capacity guarantee, token-budget increase or new authority follows.

## Validation

Synthetic oversized transcripts must fail under the old selector and fit under
this selector with the same allowance. Cover original and later submissions,
private sentinels, summary lineage, unchanged source history, complete multi-tool
units, actual dispatch and event replay. Existing tests must continue proving
incomplete-batch denial before dispatch, no replay of committed mutation,
selection cancellation atomicity and v2/provenance preservation.

[Milestone and evidence](../milestones/runtime-recovery.md).
