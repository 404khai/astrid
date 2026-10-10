# Phase 4 review: optional trace and inference observability

Date: 2026-10-08
Implementation baseline: `010a36d` (`feat/interactive-sessions-modes`)
Isolated branch/worktree: `feat/observability`, `/Users/admin/Developer/astrid-worktrees/feat-observability`
Status: Implementation complete; deterministic validation and solo review
Authorization: Maintainer requested "Implement the observability and its related adrs 8 & 9".
Maintainer closure decision: Pending
Next-phase authorization: None; AGENTS.md remains Phase 4
Separate previous-phase closure dispositions: Phase 2 and Phase 3 pending

## Exit criteria and evidence

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
