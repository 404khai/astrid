# Phase 3 review: context engine

Date: 2026-10-07
Baseline: `debd385` plus the reviewed implementation worktree
Status: Implementation complete; final acceptance evidence below
Maintainer closure decision: Pending review disposition
Next-phase authorization: Not requested; AGENTS.md remains Phase 3

## Exit criteria and evidence

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
