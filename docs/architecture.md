# Phase 0 architecture

One Cargo package contains a library and CLI binary. No multi-crate workspace,
agent SDK, persisted conversations, or later-phase runtime concepts are added.

```text
CLI: task, config, rendering, shell confirmation
  -> agent: in-memory messages and bounded sequential loop
     -> OpenAI adapter: request, streaming, completion validation, replay
        -> authentication: app-owned ChatGPT OAuth credentials and renewal
     -> tools: typed arguments, structured results, filesystem and shell
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

The loop validates call IDs across the entire batch, appends the completed
assistant response, executes tools in order, appends each structured result,
and makes the next model request. Termination and dispatch are code decisions.
They are not delegated to a prompt. A successful response with no tools ends
the run; the model-call ceiling reports an incomplete task.

## Tool boundary

All seven capabilities have explicit schemas and typed Rust arguments.
Validation and execution errors become structured tool results containing a
code and message. Tool observations expose requested arguments and outcomes.
These are ephemeral UI observations, not stable Phase 1 execution events.

Writes and edits use an atomic replacement in the target directory. Exact
replacement counts overlapping occurrences; zero or multiple matches fail
before creating a replacement. Symlinks and traversal are rejected; hard-linked
file mutations are rejected. These checks assume a disposable workspace
without hostile concurrent path swaps, not a complete process sandbox.

Shell authorization occurs before spawning `/bin/sh`. Each invocation gets a
fresh process group, workspace cwd, null stdin, and separate stdout/stderr
pipes. A timeout covers execution and pipe draining and kills that process
group. Detached descendants and general cancellation remain outside Phase 0.
Captured output is unavailable when a timeout interrupts draining; that is
reported explicitly instead of inventing output or an exit status.

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
account. There is one selected account in Phase 0, no account picker, and no
automatic API-key fallback. Astrid never reads Codex credential files.

## Official protocol references

- [Subscription registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in)
- [Account sessions and refresh](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)
- [Subscription inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)
- [Subscription route limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)
- [Responses streaming events](https://developers.openai.com/api/reference/resources/responses/streaming-events)
- [Function calling](https://developers.openai.com/api/docs/guides/function-calling)

The maintained contract is [ADR 0001](adr/0001-phase-0-execution-contract.md).
