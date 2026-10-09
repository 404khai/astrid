# Astrid

Development planning: [phase review and delivery process](docs/development.md),
[Phase 1 review](docs/reviews/phase-1.md), and
[Phase 2 tickets](docs/plans/phase-2.md).

Phase 3 implements [context accounting, selection, and compaction](docs/plans/phase-3.md).
Use `astrid run "inspect this repository" --model <model-slug> --show-context`
to see stable context item IDs/sources and each prepared request's serialized
bytes by source, retention/removal reasons, and incomplete summary text/lineage.
Provider token counts remain unavailable; the displayed non-opaque JSON size
heuristic is not a full context total or guaranteed provider fit.

The CLI protects instructions, your task, and the latest complete tool exchange.
Older exchanges are selected with file-reference/lexical priority and recency,
then omitted history can become one bounded deterministic summary. Defaults:
32,768 estimated context units, 4,096 response-reserve units, 524,288 serialized
request bytes, and 4,096 summary text bytes. Change them with `--context-tokens`,
`--response-reserve`, `--context-bytes`, and `--summary-bytes`; compare policies
with `--context-policy recency` or `--context-policy file-references`.

If protected context exceeds the configured allowance, the run stops before the
next model invocation. The response reserve is planning headroom, not an enforced
provider output limit. Original ephemeral history remains retained; request
selection does not bound all runtime memory or create persisted context.

For hands-on checks, see [Try Phase 2](docs/testing-phase-2.md).

An experimental Rust agent harness. Phase 2 provides a typed execution runtime
with ordered events, cancellation, bounded tool output, configurable permissions,
and workspace change evidence. Interactive conversations retain multiple runs;
`astrid run` executes one fresh repository task.
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
| Permission preset      | `--mode`            | —                        | auto     |
| Native read policy     | `--read-policy`     | —                        | allow    |
| Native write policy    | `--write-policy`    | —                        | allow    |
| Shell policy           | `--shell-policy`    | —                        | ask      |

The invocation directory is the workspace root. Astrid reads only that
directory's `AGENTS.md` as initial repository instructions; parent and nested
instruction discovery is deferred.

On an interactive terminal, `astrid run` shows Astrid's existing pixel logo,
compact model/provider/workspace metadata, and a scrolling execution stream.
Completed output appends to normal terminal scrollback; a small Ratatui inline
region shows unfinished streamed text, status, and permission input. Astrid does
not enter an alternate screen. Bare `astrid` opens a model/task prompt;
`astrid run` remains available for one-shot invocation. Bare `astrid` returns to
the prompt after each run, retaining the selected conversation. Ctrl-C cancels an
active run; at the idle prompt it exits.
Tool requests show their target, results show a concise summary, and shell
stdout/stderr stream separately while the command runs. Effective permission
policy, initial branch/dirty paths, native mutation patches, and final observed
workspace changes are visible in the execution stream. Completion means
the run ended normally, not that the task was independently verified.

Replies are cyan, approvals yellow, successful completion green, and failures
red. Labels and secondary information use dim terminal text. Set `NO_COLOR=1`
to disable colors. The former fixed header/footer viewport remains opt-in with
`ASTRID_FIXED_VIEWPORT=1`. The default inline renderer keeps the logo and startup
metadata in scrollback, without pinning a header above the execution stream.

When either output stream is redirected, or `TERM=dumb`/terminal sizing is
unavailable, rendering falls back to ordinary append-only text without cursor
controls or colors. Assistant text continues to stream to stdout; concise tool
activity, command output, and failures go to stderr. Operation approvals use
`/dev/tty`, including the complete command and workspace, independently of
redirected output. Context source/selection details are shown with `--show-context`.

Every successful response without tool calls ends the task. Recoverable tool
failures return to the model. In one-shot mode, provider/protocol errors and an
exhausted model-call ceiling exit nonzero. Interactive mode displays the outcome
and returns to input. Astrid does not automatically retry mutations
or provider requests. Its final answer is a model claim, not proof of success.

## Interactive sessions and modes

`/new` starts a fresh conversation. `/sessions` opens a searchable list showing
session ID, first-task label, active marker, run count, and latest outcome; use
arrow keys and Enter to switch. Up to 32 sessions live in the current Astrid
process and disappear on exit. Follow-ups retain committed messages and context;
they do not rerun old tools. An interrupted, incomplete tool batch is preserved
but cannot accept follow-ups; use `/new` in that case.

`/mode` opens the permission selector. Direct commands are:

| Command | Reads | Workspace edits | Shell |
| --- | --- | --- | --- |
| `/mode ask` | allow | ask | ask |
| `/mode auto` (default) | allow | allow | ask |
| `/mode unbound` | allow | allow | allow |

The mode is visible in startup metadata, composer, and execution status. It
applies to subsequent runs and remains unchanged when switching sessions.
`unbound` grants full tool access, including broad account-level shell execution.
Native path validation, shell timeouts, cancellation, and output limits remain.
Its logo arches are red (`#F94447`, `#EC1A1D`, `#F94447`) and eyes yellow
(`#F7C600`); `NO_COLOR` disables the palette.

One-shot runs accept `--mode ask|auto|unbound`. Explicit `--read-policy`,
`--write-policy`, or `--shell-policy` overrides the corresponding preset action;
the display uses `custom` when the effective policy matches no named mode.
Modes are not remembered after exit or read from repository settings.

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
memory remain separate from Phase 3's selected-request budget.

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

### Interactive startup and scrollback

Run `astrid` without arguments to open the designed terminal welcome screen:
blue pixel logo with cyan eyes, model/provider/workspace metadata, and a task
input directly beneath it. Astrid reuses your last-selected model; `ASTRID_MODEL` can override it.
On first use with no saved choice, it selects the first model from your account
catalog. Type `/` in the input box for commands, then `/model` to see available
models. Use ↑/↓ and Enter to select; type to filter and Esc to cancel.
The slash-command menu also supports ↑/↓ and Enter. All popups replace the
same input area; Esc returns to task entry without creating another box.
Enter sends a task; Ctrl-J inserts a newline. The composer supports multiline
paste, wrapping, cursor movement, and Unicode editing. Ctrl-C cancels; empty
Ctrl-D exits (or cancels model selection). Tab completes a slash command.
Input is capped at 64 KiB with a visible notice when a paste exceeds that limit.
During a run, approvals require typing `yes` and pressing Enter after the full
command is displayed. Pasted text and pre-prompt typeahead cannot grant an
inline approval. Redirected output retains canonical `/dev/tty` approval input.
Saved authentication is reused. Bare `astrid` keeps in-memory conversations
available for follow-ups; `astrid run` starts one fresh repository task.

The default inline display inserts completed rows above its live region, keeping
the transcript in normal scrollback. Use your terminal's scrollbar, mouse wheel, trackpad, or
scrollback keyboard shortcuts to review earlier output. Replies are cyan,
approvals and cancellations yellow, successful completion green, and failures
red. Set `NO_COLOR=1` for plain text. Redirected output stays plain.

The previous fixed header/footer viewport is available with
`ASTRID_FIXED_VIEWPORT=1`; its scroll region can prevent normal transcript
scrollback, so it is no longer the default.


## Optional observability

```sh
astrid settings
astrid settings set observability on
astrid run "inspect the failing test" --model <model>
astrid run "task" --model <model> --observability off
astrid trace
astrid trace <run-id>
astrid stats
astrid settings set observability off
```

Observability defaults to off. A run flag overrides the persistent user setting;
settings changes affect subsequent runs. Both one-shot and interactive runs use
the same runtime recorder. Disabling recording preserves execution events,
permissions, cancellation and context budgets, and retains existing trace files.
Invalid settings fail explicitly, including when a run flag is present.

Settings live in `~/.config/astrid/settings.json`; traces in
`~/.config/astrid/traces/`, with private file/directory permissions. Traces contain
correlated execution metadata, operation timings, numeric context accounting and
provider-reported usage where available. They omit task/response text, file paths,
tool arguments/results, summaries and private provider continuation. They cannot
resume sessions or replay the full conversation.

`trace <run-id>` prints a JSON summary. Timing names are explicit: inclusive
runtime call duration, adapter preparation/authentication/provider attempt, first
visible text, delivery waiting, and tool execution. Permission waiting is separate.
These are overlapping intervals; do not sum them into total elapsed time. True
first-token latency, prefill/decode rates and subscription monetary cost remain
unavailable. Missing usage is `null`; reported zero is zero. `stats` reports known
sums with reporting-call coverage; cached tokens are a subset of input tokens.
Unrecorded runs are unknown, and incomplete/unreadable traces are counted separately.

The recorder bounds its queue (256 records), each record (16 KiB), each run
(8 MiB), and the store (256 MiB, at most 4,096 directory entries including reserved
publication slots). It stops recording at limits or writer failure, reports
incomplete/unavailable recording independently of the run outcome, and never
silently deletes old traces. Finalization waits at most 250 ms by default. Partial
files remain inspectable; a complete trace has a terminal event, valid sequence,
footer and published `.jsonl` filename. See [Phase 4 evidence](docs/reviews/phase-4.md)
and accepted [ADR 0008](docs/adr/0008-observability-control.md) /
[ADR 0009](docs/adr/0009-persistent-trace-contract.md).

Tool result previews can be enabled with `/settings` or
`/settings expanded-tool-calls on` (use `off` to collapse them). The preference
persists across launches. From the shell, use
`astrid settings set expanded-tool-calls on|off`. Expanded previews show up to
12 lines; shell output and permission/error information remain visible in either
mode. The input starts at two lines and grows with wrapping or Ctrl-J newlines.

Type `@doc` in the composer to look up matching repository files; use ↑/↓ to
select and Enter or Tab to insert a repository-relative `@path`. Lookup runs off
the terminal thread and adds a path reference to the message, without reading or
attaching file contents. Ignore rules apply first; ignored/hidden files appear
only when no ordinary files match. Dependency/build directories remain excluded.

Library callers can use `astrid::file_lookup::find_files(repo_root, base, pattern,
max_hits)`. It accepts repository-relative bases and glob patterns, returns sorted,
unique paths with `/` separators, and reports validation/traversal failures.
