# Phase 2 architecture

One Cargo package contains a library and CLI binary. Sessions remain ephemeral;
tools remain sequential; OpenAI remains the only provider.

```text
CLI: config, event rendering, Ctrl-C, cancellable terminal permission input
  -> runtime API: Session, RunResult, cancellation, bounded event delivery
     -> execution state: checked transitions, Astrid identities, sequence numbers
     -> agent policy: instructions and normal termination policy
     -> OpenAI adapter: request, streaming, completion validation, replay
        -> authentication: app-owned ChatGPT OAuth credentials and renewal
     -> permission policy: typed read/write/execute allow/ask/deny
     -> workspace evidence: bounded Git/file baselines and before/after patches
     -> tool executor: bounded native inspections, streaming shell, structured outcomes
        -> workspace: invocation-root path validation and AGENTS.md loading
```

## Model and conversation boundary

`Message` distinguishes the user request, completed assistant responses, and
tool results. `ModelRequest` carries the configured model, instructions, and
message history. `ModelResponse` contains assistant text, requested tools, and
opaque continuation data owned by the OpenAI adapter.

The narrow `ModelProvider` trait exists for deterministic testing. It is not a
universal provider/capability abstraction. Authentication has its own
`Authentication` boundary, supplying a bearer credential without making the
agent loop aware of OAuth, account selection, renewal, or API keys.

The provider uses OpenAI's public Responses endpoint, manually replays complete
output items, sets `store: false`, and requests encrypted reasoning items.
Functions are grouped under the `astrid` namespace as required by the documented
subscription route. It sends no paid API key and uses no private backend URL.

## Completion barrier

SSE framing operates on bytes so network chunks may split UTF-8, line endings,
JSON, or function arguments. Assistant text is displayed immediately. Function
arguments accumulate in the adapter and are checked against final output.
Only a successful `response.completed` with a fully validated output envelope
produces a `ModelResponse`. An interrupted, failed, or malformed response never
exposes tools to the loop, even if `response.output_item.done` already arrived.

The runtime validates provider call IDs across the entire batch, appends the completed
assistant response, executes tools in order, appends each structured result,
and makes the next model request. All batch requests receive Astrid IDs and
ToolCallRequested events before any sequential dispatch. Termination and dispatch are code decisions.
They are not delegated to a prompt. A successful response with no tools ends
the run; the model-call ceiling reports an incomplete task.

## Tool boundary

All seven capabilities have explicit schemas and typed Rust arguments.
Validation and execution errors become structured tool results containing a
code and message. ExecutionEvent exposes requested arguments and outcomes. A single checked
transition boundary updates authoritative ExecutionState and emits ordered,
owned events. Consumers can reconstruct lifecycle and transcript; credentials
and opaque provider continuation stay outside the public stream.

Writes and edits use an atomic replacement in the target directory. Exact
replacement counts overlapping occurrences; zero or multiple matches fail
before creating a replacement. Symlinks and traversal are rejected; hard-linked
file mutations are rejected. These checks assume a disposable workspace
without hostile concurrent path swaps, not a complete process sandbox.

The runtime evaluates typed per-run read/write/execute policies before dispatch,
including automatic allow/deny events. Defaults preserve automatic native tools
and per-shell confirmation. Ask uses PermissionHandler; tools never read terminal
input. Explicit shell allow grants account-level authority, not filesystem or
network containment. Repository content does not provide policy configuration.

Each shell invocation gets a fresh process group, workspace cwd, null stdin, and
separate pipes. Local reader futures use fixed 8 KiB buffers and bounded captures;
try_send admits only bounded raw output into a tool/runtime channel. Admission
stops on overload, subsequent bytes are drained, and omissions are counted. The
runtime owns event sequencing and polls publication concurrently with execution,
so backpressure cannot prevent timeout/cancellation cleanup. Public lifecycle
notification may still wait for an attached observer. Cancellation terminals carry
partial output without fabricated conversation results. Native inspections run
cooperatively in joined blocking jobs with source/result bounds.

Workspace observation captures Git identity/status plus bounded file contents
before dispatch and at termination. Git commands are read-only with bounded
runtime/output, process-group cleanup, fsmonitor/filter suppression, and optional
locks disabled. Path identities use UTF-8/hex labels; NUL protocol parsing preserves
unusual names. A bounded presence inventory distinguishes missing content coverage
from actual file absence. Staged state is reported independently; content patches
use the initial working tree rather than HEAD. Per-call native mutation evidence
also covers ignored paths. A final reporting failure leaves tool/run outcomes
intact and reports unavailable evidence. No attribution of arbitrary shell or
external edits is promised.

See [ADR 0003](adr/0003-bounded-tool-output.md),
[ADR 0004](adr/0004-execution-authority.md), and
[ADR 0005](adr/0005-workspace-change-evidence.md) for limits and contracts.

## Authentication

`astrid login` uses the documented dynamic open-source registration entrypoint,
a stable host ID, PKCE S256, random state and nonce, and a loopback callback.
The host ID is a persisted UUIDv4 URI (`urn:uuid:...`), matching OpenAI's
supported formats. A rejected development host ID is repaired only before
successful registration.
It exchanges a code only with the issued application client ID. It validates
RS256 signatures against OpenAI's JWKS, issuer, audience, expiry, subject, and
nonce, then checks granted ChatGPT plan scopes.

Credential records are atomic and owner-only. A file lock serializes renewal
across Astrid processes. Rotating access/refresh tokens and their scopes/expiry
are saved together. Failed identity validation does not replace the selected
account. There is one selected account, no account picker, and no
automatic API-key fallback. Astrid never reads Codex credential files.

## Official protocol references

- [Subscription registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in)
- [Account sessions and refresh](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)
- [Subscription inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)
- [Subscription route limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)
- [Responses streaming events](https://developers.openai.com/api/reference/resources/responses/streaming-events)
- [Function calling](https://developers.openai.com/api/docs/guides/function-calling)

## Runtime lifetime and cancellation

`runtime::run` owns conversational and execution state; the CLI only consumes
its public events/result and supplies permission decisions. A bounded channel
applies backpressure while attached. Receiver closure detaches delivery without
changing execution outcomes. None permits intentional headless execution.
RunResult is returned independently of rendering. The CLI's broken-pipe policy
requests cancellation through the same Cancellation handle exposed to callers.

TextSink is asynchronous so provider text participates in bounded backpressure.
If model cancellation drops an in-progress event delivery, the runtime retains
the already-applied pending event batch and flushes it before the next transition.
The first-text marker and its actual delta are accepted together; pending storage
is bounded to that pair.
This prevents sequence gaps. A slow attached consumer may delay delivery and
cancellation acknowledgement; callers must drain concurrently or detach.

Cancellation is checked before dispatch. Atomic file operations may finish;
their real outcomes are recorded before cancellation is acknowledged. The active
operation is Cancelled; later unstarted requests are Skipped without fabricated
conversation results. Shell cancellation uses explicit cleanup, not dropping
its future. Cleanup failure makes the run Failed and retains cancellation
history when cancellation was requested. General library callers must poll the
run to termination after requesting cancellation to obtain cleanup and outcomes.

Session/Run/Turn IDs and ModelCallId/ToolCallId are Astrid-owned UUID types.
ExecutionState tracks turn, model, and tool states plus committed assistant
text. Events have per-run sequence numbers and correlated parent IDs. Public
state transition validation rejects illegal or repeated terminal transitions
without changing state; runtime events are descriptive, not event sourcing.

The maintained contracts are [ADR 0001](adr/0001-phase-0-execution-contract.md)
and [ADR 0002](adr/0002-phase-1-runtime-and-event-model.md), with the Phase 2
amendments in [the ADR index](adr/README.md).

## CLI presentation

`src/console.rs` consumes owned runtime events and keeps only presentation state:
short IDs, visible turn/model-call counters, requested tool names, and display
lines. It neither validates execution transitions nor decides when to invoke
a model/tool. Permission/output/change rendering consumes the Phase 2 public
events and does not own their execution semantics.

There is no full-screen terminal framework or raw-mode event loop. The supported
macOS terminal exposes its dimensions through `TIOCGWINSZ`; ANSI scroll margins
reserve the header and footer on the primary screen. Stream text is sanitized,
wrapped by Unicode display width, and retained in a bounded redraw buffer.
A CLI timer checks dimensions between runtime events. Canonical `/dev/tty`
input and OS Ctrl-C remain in use; the renderer places the approval prompt
before the existing cancellable input task reads it. Resize is deferred while
an approval answer is being echoed. An approval larger than the output viewport
switches to append-only rendering so its command remains reviewable. A drop guard restores normal scroll margins
and cursor visibility on completion and error paths. Redirected/dumb terminals
use append-only output. This is a reversible CLI presentation choice, not a
new runtime abstraction.
