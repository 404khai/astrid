# Phase 4: trace and inference observability

Date: 2026-10-08
Status: Implemented; deterministic validation and solo audit complete
Planning baseline: `8d765c2` plus the then-shared worktree
Implementation baseline: `010a36d` (`feat/interactive-sessions-modes`), isolated
worktree `/Users/admin/Developer/astrid-worktrees/feat-observability` on `feat/observability`.
Active phase: Phase 4; explicitly authorized by the maintainer on 2026-10-08
Maintainer request: "use our protocol on phase4" and consider a settings command
to turn observability on or off. Subsequent authorization:
"Implement the observability and its related adrs 8 & 9" accepts both proposed
contracts and authorizes implementation. Separate Phase 2/3 closure remains pending.

## Previous-phase review

The [Phase 3 review](../reviews/phase-3.md) evidences all context-engine exit
criteria and recommends closure with explicit limitations. Its separate
maintainer closure decision remains pending, as does the earlier Phase 2
disposition. Recommend closing Phase 3 with those recorded limits; do not infer
closure or Phase 4 implementation authorization from a request for planning.

Planning-baseline inspection confirmed typed run/turn/model/tool IDs, checked serializable events,
independent event replay, context provenance/selection, and adapter-owned exact
request-byte accounting. `src/events.rs` has no timestamps. `src/model.rs` has
no public usage field. `src/openai.rs` discards completion usage when building
ModelResponse. The runtime's first-text marker measures visible text, not the
first generated token. Existing bounded event delivery can stall publication.
Measurements taken by a downstream CLI consumer would therefore include rendering
delay and cannot establish provider latency.

Preserve Phase 3's distinctions: estimated admission is not actual token usage;
summary lineage is not complete retention; normal completion is not task success.
Retained history and model assembly remain unbounded. Phase 4 must bound its own
recording without claiming to solve that existing memory debt.

Fresh baseline validation: `cargo test --locked` passed on 2026-10-08 in the
existing worktree. This checks current invariants, not unimplemented Phase 4.
The sessions/modes/UI prerequisite is inherited from `010a36d`; it is not repeated
in the observability diff. Existing dependencies and unrelated fixture content are unchanged. This planning review is a solo review, not an independent adversarial
model review. No live provider acceptance was performed. Implementation research and final evidence
are in [the Phase 4 review](../reviews/phase-4.md).

## Implemented outcome and settings

With observability enabled, a headless run captures honest model/tool/context
measurements and writes a bounded local execution trace. The CLI can inspect runs
and aggregate known facts without needing the original terminal. With it disabled,
the existing execution, context budgeting, permission and cancellation contracts
continue; optional telemetry and new trace files do not.

Implemented interface:

```text
astrid settings
astrid settings set observability on
astrid settings set observability off
astrid run --observability on "task" --model <model>
astrid trace
astrid trace <run-id>
astrid stats
```

Accepted default off because persistence is new and current events contain task,
source and output content. Enabling uses local metadata-only recording. The
settings command shows stored default and location; run resolution shows effective
value and source. Precedence: explicit run flag, user setting, built-in off.
Snapshot the resolved choice once per run. A settings edit affects future runs;
off neither deletes old traces nor disables mandatory runtime events. Keep the
initial settings scope to this one setting, without a dashboard or generic
configuration framework. Runtime callers pass explicit options and do not load
CLI settings implicitly.

## Grilling and decision table

| Question / concrete failure | Recommendation | Disposition |
| --- | --- | --- |
| Does off suppress permission events or context enforcement? | Only optional telemetry/recording turns off; core events remain | [ADR 0008](../adr/0008-observability-control.md), Accepted |
| A default-on trace writes file contents without the user realizing | Default off; on stores allowlisted metadata, never raw event serialization | ADR 0008 and [ADR 0009](../adr/0009-persistent-trace-contract.md), Accepted |
| Settings change during a run or malformed settings silently enable recording | Snapshot at submission; explicit invalid configuration error; atomic settings update | ADR 0008; local file implementation in P4-01 |
| A stalled consumer inflates apparent inference time | Capture at operation boundaries before publication; separately label inclusive runtime time and available adapter phases | P4-02; do not infer pure inference time |
| Tool-only response has no visible text; cancelled call has no usage | First-text unavailable; do not call it TTFT; retain elapsed attempt/outcome, usage unavailable | P4-02/P4-03 |
| Subscription model has tokens but no per-call monetary price | Cost unavailable unless billing basis and versioned rate are known; no invented API-price cost | P4-03/P4-05 |
| Disk fills after an edit commits, or recorder stalls during cancellation | Bounded recorder; incomplete/unavailable trace diagnostic independent of run outcome | ADR 0009, Accepted |
| Process dies mid-record or unsupported format is loaded | Versioned records; valid prefix labeled incomplete, unsupported schema rejected | ADR 0009, Accepted |
| Thousands of runs exhaust storage | Per-run/total limits; stop recording at limits, no automatic deletion | ADR 0009; numeric defaults are local implementation choices |
| Raw events include secrets copied by tools or summaries | Dedicated allowlist projection; no task/text/arguments/results/paths/summary/private continuation in initial persisted traces | ADR 0009; content replay explicitly excluded |

Both architectural decisions were accepted by the maintainer's implementation instruction. No universal
provider abstraction, new backend, pricing service, session resumption, event
sourcing, tracing framework dependency or new crate is necessary.

## Tickets

Owner: Codex root agent. P4-01 through P4-06 are Done with evidence in
[the review](../reviews/phase-4.md). P4-00's authorization and ADR acceptance are Done;
its separate Phase 2/3 closure dispositions remain pending and do not undo explicit
Phase 4 authorization. No independent agent review is claimed.
Every implementation ticket requires deterministic failure
fixtures, documentation, and the standard locked test/fmt/clippy checks from
[the development protocol](../development.md).

### P4-00: Resolve closure, contracts and authorization

Dependencies: maintainer disposition, ADRs 0008/0009.
Acceptance: separately record Phase 3 closure and Phase 4 authorization; record
accepted/rejected decisions with evidence; update active phase only on explicit
authorization. Earlier Phase 2 closure remains a named independent disposition.

### P4-01: Resolve observability through settings and explicit runtime options

Dependencies: P4-00, ADR 0008.
Scope: `settings` show/set, atomic user-local config, on/off run override, fixed
per-run runtime options, effective-source reporting. No repository-controlled
setting or dependency beyond existing serialization/file primitives.
Acceptance: absent config is off; precedence is deterministic; invalid values
fail before dispatch; settings use owner-only permissions; changing a default
does not change a submitted run; off leaves permissions/context/output intact.
Validation: isolated temporary config, override/invalid/concurrent-update tests,
headless off-mode invariants and CLI help. Atomic replacement prevents torn files;
concurrent writes may use last completed replacement, without partial merging.

### P4-02: Capture runtime and adapter timing honestly

Dependencies: P4-01.
Scope: monotonic run-relative timing and operation durations; call attempt,
provider dispatch/first-visible-text/completion where available; tool execution
separate from permission waiting; context preparation and observable delivery
waiting. Wall-clock start is display metadata, never ordering authority.
Acceptance: deterministic clock fixtures distinguish preparation, auth, transport,
text, tool and delivery phases where actually measured; no-text/denied/skipped
operations have unavailable/nonapplicable timings; failure/cancellation retains
observed attempts; overlapping/inclusive timings are not blindly summed.
No pure prefill/decode or token-rate claim from text chunks.

### P4-03: Preserve provider-reported usage without changing completion semantics

Dependencies: P4-02; concrete upstream research.
Scope: inspect current documentation and applicable Codex transport evidence;
record documentation date/backend differences, then carry available input/output/
cached-token counts through optional provider telemetry. Keep private continuation
internal. Unknown fields remain unavailable and zero remains a real reported zero.
Acceptance: complete, absent, partial, invalid and inconsistent usage fixtures;
telemetry cannot authorize tools from unsuccessful response envelopes; cancellation
does not manufacture final usage. Protocol invariant violations remain failures,
but malformed optional telemetry becomes unavailable with a diagnostic.
Stop research when supported fields and unavailable capabilities are documented;
public API documentation alone does not establish subscription-backend behavior.

### P4-04: Record bounded versioned metadata traces independently of presentation

Dependencies: P4-02, P4-03, ADR 0009.
Scope: recording wired before client presentation, allowlist projection, correlated
IDs/sequences/timings/usage/context numeric totals/outcomes, schema header and
completion/completeness footer. No full transcript or resumable session.
Acceptance: successful headless/disconnected runs persist; cancellation and writer
failure preserve execution outcomes; privacy sentinels absent; queue/record/file/
total limits bounded; crash prefix reads as incomplete; incompatible schema errors;
private file permissions and no overwrite/path traversal through run-id lookup.
Use temp storage and injected slow/failing writers; explicitly measure recorder
overhead before claiming it negligible. A permanently blocked recorder cannot
hold run finalization indefinitely.

### P4-05: Inspect traces and aggregate honest statistics

Dependencies: P4-04.
Scope: list traces, inspect one run, aggregate retained supported traces. Show
provider/model, outcomes, known counts, durations, context sizes/limits and missing
values. Cost/rates only when inputs and measurement boundaries support them.
Acceptance: mixed complete/incomplete/off/missing-usage fixtures do not treat
unknown as zero; totals disclose coverage and distinguish cached subsets; off-mode
runs absent from stored stats are not counted as zero-cost runs; malformed traces
are reported; no task-success claim from RunCompleted. No rate guessing from
visible character counts, local throughput/memory or hidden reasoning contents.

### P4-06: Audit Phase 4 and record lessons

Dependencies: P4-01 through P4-05.
Acceptance: deterministic multi-call/tool scenario with context selection and
usage, on/off settings, headless trace inspection, failure/cancellation/disk-limit
scenarios; baseline-versus-enabled recording overhead experiment with environment
and repetitions recorded. Review exit criteria against actual evidence, document
metadata-only deviations and retained memory debt. Live telemetry evidence is
separate from mocks. No automatic Phase 5 transition.

## Grilling resolution

The maintainer accepted both contracts and explicitly requested implementation.
AGENTS.md now records Phase 4. Closure of Phase 3 remains a separately named
pending disposition. Phase 4 implementation does not authorize Phase 5.
