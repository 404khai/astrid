# Live provider usage — RC-02 compatibility and solo review

2026-10-18. Parent: recovery commit `dc7cbfbc`,
[PR #19](https://github.com/404khai/astrid/pull/19). Separate stacked branch
`feat/live-token-usage`; implementation `36ccc11d` published as
[PR #20](https://github.com/404khai/astrid/pull/20), base PR #19. No merge or
Phase 9 transition.

## Observable behavior

With observability enabled, each model call can publish one typed
`ModelUsageReported` event carrying validated optional input/output/cached counts
and an invalid flag. Runtime state and checked event replay retain it. The normal
CLI prints the report and known run totals with separate reporting-call coverage
for each metric. Missing and invalid counts stay unavailable; zero remains zero.
Cached is a subset of input, never added to it. Totals are cumulative request
usage, not context-window utilization, monetary cost or subscription quota.

On model completion/failure/cancellation without a report, display unavailable
usage. A cancelled or failed call can retain metadata already reported by the
provider; numeric usage never implies a committed response or successful task.
Duplicate reports cannot inflate live or trace totals. Invalid typed metadata
from custom providers is normalized before projection/recording, and replay
rejects noncanonical or duplicate reports atomically. Wide integer totals avoid
wrapping two valid u64 counts. New runs reset presentation counters.

## Compatibility review before implementation

- ADR 0008's off setting still disables optional telemetry collection and trace
  writes. This slice does not silently turn collection on or change setting
  precedence. Live counts use the existing opt-in; off renders unavailable.
- ADR 0009's schema-1 `Record::Event.kind` is already an open string and the
  existing reader validates framing/sequence/terminal state rather than a closed
  kind allowlist. `model_usage_reported` adds numeric allowlisted metadata, not a
  new record variant, reinterpretation of an old field or schema migration.
  Existing `Record::Telemetry.usage` remains the source for trace/stat counts;
  the new event is not counted a second time. Old generic schema-1 readers can
  retain this event without needing to understand its name. Existing traces
  remain unchanged. The actual schema-1 reader/summary tests verify completeness
  and identical per-call usage/counts with the new event in the sequence.
- The Rust event enum and transient `ModelState` gain an additive metadata variant
  and defaulted optional field. Exhaustive Rust consumers need recompilation and
  handling of the variant; old typed ExecutionEvent decoders cannot replay the
  new enum variant. There is no persisted ExecutionEvent log or network Host
  contract being migrated. Session v2, provenance, private continuation and idle
  snapshots are unchanged. Metadata traces do not serialize ModelState.
- The existing bounded event channel and Execution's pending-event handling apply
  to usage too. Reporting consumes no inference, tool calls or new permission.
  The metadata remains available headlessly and after client disconnect; the CLI
  does not read traces to render it. Recorder overload/failure stays independent
  of execution; enabling telemetry creates the recorder handle even when its
  underlying recording is unavailable, so live metadata is not gated on a
  successful disk write.

This is the compatibility review required by RC-02, within the requested live
projection work. It introduces no remote trust, persistence-format, credential or
permission contract. No additional ADR acceptance is inferred or required.

## Validation and limits

Deterministic cases cover full/zero/partial/absent/malformed usage, observability
off, atomic invalid/duplicate event rejection, checked replay, first-report
normalization, cancelled/failed calls, trace equality and no double counting.
CLI tests cover numeric labels, unavailable fields, per-metric coverage, reset,
cached subset semantics and totals exceeding u64. Existing privacy, recorder
coverage, routing, cancellation, persistence, PTY and scheduler checks remain
required. Final results and pinned source hashes are recorded in the
[evaluation](../evaluations/live-token-usage-2026-10-18.json).

Solo review: no new dependency, budget default, trace schema, private-session
field, permission, tool execution or remote action. Live metadata is opt-in;
missing provider telemetry and interrupted streams can leave counts unknown.
No real provider/VM acceptance, provider/InstaBox credential or private-session
content reads, or installed CLI refresh. Existing GitHub CLI authentication is
used only for the authorized publication. Unrelated local files and prior checkpoint history remain preserved.

Disposition: RC-02 passed with explicit limits. **283 Rust passed, 0 failed,
1 ignored; 2 Python passed; fmt and strict all-target Clippy passed.** Stop after
this slice and its separate commit/PR.
RE-01 still requires its concrete target-authority contract before code.
