# ADR 0018 — Read-only worker authority and explicit visible-result reconciliation

Date: 2026-10-16
Status: Accepted — 2026-10-16
Implementation: Implemented in the authorized thread; [review and evidence](../reviews/phase-8.md)
Decision evidence: Maintainer approved ADRs 0017–0019 and instructed “I approve begin implementation, new thread, new branch.”
Related: [Phase 8 plan](../plans/phase-8.md), ADRs 0004, 0006, 0015, 0016;
[ownership proposal](0017-bounded-task-dag.md)

## Context

Parallel mutation in one workspace can invalidate read/edit assumptions. Phase 9
owns worktree isolation. Shell has broad account authority, and multiple permission
handlers cannot safely share one terminal prompt. Existing private responses also
cannot be passed between workers as if they were ordinary visible text. A dependency
edge must define both authority and what data it discloses.

## Options

1. Share the parent session, permission handler and mutable workspace freely.
   Small API change, but hidden conversational/approval races and unsafe mutation.
2. Fresh worker sessions, hard read-only capability ceiling, explicit bounded visible
   results only along caller-declared edges. Reconciliation retains all source results.
3. Parallel mutating workers now, using per-task worktrees/sandboxes and a centralized
   approval broker. Larger isolation, merge and compatibility scope belonging later.

## Accepted decision

The maintainer selected option 2 for Phase 8; this limitation is accepted, not a claim that parallel
coding changes are isolated safely.

- The requesting principal is the invoking local user; execution target is the
  explicitly selected canonical folder. This is the existing local-account trust
  boundary, not a new multi-user or cross-device identity scheme. The coordinator
  records which invocation granted each task's effective authority.
- All tasks, including synthesis tasks, have a hard read-only ceiling: native reads
  may run; write/edit and shell Execute are Deny. Repository content, model output,
  dependency data, a worker role or the parent's unbound mode cannot widen it.
- Effective capability action is the most restrictive of parent grant, task request
  and this phase ceiling: Deny dominates Ask dominates Allow. Preserve denial,
  path validation, hard-link/symlink checks and per-tool policy events in the runtime.
  An Ask has no interactive worker approval broker in this scope and therefore denies
  under ADR 0004; do not convert it to Allow or race prompts on stdin. A separately
  scoped later broker may narrow/approve actions, but is not implemented here.
- A manifest may request narrower capabilities, never grant itself wider authority.
  The model cannot submit a new manifest or assign capabilities. Every actual tool
  dispatch is checked, even if its tool schema is advertised to the model.
- Each task receives the caller's explicit task input, fixed runtime instructions and
  only the accepted visible final text of declared predecessors. No automatic full
  parent-conversation copy, sibling history, tool-output transcript, credentials,
  ModelResponse, serialized Session, private reasoning or continuation export.
- Represent transfers as typed TaskInput/TaskResult data before rendering prompts:
  source BatchId/TaskId/RunId, execution outcome, exact transferred text byte length
  and digest, completeness/validation status, and declared dependency identities.
  User task keys/titles are presentation labels, not authenticated identity.
  Retain this structure in memory and expose it in the final result for inspection.
- Transfer only successfully validated, committed final response text. Provisional
  deltas and incomplete tool batches never become dependency results. A Completed
  run whose result cannot be published within bounds retains its actual RunCompleted
  fact, but its task reports a separate result-publication failure; dependents skip.
- Render dependency text as attributed untrusted task data in a fresh request, never
  as system/developer instructions or executable ToolCalls. Do not claim delimiters
  eliminate prompt injection. Capabilities and declared transfer edges enforce the
  concrete boundary; the model can still misunderstand or fabricate a conclusion.
- Oversized transferred text or aggregate rendered input fails explicitly; do not
  silently truncate, auto-summarize through another model or borrow extra call budget.
  Existing context selection still enforces its independent byte/token estimate
  budget. Failure to include the mandatory transfer must be explicit; context
  pruning cannot silently remove a required predecessor result while claiming the
  dependent task consumed it.
- Reconciliation first produces a deterministic bundle in declared dependency order:
  source IDs, statuses and the original visible results, including failures/missing
  evidence. An optional caller-declared synthesis task consumes only its declared
  successful inputs and yields a separate, unverified interpretation. It cannot
  overwrite predecessor results or relabel failed work as successful.
- Conflicting claims retain both attributed inputs; any synthesized conflict marker
  is advisory, not a proof that all contradictions were detected. Show unavailable
  coverage and failed dependencies. No automatic merge, patch application, factual
  verification or write authority is implied by a reconciliation node.
- Worker private sessions are owned during their run and never attached to the
  parent idle session. After extracting bounded committed results and diagnostics,
  they may be released with the invocation; no new content persistence/retention
  format is introduced. Metadata traces exclude transferred text under ADR 0009.

## Consequences

Two inspections of the same trusted repository or notes folder can execute without
Astrid issuing concurrent writes. This does not prevent an outside process editing
files, provide OS confinement, or isolate secrets inside files already readable by
the local account. A read-only worker can still incur authorized inference usage.
Per-task accounting remains separate; summed token use is not a single context
window, and cumulative parent/child cost remains unknown where unreported.

Existing RunConfig/PermissionPolicy and visible-text projection can be reused, but
there must be no bypass around tool-time checks. RunCompleted remains an execution
fact. Call-number routes stay caller configuration, not automatic task classification.

## Decision question

Approve read-only workers under the intersection of explicit grants, with only
bounded committed visible results shared along declared edges and source-preserving
reconciliation, deferring concurrent writes and an approval broker?

## Validation

Executor spies prove denied write/edit/shell never invoke execution, including
malicious dependency text and a parent configured unbound. Test Ask without stdin,
path escapes, no sibling/parent/private sentinel in requests, transfer hashes/order,
no streamed/incomplete transfer, oversized publication, mandatory dependency input
under compaction, conflicting outputs preserved and failed work never promoted by
successful synthesis. Include non-Git notes fixtures; no test needs paid services.

## Resolution

Accepted on 2026-10-16 in response to the explicit decision question for ADRs
0017–0019. The maintainer also authorized implementation in a new thread/branch.
The selected recommendation above is binding for this implementation; prior
planning-only scope is superseded. Phase 7 administrative closure is not invented.
Implementation and actual validation must be recorded slice by slice; no Phase 9.
