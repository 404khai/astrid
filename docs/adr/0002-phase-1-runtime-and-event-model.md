# ADR 0002: Phase 1 runtime and event model

Date: 2026-10-06

Status: Accepted by the maintainer; frozen for Phase 1. Implemented.

## Context and Phase 0 review

Phase 0 demonstrated the real subscription-backed model/tool loop, native
tools, explicit shell approval, and successful repository repair. Live provider
streams disproved assumptions about mandatory Content-Type headers and populated
terminal output arrays; the adapter now handles both cases without weakening
the completion barrier. Authentication and provider boundaries proved useful.
Universal provider abstractions and multiple crates remain premature.

The existing agent owns conversational state and sequential dispatch through
local variables. Borrowed Progress notifications serve the CLI, observer errors
abort execution, and tools coordinate shell approval. ToolStarted currently
includes permission waiting. General cancellation and Astrid-owned execution
identities are absent. These behaviors must not define Phase 1 semantics.

This ADR records the maintainer's decisions, supplementing the Phase 0 contract.
The implementation follows these decisions; transport and cleanup behavior are
documented in architecture.md.

## Session, run, turn, and identity

- A Session owns conversational state.
- A Run executes one submitted user task.
- A Turn contains one model invocation followed by its validated tool batch.
  A final response without tools is also a turn.
- Phase 1 creates one fresh, ephemeral session per run. No interactive follow-up
  or session persistence is introduced. The domain distinction permits multiple
  runs per session later.
- Introduce typed SessionId, RunId, TurnId, ModelCallId, and ToolCallId.
- Provider-issued tool IDs remain separate protocol metadata.
- Allocate ToolCallId when a validated completed model response contains the
  accepted request, including requests subsequently denied or cancelled.
  Incomplete streamed fragments have no runtime tool identity.
- Events carry all applicable parent IDs and a monotonically increasing
  per-run sequence number. Timestamps do not establish ordering.

## Authoritative state and transitions

The runtime owns typed authoritative execution state. One controlled transition
boundary validates a transition, updates state, and emits its corresponding
event. Events permit independent reconstruction of relevant state; replay is
not the runtime's execution mechanism. Do not implement event sourcing.

## Completion and terminal outcomes

ModelCallCompleted means a complete response was received and validated, not
that its contents are correct. ToolCallCompleted means an operation executed
and produced an execution outcome. A shell exit code of 1 or 101 can therefore
belong to a completed tool call. RunCompleted means normal agent-loop
termination, not independently established task success.

Use distinct terminal event variants for Completed, Failed, Denied, Cancelled,
TimedOut, and Skipped where relevant. Do not overload Completed with an outcome
that changes its meaning. Timeout is a failure condition; cancellation is a
user/runtime request. Task success remains unknown without explicit supporting
verification. A green CLI completion message says "Run completed", rather than
claiming the task was solved without evidence.

## Tool and permission lifecycle

After accepting a validated model response, emit every ToolCallRequested in
its batch immediately. Execute the batch sequentially.

For an approved operation:

```text
ToolCallRequested
PermissionRequested
PermissionGranted
ToolCallStarted
ToolCallCompleted
```

For a denied operation:

```text
ToolCallRequested
PermissionRequested
PermissionDenied
ToolCallDenied
```

ToolCallStarted means actual execution began. Permission waiting is not
execution. Tools may declare permission requirements; the runtime coordinates
requesting and receiving decisions. Tools do not own UI behavior.

Every validated requested tool receives exactly one terminal runtime outcome,
including tools that never start.

## Provisional output and event payloads

ModelTextDelta events are provisional. ModelFirstTextDelta may identify the
first visible text; do not call it FirstToken. Only successful validated
completion commits assistant text, tool requests, and provider continuation
metadata to conversation state. Interrupted text stays visible, marked with a
lightweight interruption indicator, and remains uncommitted.

Do not expose provisional tool-argument fragments as public runtime events.
Provider adapters process them internally.

Events are owned and serializable. Include lifecycle metadata, IDs, sequence
numbers, visible text deltas, committed assistant output, validated tool names
and arguments, permission lifecycle, structured outcomes, and terminal outcomes.
Consumers can reconstruct lifecycle, ordering, visible transcript, tool
requests, and outcomes. Exact provider request reproduction is not required.

Exclude credentials, API keys, private tokens, and opaque provider session
secrets. Provider continuation stays internal. Allow future storage indirection
for large values without introducing a blob store or persisted traces.

## Event transport and runtime lifetime

Use a bounded, ordered, lossless channel. Slow consumers may apply backpressure.
Runtime lifetime is independent of UI lifetime. Consumer disappearance does
not universally cancel execution. A headless runtime can continue, and its
terminal RunResult is obtainable independently of successful event rendering.

The CLI may request cancellation on unrecoverable output failure, such as a
broken pipe. That is application policy, not a universal runtime rule. UI
failure cannot retroactively change whether an operation succeeded.

## Cancellation scope and races

A public cancellation handle supports both library callers and CLI Ctrl-C.
Cancellation covers model/authentication waiting where the underlying operation
permits it, streaming, permission waiting, shell execution, and boundaries
between sequential tools.

Short synchronous file operations may complete atomically before cancellation
is acknowledged. Do not implement rollback. Ctrl-C interrupts an active shell
without waiting for timeout. Terminate the relevant process/process group and
wait for cleanup; dropping a future alone is insufficient.

Check cancellation before dispatching each new operation. After cancellation
acknowledgement, start no new work. Terminal events describe what actually
happened to operations already started. A committed edit remains Completed
even if the run subsequently becomes Cancelled.

If cancellation cleanup cannot establish that active external work stopped,
terminate with RunFailed identifying cancellation cleanup failure. Keep the
cancellation request visible in event history; do not claim clean cancellation.

The operation at which cancellation is actively acknowledged receives
ToolCallCancelled. Later requested operations that never begin receive
ToolCallSkipped with reason RunCancelled. Every request still has exactly one
terminal runtime outcome. Do not fabricate conversation-level results for
skipped operations or call the model merely to explain cancellation.

Recoverable tool failures during an active run still become tool results and
return to the model. Cancellation terminates the loop.

## Model-call ceiling

Preserve Phase 0 behavior: the ceiling limits model invocations, not tool
executions. Execute a validated batch returned by the final allowed invocation,
record its outcomes, and terminate with ModelCallLimitReached if another model
invocation would be needed. Explicitly expose that those final tool outcomes
were not subsequently inspected by the model. Do not claim normal completion.
Additional budget types and reserved finalization calls remain deferred.

## Dependency direction and scope

The CLI consumes the public Runtime API. Runtime execution separates agent
policy, events, model behavior, and tool execution. The CLI can render events,
request cancellation, and provide permission decisions. It does not own loop
state, tool lifecycle, completion semantics, or cancellation semantics.

Retain one Cargo package, one provider, sequential tools, ephemeral sessions,
and one task per invocation. Do not add persisted traces, subagents, routing,
worktrees, MCP, or context-engine machinery.

## Acceptance and core invariants

Deterministic tests must demonstrate:

- Legal runtime-state transitions and independently reconstructed state.
- Ordered, correlated events with increasing per-run sequence numbers.
- Exactly one terminal outcome per accepted run and validated requested tool.
- Provisional streamed text remaining uncommitted after interruption.
- No side effects or tool dispatch from incomplete or unsuccessful responses.
- Cancellation during streaming, permission waiting, shell execution, and
  between sequential tools; cancellation during other waiting where supported.
- No new work after cancellation acknowledgement.
- Accurate outcomes for side effects completed before cancellation.
- Correct shell cleanup and correct reporting of cleanup failure.
- Permission waiting distinguished from execution.
- Provider identity distinguished from Astrid runtime identity.
- UI failure never retroactively altering execution outcomes.
- A headless consumer using the same runtime without terminal behavior.
- A CLI consuming only public runtime/event interfaces for execution.
- Events describing behavior without becoming authoritative runtime state.
- The ceiling preserving execution of the final validated tool batch and
  reporting that its results were not subsequently inspected by the model.

## Change procedure

These decisions are frozen for Phase 1. Record implementation evidence, any
materially changed decision, and its rationale before proceeding. Significant
unresolved questions follow AGENTS.md's decision-question procedure.

## Implementation evidence

Cancelling a provider future during backpressured text delivery can interrupt
an event after state was updated. The runtime retains a small pending event batch across
that cancellation and flushes it before another transition. A first-text marker
and its delta are accepted together before awaiting delivery, so cancellation
cannot leave a first-text marker without its actual text. This preserves the
frozen lossless ordering rule without introducing persistence.

On macOS, readiness registration of /dev/tty failed with EINVAL during live
acceptance. The CLI uses nonblocking terminal reads and cancellable timer waits
instead, with no blocking reader thread. Permission requests are relayed to the
CLI input task after their ordered request event is rendered. This changes a
reversible input implementation, not runtime permission or cancellation policy.

Validation: 52 deterministic tests pass, including the preserved Phase 0
protocol/fixture tests and new lifecycle, independent state projection,
cancellation, input, and cleanup tests. Formatting, Clippy with warnings denied,
and the debug build pass. Live gpt-5.6-luna fixture acceptance completed in five
model calls and nine tool requests: initial tests failed, the existing function
was edited, GREETING.txt was created, separately confirmed tests passed, and
the run terminated normally. A separate live Ctrl-C check cancelled an active
shell and verified that its leader had been reaped.

## Phase 2 amendments (2026-10-07)

The historical phase contract above is preserved. Accepted
[ADR 0003](0003-bounded-tool-output.md) extends tool-output admission and
cancellation payloads; [ADR 0004](0004-execution-authority.md) extends fixed
permissions to explicit per-run policies with the same defaults;
[ADR 0005](0005-workspace-change-evidence.md) adds bounded workspace evidence
without changing invocation-directory scope. Final bounded observation after
cancellation is bookkeeping, not permission to dispatch new task work.
