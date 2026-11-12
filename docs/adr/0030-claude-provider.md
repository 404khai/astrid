# ADR 0030: Anthropic Messages API provider

Date: 2026-11-12
Status: **Accepted 2026-11-12.** The maintainer asked to "add a claude adapter to astrid, cuz that is the subscription I have right now".

## Decision

- **Adapter.** `src/anthropic.rs` implements `ModelProvider` over
  `POST https://api.anthropic.com/v1/messages` with streaming SSE, raw HTTP
  through the existing reqwest dependency, and `anthropic-version: 2023-06-01`.
  Astrid's tools map to `tool_use`/`tool_result` blocks with
  `eager_input_streaming`. Accumulated tool JSON is validated before any tool
  runs.
- **Models.** Exact IDs only: `claude-opus-5-5`, `claude-sonnet-5-5`,
  `claude-fable-5-1`, `claude-opus-5` and `claude-haiku-4-5`. `astrid run
  --model claude-*` selects this adapter.
- **Thinking and effort.** Thinking is left at each model's default (always on
  for the 5.x models), and Astrid's effort maps to `output_config.effort`.
  `none` is rejected because those models cannot disable thinking.
- **Refusals.** Server-side refusal fallbacks (`fallbacks: "default"`, beta
  `server-side-fallback-2026-07-01`) are on for the 5.x models. A final
  `refusal` or `max_tokens` stop is an error, never a fake completion.
- **Caching.** Top-level `cache_control: {type: "ephemeral"}` is set.
- **Replay.** Claude's raw assistant blocks, including thinking signatures and
  fallback blocks, are stored as an opaque `anthropic_content` item inside the
  existing `ResponseContinuation`, next to the provider-neutral text and tool
  items. The persisted session format is unchanged. Native replay sends the raw
  blocks back; visible-history replay (for example after switching provider)
  sends text and tool calls only. The OpenAI adapter ignores the opaque item.
- **Credentials.** A Claude Console API key, taken from `ANTHROPIC_API_KEY` or
  from the login Keychain (`astrid claude login`, written through
  `security -i`). It is sent only as `x-api-key`. `.env` files are never read.
  **A Claude Pro or Max chat subscription cannot be used:** its sign-in is for
  Anthropic's own apps (Claude, Claude Code), not third-party agents. API usage
  is billed to the Console account.

## Not yet

- Screen tools (`--look`, `--computer-use`) on Claude.
- The interactive `astrid` prompt; it still uses OpenAI.
- Provider-reported usage and cost in traces.
- `astrid models` listing.

Cost of changing later: Low to Medium.
