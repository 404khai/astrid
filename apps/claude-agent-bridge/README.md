# Claude agent bridge (ADR 0032)

Lets `astrid run --model claude-code/<model>` use the Claude Agent SDK, on
whatever account Claude Code is logged into. One bounded turn per Astrid model
call: Claude Code's built-in tools are off, and Astrid's tools come back to
Astrid to execute.

```sh
cd apps/claude-agent-bridge && npm install   # ~400 MB, bundles Claude Code
claude auth status                           # must be logged in
cd ~/some/folder && astrid run --model claude-code/sonnet "…"
```

Anthropic's terms restrict using claude.ai subscriptions in third-party apps;
see ADR 0032. Override the paths with `ASTRID_NODE` and `ASTRID_CLAUDE_BRIDGE`.
