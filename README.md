# Astrid

Development planning: [phase review and delivery process](docs/development.md),
[Phase 1 review](docs/reviews/phase-1.md), and
[Phase 2 tickets](docs/plans/phase-2.md).

For hands-on checks, see [Try Phase 2](docs/testing-phase-2.md).

An experimental Rust agent harness. Phase 2 provides a typed execution runtime
with ordered events, cancellation, bounded tool output, configurable permissions,
and workspace change evidence for one fresh repository task per process.
It streams OpenAI output and executes native tools sequentially.
macOS is the supported platform.

## Build and sign in

```sh
cargo build
cargo install --path .
astrid login
astrid models
astrid run "find and fix the failing test" --model <model-slug>
```

`astrid login` prints a **Continue with ChatGPT** link. Open it in your browser
on the same Mac and authorize Astrid's access to your ChatGPT plan. The callback
listener expires after five minutes. Astrid validates the returned identity and
plan permissions before saving credentials.

Host identifiers use persisted UUIDv4 URIs (`urn:uuid:...`). If an initial
development attempt saved the rejected `astrid-...` format, the next login
repairs it automatically before registration. Registered host identities are
never silently replaced. Generate a fresh link with `astrid login`; old links
belong to an expired or stopped callback listener.

Authentication follows the documented [open-source Sign in with ChatGPT flow](https://developers.openai.com/siwc/token-sharing-open-source/sign-in).
Astrid owns its registration and credentials; it never reads Codex credentials.
Credentials and a stable host identifier live under `~/.config/astrid/`, with
owner-only permissions. Refreshes are serialized across processes and rotating
credentials are saved atomically. Conversation history is only held in memory.
This release implements subscription authentication; API-key authentication is
not a prerequisite and is not automatically selected as a fallback.

`astrid models` queries the selected account's subscription model catalog.
The model must be supplied with `--model` or `ASTRID_MODEL`; there is no default
model in the architecture. Account entitlement requires successful inference,
not merely a model appearing in a catalog. ChatGPT plan limits still apply.

## Run configuration

| Setting                | CLI                 | Environment              | Default  |
| ---------------------- | ------------------- | ------------------------ | -------- |
| Model                  | `--model`           | `ASTRID_MODEL`           | Required |
| Model-call ceiling     | `--max-model-calls` | `ASTRID_MAX_MODEL_CALLS` | 20       |
| Shell timeout, seconds | `--shell-timeout`   | `ASTRID_SHELL_TIMEOUT`   | 30       |
| Native read policy     | `--read-policy`     | —                        | allow    |
| Native write policy    | `--write-policy`    | —                        | allow    |
| Shell policy           | `--shell-policy`    | —                        | ask      |

The invocation directory is the workspace root. Astrid reads only that
directory's `AGENTS.md` as initial repository instructions; parent and nested
instruction discovery is deferred.

On an interactive terminal, `astrid run` shows Astrid's existing pixel logo,
compact model/provider/workspace metadata, and a scrolling execution stream.
The header and three-line footer stay in place. The footer displays the actual
run ID, turn, model-call count, and current activity; the bottom prompt accepts
operation approvals when requested. Tasks still start through `astrid run`; there
is no interactive conversation composer or mode switch. Ctrl-C cancels the run.
Tool requests show their target, results show a concise summary, and shell
stdout/stderr stream separately while the command runs. Effective permission
policy, initial branch/dirty paths, native mutation patches, and final observed
workspace changes are visible in the execution stream. Completion means
the run ended normally, not that the task was independently verified.

The logo keeps its blue arches and cyan eyes, with cyan reserved for identity
and important status. Labels and secondary information use dim terminal text.
Set `NO_COLOR=1` to disable all styling colors. Narrow/short terminals use a
compact header, truncate metadata by display width, and wrap execution output;
the logo is omitted only when it would crowd out the stream. Resize redraws use
up to 2,000 retained display lines. Resizing during a permission answer is
applied after the decision to avoid erasing terminal-echoed input. An approval
that cannot fit in the viewport switches the run to append-only output so the
complete command remains reviewable in ordinary terminal scrollback.

When either output stream is redirected, or `TERM=dumb`/terminal sizing is
unavailable, rendering falls back to ordinary append-only text without cursor
controls or colors. Assistant text continues to stream to stdout; concise tool
activity, command output, and failures go to stderr. Operation approvals use
`/dev/tty`, including the complete command and workspace, independently of
redirected output. No telemetry, context budgets, or unsupported shortcuts are
shown.

Every successful response without tool calls ends the task. Recoverable tool
failures return to the model. Provider/protocol errors and an exhausted
model-call ceiling exit nonzero. Astrid does not automatically retry mutations
or provider requests. Its final answer is a model claim, not proof of success.

## Native tools

| Tool             | Arguments                      | Behavior                                                  |
| ---------------- | ------------------------------ | --------------------------------------------------------- |
| `read_file`      | `path`                         | Read a UTF-8 file                                         |
| `write_file`     | `path`, `content`, `overwrite` | Atomic creation or explicit overwrite; parent must exist  |
| `edit_file`      | `path`, `old_text`, `new_text` | Exact replacement only when there is one match            |
| `list_directory` | `path`                         | Sorted immediate entries                                  |
| `glob`           | `pattern`                      | Sorted workspace-relative file matches                    |
| `grep`           | `path`, `pattern`              | Rust regex search with filenames and 1-based line numbers |
| `shell`          | `command`                      | Policy-controlled noninteractive `/bin/sh -c` execution |

File tools automatically operate within the workspace. Traversal, outside
paths, symlink paths, and mutations to hard-linked files are denied. Search
skips `.git`, symlinks, and binary files for textual grep. Existing file
permissions survive edits. Empty or ambiguous edit targets cause no mutation.

Permission policies are `allow`, `ask`, or `deny` for native reads, native
writes, and shell execution. Defaults allow validated workspace file operations
and ask for each shell call. Policy decisions and reasons are runtime events.
The CLI asks for an explicit `yes` through `/dev/tty`; missing input/EOF denies.
Repository files cannot configure or elevate authority. Policies govern tool
dispatch; runtime instruction loading and bounded evidence collection still read
workspace data. For example:

```sh
astrid run "inspect failing tests" --model <model-slug> --write-policy deny --shell-policy ask
```

`--shell-policy allow` explicitly grants broad execution with your account's
permissions, including network and destructive commands. The general shell has
no separately enforceable network/destructive policy. It starts fresh in the
workspace with null stdin; cwd is not a sandbox. This release targets trusted
local repositories and assumes no hostile concurrent filesystem changes.

Shell reads are 8 KiB chunks, queued output is at most eight chunks (64 KiB),
and capture/live admission are limited to a 64 KiB prefix per stream. Once live
admission overflows, later bytes are drained and omitted, with explicit counters.
Already accepted public events remain lossless. Timeout and cancellation stop
and reap the process group independently of a stalled event consumer; final event
delivery/run return can still wait for the receiver to drain or disconnect.
Detached descendants remain outside the process-group contract. Partial output
survives timeout/cancellation; unknown exit codes and unread bytes are not invented.

Output metadata distinguishes observed/captured/live-queued bytes, omissions,
pipe completeness, and an incomplete UTF-8 suffix unavailable for text rendering.
Raw streamed bytes preserve per-stream order. Invalid complete UTF-8 is rendered
with replacement characters; terminal controls are sanitized. Streamed prefixes
are not printed again at completion. Captured bytes that were not streamed remain
in the structured result; the CLI reports live omissions.

Native inspection results are limited to 64 KiB serialized JSON. Inventories use
bounded sorted selection (up to 1,024 candidate records, with a 32 KiB returned
record budget). Grep scans up to 1 MiB per file/16 MiB total and skips oversized
16 KiB lines with explicit incomplete coverage. Large/deep inventories and
nonrepresentable names cannot silently establish an empty search. Mutation
inputs, repository instructions, model output, and total conversation/state
memory do not acquire a context budget in this phase.

Git observation reports branch/HEAD and staged, unstaged, and untracked paths.
It disables optional index writes, fsmonitor, configured clean/process filters,
and submodule inspection. Filter suppression can conservatively change status
classification relative to a normal Git invocation. Git operations have time and
output limits; unavailable/incomplete metadata stays explicit. Non-Git workspaces
remain usable.

Change evidence compares the starting working tree with the ending workspace,
preserving existing staged/unstaged edits. Content baselines cover tracked and
non-ignored untracked files, capped at 1 MiB per file and 16 MiB total; bounded
path-presence scans include ignored names so ignore-rule changes cannot fabricate
creation/deletion. Inventories stop at 10,000 entries or their scan deadline;
a large ignored tree can make existence coverage incomplete. Native write/edit
calls also retain bounded direct evidence, including ignored files. Patches are
whole-file replacements capped at 64 KiB; final serialized reports are capped at
1 MiB including Git metadata. Coverage changes, binary content, oversized data,
and collection failures are explicit. No rollback, staging, or commits occur.

Native mutation evidence is correlated to its tool under the stated concurrency
assumption. Shell/external changes are observations during the run; causation is
not guaranteed. Final evidence is attempted even for cancelled/failed runs without
rewriting committed tool outcomes. Events/results remain in memory; conversation
history still grows without compaction.

## Validation and acceptance

```sh
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

Tests use a local HTTP/SSE server with deliberately fragmented events. They
exercise the real Responses adapter and agent loop without network model APIs.
They cover completion barriers, interrupted responses, tool errors, edit
ambiguity, workspace escapes, shell rejection/timeout, exact call ceilings,
instruction loading, reasoning-item replay, and OAuth identity validation.
Phase 2 tests cover flood limits, early-output handshakes, cleanup with a stalled
event consumer, the permission matrix, dirty Git baselines, ignore-rule changes,
filter/fsmonitor execution prevention, and committed-edit evidence after cancellation.
Runtime tests also cover correlated lifecycle reconstruction, cancellation
races, cleanup failure, headless operation, UI failure after a committed edit,
and cancellation during backpressured streaming. CLI input tests exercise
nonblocking reads and cancellation without a blocked reader thread.

The deterministic acceptance test copies `tests/fixtures/greeting` into a
temporary directory. It runs the failing tests, searches/reads the code, edits
an existing file, creates `GREETING.txt`, runs tests again, checks the passing
result, and finishes. Model responses are scripted; native tools, file
mutations, and shell commands are real.

For live acceptance, copy the fixture to a disposable directory and invoke
the installed binary there:

```sh
fixture_dir=$(mktemp -d)
cp -R tests/fixtures/greeting/. "$fixture_dir/"
cd "$fixture_dir"
astrid run "Inspect this repository, run its failing tests, fix the greeting bug without changing tests, create the required GREETING.txt, then run tests again and report their result." --model <model-slug>
```

Review and confirm each shell command. Both tests must pass, `name.trim()`
must appear in `src/lib.rs`, and `GREETING.txt` must contain `Hello, Astrid!`
followed by a newline. No live inference result is implied by mocked tests.

## Runtime and events

`runtime::run` accepts a model provider, tool executor, permission handler,
RunConfig (including PermissionPolicy), Cancellation handle, and an optional bounded Tokio event sender.
Drain an attached event receiver concurrently with execution. Pass None for
headless operation. Dropping a receiver detaches event delivery; it does not
cancel the run. The CLI explicitly requests cancellation if rendering fails.
Keep polling the runtime until its terminal RunResult, including after calling
cancel; dropping its future is not the cancellation/cleanup API.

Each accepted run owns typed execution state and a fresh Session. Turns contain
one model invocation and its resulting tool batch. Astrid IDs identify runs,
sessions, turns, model calls, and tool calls; provider tool IDs remain protocol
metadata. Event sequence numbers establish ordering within a run.

Assistant text deltas are provisional. Only validated model completion commits
assistant output. Interrupted text remains visible with a `!` or `×` response-interrupted indicator.
All validated tool requests are announced before sequential execution. Permission
waiting is distinct from tool execution. Every accepted run and requested tool
receives exactly one terminal outcome, including denied, cancelled, timed-out,
and skipped operations. A completed shell call may have a nonzero command exit.
Normal run completion does not independently establish task success.

Events are owned and serializable, but are not persisted. They contain visible
text, raw tool-output bytes, requests, permission decisions, mutation evidence,
and workspace reports, excluding credentials and
opaque provider continuation. The runtime validates and updates state before
emitting events; it does not execute by replaying events. While an event receiver
remains attached, backpressure can delay progress and cancellation notification.
Already-applied events remain pending across model cancellation until delivered
or the receiver closes, preserving sequence ordering.

See [architecture](docs/architecture.md), the
[Phase 0 contract](docs/adr/0001-phase-0-execution-contract.md), and the frozen
[Phase 1 runtime contract](docs/adr/0002-phase-1-runtime-and-event-model.md), with
the [Phase 2 amendments](docs/adr/README.md).
