# Astrid

An experimental Rust agent harness. Phase 0 implements one repository task per
process, streaming OpenAI output and executing native tools sequentially.
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

| Setting | CLI | Environment | Default |
| --- | --- | --- | --- |
| Model | `--model` | `ASTRID_MODEL` | Required |
| Model-call ceiling | `--max-model-calls` | `ASTRID_MAX_MODEL_CALLS` | 20 |
| Shell timeout, seconds | `--shell-timeout` | `ASTRID_SHELL_TIMEOUT` | 30 |

The invocation directory is the workspace root. Astrid reads only that
directory's `AGENTS.md` as initial repository instructions; parent and nested
instruction discovery is deferred. Assistant text streams to stdout. Model
and tool lifecycle messages, arguments, results, and failures appear on stderr.
The terminal logo is a pixel interpretation of `logo.png`, with cyan eyes and
blue gradient arches. Set `NO_COLOR=1` to disable logo colors. Redirected
output contains no logo or color escapes.

Every successful response without tool calls ends the task. Recoverable tool
failures return to the model. Provider/protocol errors and an exhausted
model-call ceiling exit nonzero. Astrid does not automatically retry mutations
or provider requests. Its final answer is a model claim, not proof of success.

## Native tools

| Tool | Arguments | Behavior |
| --- | --- | --- |
| `read_file` | `path` | Read a UTF-8 file |
| `write_file` | `path`, `content`, `overwrite` | Atomic creation or explicit overwrite; parent must exist |
| `edit_file` | `path`, `old_text`, `new_text` | Exact replacement only when there is one match |
| `list_directory` | `path` | Sorted immediate entries |
| `glob` | `pattern` | Sorted workspace-relative file matches |
| `grep` | `path`, `pattern` | Rust regex search with filenames and 1-based line numbers |
| `shell` | `command` | Confirmed, noninteractive `/bin/sh -c` execution |

File tools automatically operate within the workspace. Traversal, outside
paths, symlink paths, and mutations to hard-linked files are denied. Search
skips `.git`, symlinks, and binary files for textual grep. Existing file
permissions survive edits. Empty or ambiguous edit targets cause no mutation.

Each shell call asks for an explicit `yes` through `/dev/tty`. Missing terminal,
EOF, or any other answer denies execution. There is no blanket approval flag.
Each command starts fresh in the workspace with no interactive stdin and
returns stdout, stderr, exit code, and timeout status. Nonzero command exits are
reported to the model without aborting the harness. A timeout kills the shell's
process group; partial stdout/stderr are marked unavailable on timeout.

Use disposable repositories for Phase 0. Shell confirmation grants execution
with your account's permissions; the shell is not confined to workspace paths.
Filesystem checks assume no hostile concurrent filesystem changes. Advanced
cancellation, output limits, and a configurable permission framework are
deferred. Full tool results remain in the growing in-memory conversation.

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

See [architecture](docs/architecture.md) and the frozen
[Phase 0 contract](docs/adr/0001-phase-0-execution-contract.md).
