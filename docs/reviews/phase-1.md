# Phase 1 review: runtime model

Date: 2026-10-06
Baseline: `4b8e93f`
Status: Prepared from repository inspection and local validation
Maintainer closure decision: Closed by maintainer report and subsequent request
to implement Phase 2, 2026-10-07
Next-phase authorization: Phase 2 explicitly requested on 2026-10-07

## Exit criteria and evidence

| Criterion | Implementation | Evidence | Assessment |
| --- | --- | --- | --- |
| Explicit session/run/turn/model/tool state | [runtime.rs](../../src/runtime.rs), [events.rs](../../src/events.rs) | `check_history`, illegal-transition and wrong-parent tests in [runtime tests](../../tests/runtime.rs) | Implemented; ephemeral session per run |
| Correlated, ordered execution events | Checked transitions and per-run sequence numbers | Batch-order, independent history reconstruction, backpressured cancellation tests | Implemented; in-memory transport only |
| Cancellation and honest outcomes | [cancellation.rs](../../src/cancellation.rs), runtime and shell cleanup | Model/permission/shell cancellation, committed mutation, cleanup failure tests | Implemented; callers must continue polling to terminal result |
| Separate runtime, policy, model, tools | [agent.rs](../../src/agent.rs), [model.rs](../../src/model.rs), [tools.rs](../../src/tools.rs) | Headless/disconnected consumer tests and source inspection | Implemented within one package |
| CLI consumes runtime | [main.rs](../../src/main.rs), [console.rs](../../src/console.rs) | UI failure after committed edit; terminal input tests | Implemented; presentation state remains in CLI |

Assessment: current evidence supports closing Phase 1, with the documented limits
below. This is a review recommendation, not an automatic phase transition.

## What we learned

**What did we learn?** Completion is several different facts: validated model
response, executed tool, normal run termination, and verified task success.
The runtime now represents the first three without inventing the fourth.
Permission waiting also needs its own lifecycle, before actual tool execution.

**Which assumptions were wrong?** Dropping an in-progress event send during
cancellation could leave state ahead of delivered events. The pending-event pair
preserves ordering. The Phase 1 ADR also records that macOS `/dev/tty` readiness
registration failed during live testing, requiring cancellable nonblocking reads.
These are recorded implementation findings, not new incidents observed today.

**Which abstractions proved useful?** Typed IDs and checked transitions make
independent history validation possible. PermissionHandler separates input from
execution. ModelProvider and ToolExecutor provide deterministic test seams.
Explicit cleanup outcomes prevent falsely reporting successful cancellation.

**Which abstractions are premature?** A universal provider capability system,
extra crates, persisted event replay, scheduler/worker concepts, and resumable
sessions still lack a requirement in the next execution-environment slice.

**What debt would distort Phase 2?** Shell pipes currently use unbounded
`read_to_end`; read/search results can also grow without a bound. PermissionRequest
is shaped only around shell commands. There is no Git baseline or change-report
contract. Lossless event delivery may delay cancellation acknowledgement while
an attached consumer is stalled. These need explicit decisions before extending
the event stream with shell output.

**What should not be carried forward?** Do not treat cwd as shell confinement,
confirmation as a sandbox, branch diff as proof of run attribution, or a timeout's
missing output as empty output. Do not reproduce timeouts/cancellation/path checks
merely because they also appear in the Phase 2 roadmap.

## Phase 2 inventory

| Area | Already exists | Actual remaining work |
| --- | --- | --- |
| Shell lifecycle | Timeout, cancellation, process group cleanup, exit status, separate pipes | Bounded streaming, partial-output semantics, overload behavior |
| Workspace | Root-relative checks; symlink/traversal rejection; atomic edits | Contract review and adversarial regression coverage; no sandbox claim |
| Permissions | Per-shell approval, denial, cancellation, no-terminal rejection | Configurable policy and observable decision reasons |
| Git and changes | No dedicated integration found | Baseline state, branch/dirty visibility, bounded change reporting |

## Validation record

Rerun locally on macOS at the baseline revision on 2026-10-06:

- `cargo test --locked`: 57 passed, 0 failed (9 library, 7 binary, 15 loop,
  18 runtime, 8 tool tests).
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.

[ADR 0002](../adr/0002-phase-1-runtime-and-event-model.md) records an earlier
52-test baseline and live fixture/active-shell cancellation acceptance. Those
live checks were not repeated for this planning change; the newer test count
does not invalidate that historical record.

## Remaining decisions

- Can process cleanup be held up by a stalled event consumer? Resolve in ADR 0003.
- Is the next safety contract trusted local repositories with explicit shell
  authority, or hostile-code containment? Resolve in ADR 0004.
- Which changes can we attribute, and which can we only observe? Resolve in ADR 0005.

See [the Phase 2 plan](../plans/phase-2.md) for ordered work and acceptance.
