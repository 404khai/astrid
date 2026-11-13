# ADR 0032: Claude Agent SDK provider

Date: 2026-11-13
Status: **Accepted 2026-11-13.** The maintainer said "build and integrate the sdk, I don't mind", after being told about Anthropic's restrictions on subscription use.

## Context

The maintainer has a Claude subscription and no API credits. The Claude Agent
SDK runs the locally installed Claude Code, which can use the claude.ai login.

Anthropic's Agent SDK documentation says: "Unless previously approved,
Anthropic does not allow third party developers to offer claude.ai login or
rate limits for their products, including agents built on the Claude Agent
SDK." The maintainer accepted that risk for their own personal use. Astrid
does not offer or broker a claude.ai login; it uses whatever account Claude
Code is already logged into.

## Decision

- **Selection.** `astrid run --model claude-code/<model>` uses
  `ClaudeCodeProvider`. The model is `opus`, `sonnet`, `haiku`, `fable` or an
  exact `claude-*` id.
- **The bridge.** Each Astrid model call spawns
  `node apps/claude-agent-bridge/bridge.mjs` (the SDK pinned at 0.3.323, about
  400 MB because it includes Claude Code; installed with `npm install`, and
  gitignored). The bridge runs **one bounded turn** (`maxTurns: 1`) with:
  - `tools: []`, so Claude Code's built-in tools (Bash, Read, Edit, …) are
    disabled;
  - `settingSources: []`, so no user or project `CLAUDE.md` or settings are
    loaded;
  - Astrid's instructions as the system prompt;
  - Astrid's tools declared through an in-process MCP server, with JSON Schema
    converted by `z.fromJSONSchema`.
- **Astrid keeps authority.** `canUseTool` captures every Astrid tool call and
  **denies** it, so Claude Code executes nothing. The calls come back to
  Astrid's runtime, which applies its own permissions and approvals (local
  modes, mail, InstaBox) and runs them. Events, context accounting and traces
  are unchanged.
- **History.** The SDK takes a prompt rather than a Messages array, so each
  turn replays visible history (text, tool calls, results) as a tagged
  transcript. There is no native thinking replay, no effort control and no
  prompt-caching control. Usage is reported as unavailable.
- **Limits and errors.** 15-minute turn timeout, 4 MiB per bridge line, and
  the process is killed on drop. Bridge errors become provider errors.
  Tool-call arguments must parse as JSON objects.
- **Visibility.** The CLI prints a one-line notice about Anthropic's terms on
  each run.

## Consequences

The maintainer can use Astrid with their subscription, at the risk of action
on their account. Each turn costs a Node process start, about a second.
Screen tools and effort are rejected on this provider.

Cost of changing later: Low. It is a separate, opt-in provider.
