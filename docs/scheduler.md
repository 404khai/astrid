# Read-only task batches

Astrid can execute a caller-authored static DAG in one process. Each task owns one
fresh session/run/context, cancellation handle and noninteractive permission
handler. This is direct model inference inside Astrid's harness. A task can make
several model/tool calls; a model invocation is not itself a worker.

```sh
astrid batch run docs/evaluations/phase-8-notes-tasks.json \
  --folder tests/fixtures/personal-tasks/outstanding --concurrency 2
astrid batch run docs/evaluations/phase-8-code-tasks.json \
  --folder tests/fixtures/greeting --observability on --json
astrid batch trace
astrid batch trace <batch-id>
```

The examples explicitly configure models/effort; replace those values with supported
account models as needed. There is no implicit model selection or extra planning
inference. The existing observability setting applies unless overridden. Running a
batch uses authorized inference. The notes/code examples are synthetic fixtures.

A minimal manifest:

```json
{
  "schema": 1,
  "tasks": [
    {"key": "source", "instruction": "Inspect source behavior", "model": "configured-model"},
    {"key": "tests", "instruction": "Inspect test declarations; do not execute them", "model": "configured-model"},
    {"key": "review", "instruction": "Reconcile attributed findings, preserving uncertainty", "model": "configured-model", "depends_on": ["source", "tests"]}
  ]
}
```

The manifest is a bounded regular file read once; later edits do not change that
invocation. Duplicate keys/edges, unknown/self dependencies, cycles, invalid model
routes/effort and aggregate call reservations reject before authentication or task
tool dispatch. Unknown manifest fields and unsupported schemas reject explicitly.
`routing` accepts the existing direct-inference policy, `context_budget` the existing
per-request budget, and `permissions` a narrower requested policy. Recording is one
invocation-level policy/store, shared by every child; tasks cannot choose another
recording allowance.

Defaults: concurrency 2, 8 reserved calls per task, batch ceiling 64 calls, task
120 seconds and batch 300 seconds. Task and batch ceilings are explicit manifest
fields `max_model_calls` and `timeout_seconds`. Hard admission limits are 32 tasks,
128 edges, concurrency 8, 1024 total reserved calls, 1 MiB manifest, 64 KiB rendered
task input/final transferable text, and 24 hours for either deadline. Reservations
include synthesis tasks and cannot be borrowed. For example, 32 tasks at the
8-call default exceed the default batch ceiling and reject. Byte/context limits
are independent; oversize inputs/results fail rather than silently truncate.

All workers, including reconciliation, have read-only tool authority. Deny dominates
Ask dominates Allow when intersecting parent/task grants with that ceiling. Write,
edit and shell cannot dispatch under an unbound parent either. `--read-policy ask`
has no worker approval broker and denies reads without stdin. Native path/symlink
checks still apply. Authentication, optional metadata recording and workspace
evidence retain the existing runtime authority. The selected live folder is not
a filesystem snapshot or OS sandbox.

Dependencies are all-required. Readiness uses manifest order; inputs use declared
edge order. Only bounded committed visible final text travels over declared edges,
with source batch/task/run identities, exact bytes/digest and publication validation.
`bounded_committed_final` validates transfer provenance/bounds, not factual accuracy.
Private continuation, parent/sibling conversation, provisional text and tool requests
are not transferred. The original user task protects mandatory inputs during
context selection: they remain present or request admission fails explicitly.

The report preserves original source results/statuses in manifest order. A declared
synthesis is another unverified task and cannot replace sources, erase contradictions
or make failed work successful. A completed child whose final text exceeds bounds
keeps its actual child completion fact but fails task publication. Failed/cancelled
predecessors skip descendants; independent branches continue. Failure takes precedence
over cancellation in the batch outcome. Execution completion is not a quality grade.

Ctrl-C stops admission, signals active children and joins owned work. Headless
`BatchControl::cancel_task` affects only that task/dependent branch. Deadlines have
separate reasons. Terminal facts are immutable; cancellation is cooperative and
can outlast a deadline during cleanup. It does not prove remote billing stopped.
CLI rendering begins after join, so blocked stdout cannot hold active workers open.
The JSON report includes the complete bounded source bundle/lifecycle, explicit
recording coverage, invocation-only writer diagnostics and measured wall/cleanup/finalization durations. It leaves
usage/cost/model-window occupancy unavailable rather than summing fictional window
percentages. Child traces retain available per-call usage separately. The CLI does
not display live worker context meters or full transcripts.

The headless API is [scheduler::run](../src/scheduler.rs), with [manifest parsing](../src/batch_manifest.rs)
when useful. Await a batch through cleanup after requesting cancellation; dropping
its future is not a joined cancellation operation. Its optional bounded lifecycle
projection detaches on saturation/disconnection and reports that coverage in
`BatchResult`; a closed subscriber is not cancellation. Internal child event drains
continue independently. `BatchResult.lifecycle` retains the bounded complete log.
Existing attached single-run streams keep their original backpressure semantics.

Optional batch metadata uses private `traces/batch-<UUID>.jsonl` files (or `.partial`
when incomplete), separate from unchanged run trace schema 1 and idle session v2.
Both trace families share total byte/file quotas and writer exclusion. Records
contain generated IDs/topology, policy actions, lifecycle/categorical outcomes,
numeric counts/budgets/lengths and child recording coverage. They exclude task
labels/text, file/manifest paths, content digests, arbitrary errors, credentials and
private continuation. Batch completeness does not establish child completeness or
answer correctness. Run statistics exclude batch aggregates, counting child usage
once. Writer failure/saturation affects recording, not execution; nothing is
removed automatically.

Original results live in memory and explicit command output, not idle sessions or
metadata traces. Process exit can lose them. Trace inspection does not recover a
job. There is no dynamic spawning, retry, lease, durable queue, remote worker,
concurrent mutation/shell, worktree, daemon or desktop integration in this feature.
See the [accepted contracts and validation plan](plans/phase-8.md) and
[implementation review](reviews/phase-8.md).
