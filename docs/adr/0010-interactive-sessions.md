# ADR 0010: Interactive in-memory sessions

Date: 2026-10-08
Status: Accepted; ephemeral-storage restriction superseded by [ADR 0012](0012-local-session-persistence.md) on 2026-10-09
Implementation: Implemented; deterministic validation passed
Related: ADR 0002 session/run lifecycle; ADRs 0006 and 0007 context contracts
Decision evidence: Maintainer requested `/sessions`, then explicitly requested
continued conversation after runs and an AGENTS.md/ADR update on 2026-10-08.
The initial ephemeral scope resolves storage locally without introducing a
persisted-state contract.

## Context

The interactive CLI currently accepts one task, executes one run, and exits.
`runtime::run` creates a fresh SessionId and conversational state for every task.
There is no session registry or API for submitting a follow-up to an existing
session. ADR 0002 explicitly establishes fresh ephemeral sessions per run and
excludes interactive follow-ups and persistence.

The maintainer requested a `/sessions` command to list current sessions and
switch between them. A menu alone cannot deliver that behavior. Session
ownership and multiple runs per session must become explicit runtime contracts.

## Options

1. In-memory sessions in one running Astrid process. Supports follow-ups and
   switching without introducing disk formats or recovery semantics. Sessions
   disappear when the process exits.
2. Persistent sessions restored across launches. Requires a separate storage,
   privacy, compatibility, provider-continuation, and recovery contract. Metadata-only traces would be insufficient for resumption.

## Decision

Implement option 1 as a bounded extension of the current Phase 3 scope. Replace
only ADR 0002's restriction to one fresh session per run and its exclusion of
interactive follow-ups. Preserve ephemeral storage, sequential execution,
permission policies and event ordering. Extend the context contracts for multiple
user submissions as described below.

Concrete client behavior:

- Bare `astrid` returns to the composer after a run.
- `/new` creates and selects a fresh session in the current workspace.
- `/sessions` lists sessions with ID, first-task label, active marker, run count,
  and latest outcome; selecting an entry changes the active session.
- Subsequent tasks use the selected session's conversation and context ledger.
- Switching occurs only between runs. It starts no tools or model requests.
- `astrid run` retains its existing one-shot behavior.
- The UI states that sessions last only until Astrid exits.

The runtime owns conversational sessions and provides a headless follow-up API.
The CLI owns menus and active selection. Each session remains associated with
its workspace; switching does not change permission policy or grant authority.
Each task allocates a new RunId while preserving the session's SessionId.

## Consequences

The initial CLI retains at most 32 sessions in memory; each session's retained
history is not globally memory-bounded, matching the existing history limitation.
Request admission remains bounded. No disk persistence, background execution,
parallel runs, checkpoint recovery, or phase advancement is introduced.

Cancelled/failed runs retain committed state and their terminal outcome.
Continuation must respect the completion barrier and whole-exchange selection:
never replay a tool side effect, commit interrupted assistant output, or submit
an unmatched tool request/result sequence. If a terminal session cannot safely
continue, report that limitation explicitly and offer a fresh session rather
than silently discarding state. `run_in_session` validates history and workspace before accepting a submission.
An unmatched terminal batch rejects continuation and returns the unchanged owned
session in `SessionStartError`; the CLI recommends `/new`. A cancelled model call
without a committed tool batch can continue. No synthetic outcomes are added.

## Compatibility

Keep the existing fresh-session `runtime::run` entry point and one-shot CLI
behavior. Add a separate session-aware entry point. Existing lifecycle and
permission events remain per run with the correct stable session parent ID.
`ContextInherited` initializes each follow-up run's reconstructed context ledger
with prior provenance, without inventing current-run model/tool lifecycles or
claiming the prior messages were newly added. New context IDs continue the
session ledger. Operating/repository sources are not repeatedly added, while
repository instructions are re-read for each run and policy is emitted per run.

Extend ADR 0006's task protection to protect the original task, current user
submission, and latest complete exchange. Earlier follow-up submissions and
all their response exchanges are selected/omitted together, preventing historical
answers from losing their associated question. Within the current submission,
whole assistant/tool exchanges remain the selection unit. Ranking uses the
current task's file signals. ADR 0007's incomplete summaries may include bounded
historical user excerpts; provider-private data remains excluded.

There is no persisted-state migration.

## Resolved scope

`/sessions` initially covers the current Astrid process with follow-up runs;
restoration after restart requires a separate contract.

## Validation

Deterministic tests must establish:

- Two runs in one session share SessionId and have distinct RunIds.
- A follow-up receives prior committed conversation under the context budget.
- Switching between sessions does not mix messages, context, or workspace state.
- Switching alone dispatches no model/tool work and changes no authority.
- Cancellation/failure preserves committed edits without replaying side effects
  or sending invalid provider history on a later task.
- Closing the client does not introduce a core runtime lifetime dependency.
- One-shot execution and existing runtime invariants remain valid.

## Resolution

Accepted as the scoped implementation of the maintainer's 2026-10-08 request.
Cost of changing later: Medium for the session lifecycle API; persistence would
require an additional, higher-cost contract. No later-phase authorization.

Deterministic evidence: `tests/sessions.rs`, context-selection fixtures in
`src/context.rs`, and the conversation/switching PTY scenario in
`src/console/pty_tests.rs`. The one-shot runtime API remains covered by the existing
runtime/environment/context suites.
