# Phase 8 planning readiness review — 2026-10-16

Subsequent disposition: maintainer approved ADRs 0017–0019 and Phase 8 implementation
in a new thread/branch. The planning findings below preserve their checkpoint;
accepted contracts and the current handoff are in [CURRENT](../CURRENT.md).
Implementation is now complete; see the [implementation review](phase-8.md) for
actual source changes, validation and limits. This readiness audit remains historical.

Review: Solo repository/contract review; no independent agent review or new runtime
acceptance. Planning authorized by the maintainer's instruction to begin the Phase 8
plan. Scheduler implementation has not begun. [Plan](../plans/phase-8.md).

Source reviewed: `395af2c3` in the existing checkout. GitHub reports [PR #15](https://github.com/404khai/astrid/pull/15)
merged on 2026-10-16 at `16d755c4f04c70d07eed23e9302f725176f3960f`. No local
branch/worktree switch. Merge is not recorded as a separate administrative closure
of Phase 7 or approval of the new Phase 8 architecture.

## Previous-phase exit and evidence disposition

| Phase 7 criterion / follow-up | Existing evidence | Carry-forward disposition |
| --- | --- | --- |
| Multiple model classes in one run under observable deterministic routes | Original live notes run plus eight controlled coding/notes comparison episodes; route/model/effort metadata | Met with narrow provider/model scope; class labels are intent, not proven competence |
| Supported provider-aware effort and visible route reason | Exact verified model table, request validation, per-call trace events and runtime tests | Reuse inside each worker; no automatic task classification |
| Continuation compatibility and persistence | Accepted ADRs 0015/0016; routed/model-changing/v1 history uses visible replay, v2 dispatch provenance; restart/permission/private-sentinel tests | Earlier one-shot persistence debt resolved; child sessions still must not be shared |
| Context visibility | Estimated local input-budget meter with opaque/unknown labels | Reuse per child; never aggregate percentages into a fictional model window |
| Empirical routing comparison | Two samples per task/config, all task rubrics pass, higher routed coding input here | No quality/speed superiority; not a basis for cost-aware automatic worker selection |
| Validation baseline | 218 Rust passed, 0 failed, 1 optional ignored; 2 Python passed; fmt/strict clippy; pinned source hashes verified during publication | Prior Phase 7 evidence only; no new Phase 8 test claim |

[Phase 7 review](phase-7.md) contains historical one-shot restrictions and pending
validation entries. [Follow-up review](phase-7-followups.md) and accepted ADR 0016
supersede those specific statements. Do not inherit the older inspection-only
restriction as current truth, or interpret idle persistence as durable execution.

## Source findings and implications

| Source | Observed behavior | Planning consequence |
| --- | --- | --- |
| [runtime](../../src/runtime.rs): `run`, `run_in_session`, `RunConfig`, `RunResult` | A fresh run owns a Session; continuation validates workspace/history and source compatibility | Compose fresh child runs; do not clone a parent session into workers or manufacture a parent Run |
| [cancellation](../../src/cancellation.rs) | Clones share one watch sender | Child cancellation must use fresh handles and explicit one-way propagation |
| [permissions](../../src/permissions.rs), [tools](../../src/tools.rs) | Read-only helper denies writes/execute; tool-time policy/path checks remain distinct from provider prompts | Hard read-only ceiling plus grant intersection; no shared interactive handler |
| [events](../../src/events.rs) | State transition precedes event; child sequence belongs to its RunId | New batch/task reducer and correlation; keep child sequences intact |
| [runtime](../../src/runtime.rs): `Execution::flush` | Attached receiver backpressure may hold delivery open; disconnection detaches | Never make UI forwarding the only child drain; new batch delivery policy requires a decision |
| [model](../../src/model.rs) | Provider is Send + Sync; responses have private continuation; completion commits typed results | Interface supports concurrent callers, not proof of backend capacity; export only committed visible task result data |
| [auth](../../src/auth.rs): `lock`, `bearer_token` | Blocking file lock inside `spawn_blocking`; lock protects credential load/refresh/save | Test real shared-auth contention/cancellation; dropping an async wait does not preempt a blocking acquisition |
| [observability](../../src/observability.rs) | Schema-1 metadata allowlist, bounded fallible writer, explicit coverage | Separate batch trace family and shared quota accounting, never raw event/transcript dumps |
| [session storage](../../src/session_store.rs) | Private atomic idle snapshots, v1 load/v2 publication, exclusive writer | No batch execution state or worker history auto-save; preserve existing schema and restart behavior |
| [workspace evidence](../../src/changes.rs) | Invocation snapshots observe bounded workspace changes | Parallel reads share a live folder; no filesystem snapshot/isolation or causal attribution guarantee |

The authentication concern is source-supported, not a reproduced Phase 8 failure:
this planning task did not execute a contention experiment. It is an explicit
implementation prerequisite, with a deterministic held-lock scenario before any
joined-cleanup claim. Provider rate limits/server scheduling likewise cannot be
inferred from Rust Send + Sync. Existing shell cleanup/permission regression evidence
remains useful but does not prove worker lifecycle correctness.

## Grilling outcomes and proposed decisions

- Duplicate/stale worker completion: one coordinator owns immutable task assignment,
  terminal commits and slot release; no retry/lease semantics to infer.
- One task fails while another finishes: preserve the successful result, skip only
  dependent work, and make aggregate failure explicit. No silent all-or-nothing loss.
- Child cancellation could cancel a sibling: separate tokens, parent-owned propagation,
  terminal immutability and barrier-based race tests.
- Dependency text asks for shell or reveals a sibling's secret: declared-edge data
  transfer only, no raw sessions, hard tool-time read-only cap. Delimiters are not a
  security sandbox and no content-based prompt-injection protection is promised.
- Two workers disagree: retain source envelopes; synthesis is separately unverified,
  not a deterministic conflict detector or automatic merge.
- External stream/writer stalls: independent child drains, explicitly fallible new
  batch projection and optional bounded metadata writer; no false completeness.
- Process crashes: idle session v2 and inspection traces do not recover active DAGs;
  no durable scheduler claim.

These are concrete proposals in ADRs
[0017](../adr/0017-bounded-task-dag.md),
[0018](../adr/0018-worker-authority-and-results.md) and
[0019](../adr/0019-batch-events-and-traces.md). All remain Proposed, implementation
Not started. Planning recommendations do not resolve maintainer decisions.

## Six retrospective questions

What did we learn? Existing run/context/permission primitives are useful building
blocks, but execution completion, answer quality and persistence are different facts.

Which assumptions were wrong? Original private history was not safe to resume across
models merely because it was retained; ADR 0016 now addresses that debt. Likewise,
sharing Cancellation clones or multiplexing an attached stream does not automatically
produce independent worker scopes or nonblocking cleanup.

Which abstractions proved useful? Owned RunResult/Session, checked events, pure
context preflight, explicit policy, metadata projection and versioned idle storage.

Which abstractions are premature? Distributed leases, task retries, dynamic planning,
Astrix/groups, universal provider capability registries, scheduler databases and
external coding-agent harness adapters.

What debt would distort Phase 8? Auth-lock wait lifetime and event-drain coupling need
concrete tests. Narrow quality samples and approximate context must not justify
automatic routing, cost claims or aggregate window displays. Missing verified token
capacity is a labelled limitation, not a blocker for a fixed read-only scheduler.

What should not be carried forward? Mixed private worker conversation, shared stdin
approvals, general shell described as read-only, unverified model synthesis treated
as reconciliation proof, or idle snapshots advertised as active-job recovery.

## Readiness disposition

Ready for maintainer review of the plan and three proposed contracts. Hold scheduler
implementation until those contracts and explicit transition authorization are
resolved; Phase 7 closure disposition remains separately recorded. Implementation
tickets are Draft. The first proposed slice is two headless read-only workers with
barrier-proven overlap and owned results, followed by a dependent synthesis node.

This task changes documentation only. Validate document navigation, proposal/gate
consistency, scope and whitespace; do not rerun or invent runtime tests for these
plans. Existing 218/0/1 is the previous baseline. No new live runs, providers,
infrastructure, source changes, staging/commit, new branch or worktree. Unrelated
files and recovery data are retained. M02 Paused/unaccepted; Phases 5/13 Deferred.

Planning checks passed: 385 local Markdown file targets, new-document whitespace,
`git diff --check`, proposal/ticket gate consistency and unchanged Phase 7 source
hashes. The plan also links back to CURRENT after this check; navigation was
rechecked at completion. No new runtime execution evidence is claimed.
