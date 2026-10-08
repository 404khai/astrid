# Interactive sessions and permission modes — implementation review

Date: 2026-10-08
Scope: maintainer-requested Phase 3 extension; ADRs 0010 and 0011
Review: solo review; no phase closure or advancement

## Behavior

Bare `astrid` returns to the composer after terminal run outcomes. Each submitted
message starts a new run in the selected workspace-bound session. `/new` creates
a session and `/sessions` selects among up to 32 in-memory sessions, with stable
IDs, first-task labels, counts, active marker, and latest outcomes. One-shot runs
remain available. No old tool call is rerun on selection or continuation.

`/mode` and `/mode ask|auto|unbound` configure the next run through existing
permission policies. Auto preserves prior defaults. Ask requires write/shell
approval. Unbound allows all supported capabilities without prompting, retaining
native validation and execution bounds. Mode is visible in metadata and input/run
status; the unbound logo uses the requested red arch bands and yellow eyes.
Per-capability one-shot flags override presets; unmatched policies display custom.

## Runtime and context review

Session-aware execution is usable headlessly and transfers owned sessions.
Rejected configuration, mismatched workspace, missing provenance, or incomplete
tool history returns unchanged ownership before dispatch. SessionId persists;
RunId, per-run event sequence, execution state, and cancellation are fresh.

`ContextInherited` supplies prior provenance for reconstructing a follow-up run.
New context item IDs and history indices remain session-wide. Selection protects
the original/current tasks and latest complete exchange; older submissions stay
with their answers. Relevance uses the current task. Incomplete deterministic
summaries may include bounded historical user excerpts. Replay rejects a split
historical submission without partially changing state.

Runtime checks reject reused provider tool IDs across runs before side effects.
Cancelled batches with unmatched requests remain inspectable but cannot continue;
the CLI directs the user to a fresh session. Interrupted model output remains
uncommitted. Permission policy is snapshotted per run; session switching cannot
restore or elevate it.

## Validation

Final shared-workspace validation: 146 tests passed across 15 suites;
formatting, Clippy with warnings denied, diff checks, and local documentation
links passed. `cargo install --path . --locked` rebuilt and replaced the installed
CLI; its help exposes the named modes. An earlier concurrent observability test
failure was resolved before this final pass. No live inference was performed.

- `cargo test --locked`: runtime, permissions, context, native tools, event replay,
  session follow-ups, pruning, cancellation, and real PTY interaction.
- `cargo fmt --check` and `cargo clippy --locked --all-targets -- -D warnings`.
- `git diff --check` and documentation-link checks.
- The PTY scenario submits consecutive messages, changes mode, creates another
  session, switches back, and verifies the original conversation continues.
- Palette tests check exact RGB values, plain rendering, and effective mode.
- No live provider or account authentication is needed for acceptance fixtures.

Existing working-tree changes and concurrent observability work are outside this
review's implementation scope. Validation uses the shared workspace; any
concurrent-suite failure is reported separately until resolved.

## Limitations and debt

Sessions disappear when the process exits. The client session count is bounded;
total retained conversation memory remains unbounded, as in the preceding context
slice. Protecting an entire previous submission can cause request admission to
fail when it exceeds the allowance. There is no checkpoint recovery for incomplete
tool batches, no parallel execution, no transcript redraw on switching, and no
persisted transcript contract. Model and mode selections apply to future runs
across the current client's sessions; repository instructions cannot change mode.

The session registry is client selection/metadata, while the runtime owns each
conversation and its execution. Future clients can use the same session-aware
entry point without terminal dependencies.


## Isolated PR validation

On 2026-10-08 the session/mode/UI changes were extracted into
`feat/interactive-sessions-modes`, based on `8d765c2`, without the observability
module, fields, commands, notices, or ADR 0008/0009 implementation. The isolated
checkout passed 139 tests across 14 suites, `cargo fmt --check`,
`cargo clippy --locked --all-targets -- -D warnings`, and diff/link checks.
The completed inline terminal migration is included as the client prerequisite.
Observability is a separate stacked change based on this branch. The original
shared checkout and unrelated fixture/research changes remain untouched.
