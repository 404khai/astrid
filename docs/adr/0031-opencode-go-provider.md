# ADR 0031: OpenCode Go provider

Date: 2026-11-12
Status: **Accepted 2026-11-12.** The maintainer said "add an opencode v2 provider and merge as well".

## Context

The maintainer can't pay for Claude API credits right now. A Claude chat
subscription can't back a third-party agent (ADR 0030). OpenCode Go
($10–40/month) explicitly allows other coding agents. Its conditions: send
typical coding-agent traffic, identify with the client's own user agent, and
send a stable `x-opencode-session` per conversation.

## Decision

- **Selection.** `astrid run --model opencode/<id>` uses an OpenAI-compatible
  Chat Completions adapter at `https://opencode.ai/inference/go/openai/v1`.
  The model must appear in the live public catalog
  (`/inference/go/v1/models`) with `chat-completions` and tool support.
  `astrid opencode models` lists them. Models served only through the
  responses, messages or generate-content APIs are rejected for now.
- **Requests.** Astrid sends user agent `Astrid/<version>`, the API key as a
  bearer token, and `x-opencode-session: astrid-<sha256(model, instructions,
  first message)>`, which stays the same as a conversation grows. Reasoning
  effort is sent only when the catalog lists that exact level.
- **Streaming.** Responses stream over SSE. Tool-call arguments are
  accumulated per index and must parse as a JSON object. A `length` or
  `content_filter` finish is an error, never a fake completion.
- **Replay.** The raw assistant message, including the catalog-declared
  `reasoning_content`, is stored as an opaque `opencode_chat` item in the
  existing continuation and replayed natively. Visible-history replay never
  invents reasoning. The persisted format is unchanged, and the OpenAI adapter
  ignores the item.
- **Credentials.** The key comes from `OPENCODE_API_KEY` or the login Keychain
  (`astrid opencode login`, through a shared `keychain` module that writes via
  `security -i`). `.env` files are never read.

## Not yet

- OpenCode models on the responses, messages or Google APIs.
- Screen tools.
- Usage and cost telemetry.
- The interactive prompt.
