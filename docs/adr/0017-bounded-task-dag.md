# ADR 0017 — Bounded process-local task DAG and worker ownership

Date: 2026-10-16
Status: Accepted — 2026-10-16
Implementation: Implemented in the authorized thread; [review and evidence](../reviews/phase-8.md)
Decision evidence: Maintainer approved ADRs 0017–0019 and instructed “I approve begin implementation, new thread, new branch.”
Related: [Phase 8 plan](../plans/phase-8.md), ADRs 0002, 0012, 0016;
[authority/results proposal](0018-worker-authority-and-results.md),
[event delivery proposal](0019-batch-events-and-traces.md)

## Context

`runtime::run` executes one sequential agent loop in a fresh Session. A model call
is not a worker. `Cancellation::clone` shares one signal; it does not create a child
scope. Events have per-run ordering, and idle snapshots contain conversations,
not active jobs. Phase 8 needs independent tasks with dependencies without inventing
worktree isolation, restart recovery or a daemon.

## Options

1. Continue sequential independent runs. Retains current semantics but cannot
   satisfy Phase 8's concurrent-task exit criterion.
2. Bounded, static, process-local DAG; one coordinator owns assignment and result
   publication, with fresh child runs and explicit dependencies. No retry/recovery.
3. Durable/distributed scheduler with leases, retries and recovery. Requires new
   authority, persistence and side-effect contracts beyond this first scope.

## Accepted decision

The maintainer selected option 2; implementation is explicitly authorized.

- A caller submits a complete graph through a headless API or bounded CLI manifest.
  Models cannot add tasks, edges, permissions or workers. Validate the entire graph,
  configurations and budgets before authentication or dispatch. Reject cycles,
  duplicate keys/edges, self-dependencies and unknown dependencies.
- Introduce BatchId, TaskId, WorkerId, TaskSpec, TaskState and TaskResult where used.
  A Batch groups submitted work; it is not a synthetic Run containing child runs.
  Each task performs one fresh `runtime::run` with its own RunId/SessionId, history,
  context, permission handler and cancellation signal. No shared conversation or
  private provider continuation. A worker is an execution slot assigned a complete
  task, potentially containing several model/tool turns.
- The coordinator alone owns graph state and assignment. One task has at most one
  active owner and one attempt. A slot is not reusable until its child has returned
  a terminal RunResult or an explicit worker-failure observation has been joined.
  Ownership records are process-local; do not add expiring leases without retries
  or remote ownership. Reusing a slot never reuses its previous session.
- Dependencies are all-required: a task becomes ready only after every declared
  predecessor has completed execution and published an accepted bounded result.
  Ready ties use manifest order; observed completion order does not reorder inputs.
  A failed/cancelled predecessor skips its descendants with explicit causal IDs.
  Unrelated branches continue. No automatic retries, fallback or speculative starts.
- Parent cancellation stops admission, cancels queued tasks, signals each running
  child's distinct token and joins running work. Task-only cancellation affects
  that task and its dependent branch; independent siblings continue. Children cannot
  cancel siblings/parent by sharing the parent's Cancellation handle. Cancellation
  is cooperative, not rollback or proof that a remote request stopped billing.
- Terminal child facts are preserved. If completion committed before cancellation
  was acknowledged, retain that completion. A cancellation accepted before the
  batch's terminal commit makes the batch Cancelled unless any execution/publication
  failure makes it Failed. Failure has precedence over cancellation; Completed
  requires all required tasks completed with accepted results. Neither Completed
  nor a successful synthesis is independent evidence of factual task success.
- Pending/ready tasks can become Cancelled without fabricated RunIds. Running tasks
  can enter Cancelling, then terminate according to the actual child outcome.
  Skipped dependency work is distinct from a child that ran and failed. Terminal
  transitions are immutable; duplicate cancellation is harmless.
- Use explicit fixed ceilings. Proposed local defaults: 32 tasks, 128 edges,
  concurrency 2 (hard maximum 8), 1 MiB manifest, 64 KiB rendered task input, 64 KiB
  transferable final text per task and 8 model calls per task. Reserve each task's
  call ceiling upfront; their sum must fit an explicit batch call ceiling (default
  64). No hidden additional inference for planning or summarization. Numeric defaults
  may be adjusted with evidence without changing ownership/authority semantics.
- Bounded wall deadlines cover task and whole batch, using the existing cooperative
  cancellation path; deadline reasons remain distinct from user cancellation.
  Proposed defaults: 120 seconds per task, 300 seconds per batch. Never turn an
  elapsed deadline into a promise of instantaneous cleanup or hard process killing.
  Cleanup/recording overhead is observed separately from admission deadlines.
- Batch and child execution live within the invoking process. No active graph,
  worker history or task results are automatically written into idle sessions.
  Existing session v2, permissions and original parent conversation remain unchanged.
  Process exit can lose work/results; metadata inspection is not restart recovery.

## Consequences

The smallest useful slice is two independent read-only inspections overlapping in
one process, with owned results and cancellation. Add a dependent reconciliation
node after that slice. Shared workspace reads are not snapshot isolation: external
edits may change observations. No writer, shell, worktree, cross-device worker,
external coding-agent harness, group identity or background service follows from
this decision. Accepted ADR 0018 constrains authority and result transfer.

The coordinator must not be the only code polling cancellation while blocked on
presentation. Internal child lifecycle drains and joins must progress independently
of an external event consumer; ADR 0019 defines the proposed new batch stream.

Authentication currently uses a blocking file-lock acquisition inside
`spawn_blocking`. Dropping a waiting async caller does not stop that operation.
The implementation must test real lock contention/cancellation with synthetic auth
fixtures and fix any demonstrated orphan-wait problem before claiming joined
cleanup. Keep credential locking and account validation; do not duplicate token
stores or weaken locks to obtain concurrency. Existing native blocking reads also
remain cooperative, not preemptively killable. Bounded counts do not prove a global
memory or OS-thread bound for underlying libraries.

## Decision question

Approve a bounded static in-process DAG with exclusive one-attempt ownership,
independent sessions, all-required dependencies, continued unrelated branches and
scoped cancellation, while deferring durable jobs/retries/leases?

## Validation

Use barriers to prove two tasks enter inference before either is released; do not
infer concurrency from elapsed-time improvements. Test rejected graphs before
authentication/generation or task-tool dispatch, single ownership, stable input ordering, dependency failure chains,
worker panic/loss, task-only and parent cancellation races, deadline reasons,
aggregate call reservations, oversized input/result rejection and shared-auth lock
contention. Require no unfinished owned worker at return and no tool replay.

## Resolution

Accepted on 2026-10-16 in response to the explicit decision question for ADRs
0017–0019. The maintainer also authorized implementation in a new thread/branch.
The selected recommendation above is binding for this implementation; prior
planning-only scope is superseded. Phase 7 administrative closure is not invented.
Implementation and actual validation must be recorded slice by slice; no Phase 9.
