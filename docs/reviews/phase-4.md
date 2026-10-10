# Phase 4 review: optional trace and inference observability

Current audit: 2026-10-10, `cdebe18` plus closure worktree changes
Status: Administratively closed with documented limitations; solo current audit
Authorization: Maintainer requested "Implement the observability and its related adrs 8 & 9".
Maintainer closure decision: Approved on 2026-10-10, conditional on verified exit
criteria and accurate limitations; the current audit below supplies that evidence
Next-phase authorization: None; AGENTS.md remains Phase 4
Separate previous-phase closure dispositions: Phases 2 and 3 approved 2026-10-10
Historical implementation baseline: 2026-10-08, `010a36d`
(`feat/interactive-sessions-modes`); isolated `feat/observability` worktree

## Current exit-criteria audit

The exit criterion is completed-run inspection showing where context, tokens,
time, tools and model calls were spent, with provider availability limits.

| Requirement | Current code and rerun evidence | Assessment / limit |
| --- | --- | --- |
| Prompt/completion/cached tokens and provider/model | [openai.rs](../../src/openai.rs), [observability.rs](../../src/observability.rs); actual-adapter local HTTP and absent/zero/partial/invalid usage tests | Met where supplied; usage never inferred and cached tokens are an input subset; no live subscription telemetry verification |
| TTFT, model latency, generation rate | Adapter preparation/auth/dispatch/first-visible-text/attempt and runtime offsets; fixed-timeline, no-text, cancellation and stalled-consumer tests | Honest measured latency available; true TTFT and generation/decode rate unavailable, not synthesized from chunks |
| Tool execution time | Runtime measures actual executor future; fixed permission/execution and multicall fixtures | Met; permission and delivery waits separated, inclusive intervals can overlap |
| Context utilization | ContextSelected/Prepared allowlist snapshots, budgets/counts/summary bytes; multicall fixture reads recorded summaries | Met for local request admission/selection; no exact provider-capacity utilization or persisted source text |
| Cost and local telemetry | Summary/stats explicit optional cost/true_ttft/decode fields, missing-usage tests | Unavailable; no known subscription billing basis or implemented local backend exposing prefill/decode/memory/KV metrics |
| Persistent traces and reconstruction | Bounded private schema-1 JSONL projection and reader; disconnected/headless multicall, cancellation/failure, privacy sentinels and malformed-prefix/schema tests | Met as ADR 0009 lifecycle/measurement inspection; no transcript, event sourcing or recovery |
| trace/list/stats and completeness | CLI isolated settings/inspection fixture, reader and stats coverage assertions | Met; unknown differs from zero, partial/unreadable/unrecorded coverage explicit |
| Optional observability preserves execution | Off-mode, disk-limit-after-committed-edit, queue/deadline and runtime regressions | Met; default off, frozen per run, separate recorder diagnostics, no disabling mandatory events |

Recommendation: **close Phase 4 with explicit limitations**, including no live
subscription telemetry acceptance. P4-06 requires separating live evidence from
mocks; it does not require inventing unavailable provider data or a live call.
No required implementation gap was found under accepted ADRs 0008/0009. The
[shared validation and debt dispositions](foundation-closure.md) record 189 passing
tests and the subsequently approved maintainer decision. Phase 4 remains active
until a separate successor authorization; ADR 0013 is accepted with implementation
Not Started.

Current retrospective: preserve the six lessons and measured historical overhead
below. Sessions are now resumable at idle boundaries under ADR 0012 in a separate
content store; metadata traces still cannot resume them. Total retained history,
recorder startup/capacity scan overhead, store contention and syscall lifetime
remain debt with future triggers in the shared audit. Neither first text nor
inclusive intervals should be relabeled as token/prefill/decode performance.
Next recommended milestone is scoped Mac A after foundation disposition and
ADR 0013 acceptance, not automatic Phase 5 or background-service implementation.

Closure resolution (2026-10-10): the maintainer explicitly approved administrative
closure of Phases 2, 3 and 4 provided existing exit criteria were verified and
limitations accurately documented. The current matrix and shared validation
record satisfy those conditions within the accepted contracts. No unverified live
telemetry, unavailable metric or new runtime guarantee is marked complete.
Phase 4 remains the active marker. ADR 0013 is separately accepted, with Mac A
implementation Not Started and separate implementation authorization required.
The remaining decision/status statements below preserve the historical review.

## Historical exit criteria and evidence (2026-10-08)

| Criterion | Implementation / evidence | Limits |
| --- | --- | --- |
| Turn optional observability on/off | settings show/set; run overrides; immutable explicit runtime options; isolated CLI and config tests | Off by default; no dynamic per-run reconfiguration; old traces retained |
| Provider/model and token/cache usage | Optional provider identity and typed sink telemetry; actual HTTP fixtures with absent/zero/partial/invalid usage | No subscription billing assumptions or synthetic tokens |
| Model and tool timing | Monotonic runtime offsets; adapter preparation/auth/dispatch/first text/attempt; actual tool-future duration; fixed-timeline and stalled-consumer fixtures | Inclusive intervals overlap; no true token/prefill/decode timing |
| Context utilization | Request bytes, labeled heuristic, budget and retained/evicted/summary byte counts | Local admission utilization, not actual model context capacity; source text not stored |
| Persistent inspectable traces | Private bounded schema-1 JSONL metadata; trace list/lookup; headless and disconnected multicall acceptance | Not content replay, session recovery, or cryptographically authenticated history |
| Trace/stats commands | JSON summaries with correlated call outcomes, optional usage, distinct waits; known sums plus reporting-call coverage | Cost and decode rate unavailable; unrecorded runs unknown |
| Failure/cancellation/privacy invariants | Denials, committed edit after storage failure, pre/in-stream cancellation, failed provider, queue/deadline, store limits, reader/gap/suffix/schema/symlink fixtures | A blocked OS syscall can outlive the runtime's bounded recording finalization |

Recommend closing Phase 4 with these limitations after maintainer review. No
required implementation slice remains. This is a solo scope/diff review, not an
independent adversarial-model review. Sessions/modes/UI are inherited from the exact `010a36d` base. Validation includes
its deterministic suites but does not attribute those features to Phase 4. Merge
order is sessions/modes first, observability second; the observability PR targets
`feat/interactive-sessions-modes`, not the Phase 3 branch.

## Provider research

On 2026-10-08, fetched the current official
[Responses reference](https://developers.openai.com/api/reference/python/resources/responses/methods/create)
with a direct HTTP read after the web tool rejected the oversized page. The
reference includes usage fields for input/output/total tokens and input cached
tokens. The adapter reads that shape only when the actual completed envelope
supplies it. Public API documentation is not evidence that the subscription backend
always supplies usage. There is no live subscription-telemetry or live billing
verification in this review. Deterministic local HTTP fixtures exercise the actual
subscription adapter, not a separate invented provider.

## Validation

- `cargo test --locked`: 153 passed, 0 failed, 1 normally ignored overhead experiment across the current runtime/provider/context/tool/CLI/session suites.
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `git diff --check` and relative Markdown link validation: passed.
- CLI fixtures use an isolated HOME and require no authentication: settings default/update, malformed configuration, stats/list and unsafe-ID rejection.
- Recording acceptance performs two real local HTTP model calls and a committed write, followed by trace read/summary/stats; privacy sentinels in task/instructions/path/result/text/private reasoning/credentials are absent on disk.
- Fixed timestamps establish deterministic duration/permission semantics. HTTP mocks establish wire usage semantics. The stalled-consumer fixture establishes delivery wait independently of adapter first-visible-text timing.
- Saturated queue and deliberately nonresponding worker completion exercise bounded finalization. Real filesystem limits/unavailable directories exercise writer errors without changing execution outcomes.
- Reader fixtures cover complete files, valid crash prefixes, invalid suffixes, sequence gaps, incompatible schema, symlink files/stores and unsafe lookup IDs.

No live model or manual terminal acceptance is claimed for Phase 4.

## Recorder overhead experiment

Command: `cargo test --locked --test observability recording_overhead_experiment -- --ignored --nocapture`.

Isolated macOS observability worktree based on `010a36d`, debug profile, non-Git
temporary workspace, no network,
one final-response mock per run, alternating 21 runs per mode:

| Mode | Median wall time |
| --- | --- |
| Off | 9,974 microseconds |
| On | 13,614 microseconds |

On ranged from 11,943 to 17,962 microseconds. The observed median addition was
3,640 microseconds (about 36.5% of this deliberately tiny mocked run). This includes
worker startup, private file writes, repeated bounded capacity scans, sync and
publication. It is not a model-latency benchmark or a claim about live-task
percentage overhead. No optimization claim is made.

## Retrospective

**What did we learn?** Useful tracing starts with clear boundaries and missing-data
semantics. Existing public events are suitable for live presentation but unsuitable
for automatic content-safe persistence. Recorder diagnostics must be independent
of task outcomes.

**Which assumptions were wrong?** A closed file handle alone did not reliably
release the tested store lock in this environment; an explicit unlock guard fixed
the failure and has a regression test. Optional usage cannot be assumed present
just because the API reference describes it. A timer at a downstream consumer
measures client delay as well as execution.

**Which abstractions proved useful?** Typed execution IDs/sequences, an allowlist
projection, optional sink telemetry, immutable per-run options, bounded nonblocking
record submission and an independent recording report.

**Which abstractions are premature?** A database, external tracing framework,
pricing service, universal provider capabilities, content trace modes, session
resumption, remote export and a new dashboard.

**What debt would distort the next phase?** Conversation/provider assembly remain
unbounded. Capacity scans and per-run worker startup add measured overhead. One
worker may remain in an OS syscall after timeout; no unsafe cancellation is attempted.
Concurrent recorder lock contention conservatively rejects recording. Local backend
telemetry will need genuinely measured prefill/decode fields, not reuse of first-text
or inclusive hosted timing labels.

**What should not be carried forward?** Treating unknown as zero; cached tokens as
additional input tokens; first visible text as first token; inclusive latency as pure
inference; complete recording as verified task success; partial trace absence as
proof an operation never happened; metadata traces as a full transcript or checkpoint.

## Disposition

ADRs 0008 and 0009 are accepted and implemented. Phase 4 is active by explicit
maintainer instruction. Phase closure remains a separate disposition; no Phase 5
integration is introduced or authorized.


## Stacked PR scope audit

Compared the isolated observability worktree directly with `010a36d` and verified
an exact 24-file allowlist. `Cargo.toml`/lock, `src/context.rs`, `src/events.rs`,
`src/permissions.rs`, `src/sessions.rs`, ADRs 0010/0011, session review, research,
greeting fixture content, and the rest of the terminal UI have zero delta.
The seven existing config-fixture files (including sessions and PTY fixtures)
change only by adding `observability: None`. Main/runtime changes are recorder,
telemetry, settings and inspection integration; they do not repeat session/mode
implementation. The base README conversation wording and rejection display after
console cleanup are preserved. Observability uses the base's `model_directory`
settings seam. No change was made to the shared main checkout during extraction.


## 2026-10-09 authorized session persistence extension

The maintainer required non-temporary sessions and names from the first submitted
chat. ADR 0012 supersedes only ADR 0010's ephemeral-storage clause. Phase 4 remains
active; metadata-only trace retention remains unchanged. Session snapshots are a
separate private workspace store containing full conversation/tool data and provider
continuation, with atomic idle saves, schema validation, byte ceilings, and an
exclusive writer lock. Permissions and authentication are not restored from it.

Solo review checked persistence against whole-exchange selection, provider response
completion, permission recomputation, first-message naming, incomplete terminal
batch rejection, cross-workspace isolation, and atomic failed-save behavior. A
long-name footer regression was corrected by keeping the compact SessionId in the
footer and the derived names in the picker. A real-write restart fixture verifies
that manually changed file contents are not overwritten by restoration.

Validation passed: `cargo test --locked --test session_store --test sessions`,
`cargo test --locked --bin astrid`, `cargo test --locked`, `cargo fmt --check`,
`cargo clippy --locked --all-targets -- -D warnings`, and `git diff --check`.
Limitations: restore the last successful idle snapshot only; no mid-run checkpoint,
background execution, simultaneous interactive writers in one workspace, or recovery
of conversations already lost by earlier releases. Unsupported/corrupt/oversized
stores fail explicitly; save failures do not relabel runtime outcomes.

Protected context overflow now reports numeric admission limits and category sizes.
Interactive environment overrides permit explicit budget changes without silently
increasing defaults or pruning protected exchanges. The rejected request is not
proof of exceeding a provider context window.


### Default allowance adjustment

On 2026-10-09 the maintainer requested a larger default after an architecture
question stopped with a 32,251 input heuristic against the former 28,672 usable
allowance. The default total allowance is now 65,536, leaving 61,440 for input
with the existing 4,096 reserve. CLI and interactive/library defaults share the
same ContextBudget value; explicit flag/environment overrides remain. The existing
524,288-byte ceiling and protected-exchange policy are unchanged. A regression
fixture uses the reported 133,328-byte request and input heuristic to establish
admission under the new default and rejection under the old default, while keeping
oversized byte requests rejected. This is an admission policy change, not a claim
about provider capacity or task success.
