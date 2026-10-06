# Astrid

An experimental Rust agent harness. Phase 1 provides a typed execution runtime
with ordered events, cancellation, and one fresh repository task per process.
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

The invocation directory is the workspace root. Astrid reads only that
directory's `AGENTS.md` as initial repository instructions; parent and nested
instruction discovery is deferred.

On an interactive terminal, `astrid run` shows Astrid's existing pixel logo,
compact model/provider/workspace metadata, and a scrolling execution stream.
The header and three-line footer stay in place. The footer displays the actual
run ID, turn, model-call count, and current activity; the bottom prompt accepts
shell approvals when requested. Tasks still start through `astrid run`; there
is no interactive conversation composer or mode switch. Ctrl-C cancels the run.
Tool requests show their target, results show a concise summary, and shell
stdout/stderr appear separately after the command completes. Completion means
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
activity, command output, and failures go to stderr. Shell approvals use
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
| `shell`          | `command`                      | Confirmed, noninteractive `/bin/sh -c` execution          |

File tools automatically operate within the workspace. Traversal, outside
paths, symlink paths, and mutations to hard-linked files are denied. Search
skips `.git`, symlinks, and binary files for textual grep. Existing file
permissions survive edits. Empty or ambiguous edit targets cause no mutation.

The runtime requests permission before starting each shell call. The CLI asks
for an explicit `yes` through `/dev/tty`. Missing terminal,
EOF, or any other answer denies execution. There is no blanket approval flag.
Each command starts fresh in the workspace with no interactive stdin and
returns stdout, stderr, exit code, and timeout status. Nonzero command exits are
reported to the model without aborting the harness. A timeout kills the shell's
process group; partial stdout/stderr are marked unavailable on timeout. Ctrl-C
requests cancellation through the public runtime handle, interrupts permission
waiting and model streaming, and stops active shell commands without waiting
for their timeout. Completed file changes are retained.

Use disposable repositories for this experimental runtime. Shell confirmation
grants execution with your account's permissions; the shell is not confined to
workspace paths. Filesystem checks assume no hostile concurrent filesystem
changes. Detached-process confinement, output limits, and a configurable
permission framework are deferred. Full tool results remain in the growing
in-memory conversation.

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
RunConfig, Cancellation handle, and an optional bounded Tokio event sender.
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
text, requests, permission decisions, and outcomes, excluding credentials and
opaque provider continuation. The runtime validates and updates state before
emitting events; it does not execute by replaying events. While an event receiver
remains attached, backpressure can delay progress and cancellation notification.
Already-applied events remain pending across model cancellation until delivered
or the receiver closes, preserving sequence ordering.

See [architecture](docs/architecture.md), the
[Phase 0 contract](docs/adr/0001-phase-0-execution-contract.md), and the frozen
[Phase 1 runtime contract](docs/adr/0002-phase-1-runtime-and-event-model.md).
