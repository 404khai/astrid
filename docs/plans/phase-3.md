# Phase 3: context engine plan and implementation

Date: 2026-10-07
Status: Implementation complete; deterministic acceptance and adversarial review passed
Baseline: `debd385` (same source tree as inspected `c62df7a`)
Active phase: Phase 3; do not begin Phase 4
Phase authorization: maintainer requested adversarial critique and implementation
on 2026-10-07. Admission and compaction recommendations were explicitly accepted
later with: "use both recommendations, then open a pr for it on a new branch".
Previous-phase closure: separate [Phase 2 review](../reviews/phase-2.md) disposition
remains pending; this does not undo explicit Phase 3 authorization.

## Plain-language outcome

Astrid knows which information enters each model request, where it came from,
and why it stays or leaves. It protects instructions, the task, and the latest
complete tool exchange. Older exchanges are ranked and selected within configured
local limits. Omitted history can become one bounded, explicitly incomplete
summary. Inspection shows the actual selection, summary, source IDs, and sizes.

A ticket is a work order with proof of completion. An ADR records a consequential
design choice and why we made it. The two accepted ADRs below define what context
numbers, removal, and summaries promise.

## Review and decisions

The prior phase review identified unbounded conversation/instruction state and
warned that individual tool byte limits were not a context budget. Inspection
confirmed that the provider sends private continuation, serialized outcomes,
and tool definitions in addition to visible text. The
[adversarial review](../reviews/phase-3-adversarial.md) separated measurement from
admission and challenged tool closure, summary coverage, replay, and cancellation.

| Question | Resolution | Contract |
| --- | --- | --- |
| Must budgeting guarantee exact provider-token fit? | No. Label a non-opaque JSON size heuristic, retain unknown provider token cost, and independently enforce exact serialized bytes | [ADR 0006](../adr/0006-context-admission-and-selection.md), Accepted |
| What may leave the next request? | Only complete older assistant/continuation/tool-result exchanges; protect instructions, task, latest complete exchange | ADR 0006 |
| How are summaries produced? | Deterministic bounded task-data condensation, not extra model calls; retain lineage and explicit incompleteness | [ADR 0007](../adr/0007-context-compaction.md), Accepted |
| What does the budget bound? | Selected request size; original ephemeral history and runtime evidence remain separately retained without a total memory guarantee | ADR 0006 consequences |
| How does file relevance work? | Compare recency against explicit requested-file references and lexical path signals in fixed fixtures, with recency tie breaking; no extra file reads | Reversible local policy; P3-05 |
| How is context inspected? | Runtime events/state and opt-in `--show-context` during an ephemeral run; historical standalone inspection remains deferred to persistence work | P3-06; no new persistence contract |

Historical sequence: P3-02 passive accounting preceded accepted admission rules;
P3-08 source provenance followed while answers were pending. The maintainer then
accepted both recommendations, enabling P3-03 through P3-07. Pending proposals
were never silently recorded as consent.

## Tickets and delivery evidence

| ID | Outcome | Dependencies | State |
| --- | --- | --- | --- |
| P3-00 | Record phase authorization and separate previous-phase closure | Maintainer decisions | Authorization done; Phase 2 closure disposition pending |
| P3-01 | Resolve context admission/compaction contracts | Grilling; ADRs 0006/0007 | Done |
| P3-02 | Measure the actual prepared request | Phase authorization; provider boundary | Done |
| P3-08 | Track stable context sources and addition reasons | Existing runtime/event contracts | Done |
| P3-03 | Select valid complete exchanges within local limits | P3-02, P3-08, ADR 0006 | Done |
| P3-04 | Condense omitted history honestly | P3-03, ADR 0007 | Done |
| P3-05 | Compare simple relevance policies | P3-03 | Done |
| P3-06 | Inspect selected context and summary lineage | P3-03 through P3-05 | Done |
| P3-07 | Verify, review, and document the complete slice | Implementation tickets above | Done |

Implementation owner: Codex root agent. Independent adversarial reviewer:
`/root/phase3_adversarial`. These are sequential implementation slices, not new
Astrid workers or scheduler features.

### P3-02: Measure the actual request

Problem: runtime-visible messages omit provider continuation and tool definitions.
Scope: adapter-owned counting of the same UTF-8 JSON body sent over HTTP, with
metadata publication before authentication/dispatch. No private continuation dump.
Acceptance: exact request byte totals, source-category sums, escaped Unicode and
JSON outcomes included, unknown provider token count, opaque reasoning excluded
from the labeled heuristic. Providers may omit accounting only in unbudgeted
library runs; configured budgeting fails explicitly if accounting is unavailable.
Evidence: `tests/context.rs`; local HTTP capture checks and sentinel privacy tests.

### P3-08: Track source identities

Problem: individual committed messages had no stable context identity or origin.
Scope: Session ledger links typed item IDs to original history indices, operating
and repository instructions, task, model IDs, and tool IDs. Requested native file
path labels are capped at 1,024 UTF-8 bytes with explicit truncation. No content
copies or successful-read/freshness claims are inferred from requested paths.
Acceptance: one item per committed message, correlated origins, addition reasons,
atomic rejection of forged indices/duplicate origins, replay matching Session and
ExecutionState, cancelled/skipped/provisional work not creating committed items.
Evidence: `tests/context.rs`, existing runtime cancellation/denial/error fixtures.

### P3-03: Select valid exchanges

Problem: every call previously resent all committed history.
Scope: explicit ContextBudget/ContextSelection; adapter preflight accounting;
protected instructions/task/latest exchange; greedy selection of other complete
exchanges in policy order. Original history and global reused-call-ID checks stay
independent of the selected request. Commit selection only after cancellation
checks, before allocating/counting the next model call. Compare the adapter's
actual prepared snapshot with preflight before authentication/HTTP dispatch.
Acceptance: no split batches or orphan results, denied/failed outcomes preserved,
protected overflow and unavailable accounting fail without inference, cancelled
preparation preserves prior selection, accepted metadata matches sent requests,
replay rejects duplicate selection, protected eviction, split exchanges, forged
lineage, and mismatched prepared snapshots atomically.
Evidence: `context::tests`, `tests/context_budget.rs`.

### P3-04: Condense omitted history

Problem: full-exchange removal can lose useful historical observations.
Scope: one bounded summary rebuilt from original omitted history; visible assistant
excerpts and tool status, requested path, path-label truncation, coverage, result
truncation, and bounded content excerpts. Private reasoning is never interpreted
or copied. Source IDs identify inputs considered, not lossless retention.
Acceptance: bounded UTF-8 text, explicit incomplete/stale task-data label, one
summary per request, no extra inference/tool dispatch, real source lineage and
inspection text. If a summary cannot fit, keep the valid uncondensed candidate;
protected overflow or cancelled preparation leaves the last committed selection
unchanged. This fallback is an ordinary selection outcome, not a committed failed
summary. No accumulating summary-of-summary transcript.
Evidence: long Unicode/private-reasoning/coverage fixtures, exact-fit fallback,
cancellation/error preservation, and actual HTTP summary content checks.

### P3-05: Compare relevance rules

Problem: recency can discard an older file explicitly named in the task.
Scope: configured `recency` and `file-references` policies; the latter ranks exact
requested-file mentions, then lexical path components, then recency. No filesystem
reads, semantic index, dependency graph, or learned ranking.
Acceptance: deterministic fixture compares both policies under the same request
allowance; records full exchanges retained/omitted and summary limitations.
Evidence: [comparison](../experiments/phase-3-context-selection.md) and
`selection_compares_recency_and_file_relevance_without_reading_more_files`.
The comparison measures context retention, not model task success.

### P3-06: Inspect context used for each call

Scope: ContextItemAdded, ContextSelected, and ContextPrepared are runtime-owned
structured events. Selection records retained/evicted IDs and reasons, configured
limits, incomplete summary text/lineage, and measured request size. Combined
ContextSelected records selection/removal/compaction atomically; no separate event
can claim a summary committed while its selection did not. CLI consumes events.
Acceptance: inspection agrees with request capture; public metadata never contains
private continuation; summary text is bounded visible task data; headless state
replays independently; selection committed before cancellation stays observable
without subsequent model dispatch. No standalone persistent `astrid context`.
Evidence: local HTTP/replay/forgery/stalled-publication fixtures; CLI flags/help.

### P3-07: Validation and closure review

Acceptance: complete small-budget run exercises inspection, pruning, compaction,
accounting, and normal completion; companion fixtures cover mandatory overflow,
cancellation, drift, and global call IDs. Existing phase invariants remain green.
Independent review material findings are resolved before publication. Write phase
review without treating implementation completion as phase closure/Phase 4 consent.

Final validation on 2026-10-07 in the implementation worktree based on `debd385`:

- `cargo test --locked`: 110 passed, 0 failed.
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `target/debug/astrid run --help`: context inspection, policy, and limit flags present.
- `git diff --check` and local Markdown link-target validation: passed.
- Independent reviewer: six context-budget and five context-unit tests passed;
  no material blocker remains. Root final review was a separate solo review.

The [Phase 3 review](../reviews/phase-3.md) records exit criteria and limitations.
No live model or interactive terminal acceptance was run for this phase. Earlier passive/provenance slices passed 97 and 98
checks respectively; those historical numbers are not the final suite total.

## Configuration and limits

CLI defaults: estimated context allowance 32,768; response reserve 4,096;
serialized request ceiling 524,288 bytes; summary ceiling 4,096 UTF-8 bytes;
`file-references` policy. Configure `--context-tokens`, `--response-reserve`,
`--context-bytes`, `--summary-bytes`, and `--context-policy`. The reserve subtracts
planning headroom; it does not enforce provider output length or advertise model
capacity. Library callers can explicitly use `context_budget: None` to preserve
unbudgeted behavior.

The heuristic is ceil(non-opaque serialized JSON bytes / 4), not a tokenizer
count or full request token estimate. If continuation contains reasoning, the
entire assistant category is excluded from that heuristic, including its readable
text/calls. The exact byte ceiling still includes everything. Provider overflow
remains possible. Preparation/counting are synchronous; cancellation is checked
between measurements and before commits/dispatch. Original history, event/state
metadata, model output, and provider assembly remain outside total-memory bounds.

Preserve one package/provider, sequential tools, ephemeral sessions, permissions,
workspace boundaries, and existing lifecycle semantics. No persistence, vector
search, embeddings, routing, workers, worktrees, MCP, or new dashboard.
