# Phase 3 review: context engine

Current audit: 2026-10-10, `cdebe18` plus closure worktree changes
Status: Administratively closed with documented limitations; solo current audit
Maintainer closure decision: Approved on 2026-10-10, conditional on verified exit
criteria and accurate limitations; the current audit below supplies that evidence
Current phase: Phase 4; separately authorized 2026-10-08
Historical review: 2026-10-07, `debd385` plus reviewed implementation worktree

## Current exit-criteria audit

The exit criterion is inspecting, budgeting, compacting and explaining the actual
working request context.

| Requirement | Current code and rerun evidence | Assessment / limit |
| --- | --- | --- |
| ContextItem/Source/Budget/Snapshot/Selection and provenance | [context.rs](../../src/context.rs), model/runtime/events; source/history-index and independent replay tests | Met; requested file labels do not establish successful/current reads |
| Token estimation/counting and context budget | [openai.rs](../../src/openai.rs) actual request accounting; context/context_budget HTTP, Unicode/opaque, reserve/byte and default-budget fixtures | Met as ADR 0006 estimate plus exact byte ceiling; provider fit remains unknown |
| Message pruning | Complete-exchange selection in context and checked events; multi-tool denied/success, overflow, reused-ID and follow-up history fixtures | Met; instructions/original/current tasks/latest exchange protected, original history retained |
| Compaction | Deterministic bounded summary; privacy, coverage/path-before-excerpt, lineage, fallback and cancellation tests | Met under ADR 0007; incomplete/stale, no semantic equivalence or extra inference |
| File relevance heuristics/experiment | File-reference/lexical and recency policies; fixed selection comparison fixture and recorded experiment | Met; retention comparison supplies no task-success evidence or semantic index |
| Inspection and reasons for addition/retention/eviction/compaction | Runtime ContextItemAdded/Inherited/Selected/Prepared, CLI show-context; sent-wire match and replay/forgery tests | Met; live headless state/CLI inspection, metadata traces omit source text; no required standalone historical context command |
| Preserve previous guarantees | Runtime preflight, preparation drift checks and permission/cancellation suites | Met; rejected admission starts no inference/auth, cancellation checks surround synchronous measurement |

Recommendation: **close Phase 3 with explicit limitations**. No remaining required
implementation work was found. [Shared validation and debt dispositions](foundation-closure.md)
record the current 189-test suite and closure decision. Historical Phase 2/3
adversarial findings have regression coverage; this current audit is solo.

Current differences: the default estimated allowance is **65,536**, response
reserve 4,096, exact request ceiling 524,288 bytes and summary ceiling 4,096 bytes
(`ContextBudget::default`, CLI uses that default). ADR 0010 extends whole-exchange
selection to follow-up submissions; ADR 0012 persists private idle conversations
and provenance independently of metadata traces. Targeted read results include
numbered lines and coverage within bounded JSON; corresponding wire-accounting
fixtures still assert eviction and compaction. These accepted extensions do not
erase the original limits or silently change the provider-fit guarantee.

Current retrospective: retain the six lessons below. Ephemeral-only history and
no persistence are historical scope statements now extended by ADR 0012. Model
summaries, embeddings and dependency indexing remain premature. Total retained
history/assembly memory and synchronous copying remain debt; bound future client
history projections without claiming the request budget bounds all memory. Do
not equate summaries with fresh file evidence, idle snapshots with active-run
recovery, or selection quality with task success. Only closure disposition remains.

Closure resolution (2026-10-10): the maintainer explicitly approved administrative
closure of Phases 2, 3 and 4 provided existing exit criteria were verified and
limitations accurately documented. The current matrix and shared validation
record satisfy those conditions within the accepted contracts. No unverified live
telemetry, unavailable metric or new runtime guarantee is marked complete.
Phase 4 remains the active marker. ADR 0013 is separately accepted, with Mac A
implementation Not Started and separate implementation authorization required.
The remaining decision/status statements below preserve the historical review.

## Historical exit criteria and evidence (2026-10-07)

| Criterion | Implementation | Evidence | Limits |
| --- | --- | --- | --- |
| Explicit context items, sources, budgets, snapshots, selection | context/model/runtime/events | Source/history-index replay, configured selection and HTTP snapshots | Ephemeral; no persistence |
| Count/estimate context honestly | OpenAI adapter and ContextBudget | Wire-byte/category totals, Unicode/escaping, opaque sentinel, reserve/byte boundaries | Provider tokens unknown; heuristic excludes whole opaque assistant category |
| Prune valid message history | Whole assistant/tool exchanges, protected guidance/task/latest exchange | Multi-tool success/denial closure, protected overflow, global reused-call ID test | Original history remains retained |
| Compact old context | Bounded deterministic visible summary | File identity/coverage before excerpts, private reasoning exclusion, exact-fit fallback | Explicitly incomplete/stale; no semantic equivalence promise |
| File relevance heuristics | Recency or file-reference/lexical policy | Fixed fixture comparing retained full target/noise exchanges | Retention comparison, not task-success evidence |
| Inspect/explain working context | ContextItemAdded, ContextSelected, ContextPrepared; CLI | HTTP selection/summary match, reasons/source IDs, replay/forgery fixtures, CLI help | No standalone historical lookup |
| Preserve failure/cancellation/permission contracts | Runtime preflight and existing execution boundaries | Auth0 on overflow/drift, unavailable accounting, cancellation during measurements/stalled publication, full previous suites | Synchronous accounting cancellation is cooperative |

Recommendation: close Phase 3 with explicit limitations after maintainer review.
No required implementation ticket remains. Previous-phase closure disposition is
still separately pending; neither this review nor the PR advances to Phase 4.

## What we learned

**What did we learn?** Serialized request size, readable-size heuristics, and
provider token usage are different facts. Provider-owned preparation is the right
place to account for continuation and tool definitions. Context selection can be
observable without copying private reasoning into runtime events.

**Which assumptions were wrong?** Clipping generic tool JSON does not preserve
its meaning: long read content can hide path and coverage fields. Those fields
must precede excerpts. A dispatch path that protects context does not automatically
make event replay reject forged protection/lineage. Cancellation checked before a
synchronous measurement can still become a false failure unless checked after it.
The independent reviewer exposed all three gaps, which are now resolved.

**Which abstractions proved useful?** Existing typed execution IDs, checked
transitions, and provider privacy boundaries support context provenance and
selection. A pure accounting method, one ContextBudget, complete-exchange units,
and one atomic ContextSelected event are enough for this slice.

**Which abstractions are premature?** Persisted context/history, summary model
calls, vector search, repository dependency indexing, model routing, multiple
providers, extra crates, and a separate context dashboard remain excluded.

**What debt would distort the next phase?** Runtime history and response assembly
remain unbounded independently of selected requests. The heuristic cannot expose
actual encrypted-reasoning token cost, model output limits, cache usage, or task
success. Repeated synchronous serialization/state copying can cost time; measure
before optimizing. Future traces must distinguish preparation, selection, attempted
model lifecycle, and actual provider telemetry rather than infer unavailable facts.

**What should not be carried forward?** Do not equate admission with guaranteed
provider fit, summary lineage with complete retention, requested paths with actual
successful reads, stale excerpts with current file content, a response reserve
with enforced output tokens, or normal run completion with verified task success.

## Validation record

Root-agent validation in the macOS implementation worktree, 2026-10-07:

- `cargo test --locked`: 110 passed, 0 failed.
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `target/debug/astrid run --help`: inspection/policy/limit flags present.
- `git diff --check` and local Markdown link-target validation: passed.
- Local HTTP fixtures measure actual request Content-Length and compare it with
  prepared/selected snapshots. The complete four-call fixture exercises reading,
  selection, compaction, and final response; a companion fixture rejects a reused
  call ID after its old exchange is removed.
- Failure fixtures cover protected overflow before inference/authentication,
  unavailable accounting, preflight/dispatch drift, cancellation during accounting
  including accounting error, and stalled selection publication before dispatch.
- Replay fixtures reject forged task protection, split old exchanges, lineage,
  duplicate selection, prepared sizes, message indices, and duplicate origins.
- Existing Phase 0/1/2 provider, permission, cleanup, output-bound, Git, and native
  tool fixtures remain in the full suite.

Separate adversarial agent independently passed six context-budget integration
fixtures and five context-unit fixtures, and found no unresolved material blocker.
See [the adversarial record](phase-3-adversarial.md). It did not independently run
all previous-phase tests. Root final review is a separately labeled solo review.
No live provider or interactive-terminal acceptance was performed for Phase 3;
mocked transport acceptance is not presented as live model evidence.

## Disposition

All implementation outcomes are covered by deterministic evidence. Recommend
closing with the documented estimate, summary, retention-memory, and cancellation
limitations. Maintainer closure remains pending. Phase 4 persistence/telemetry
must be separately authorized; no such feature is introduced here.
