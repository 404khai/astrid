// ADR 0032: one bounded Claude Agent SDK turn per Astrid model call.
// Reads one JSON request on stdin; writes JSON lines on stdout:
//   {"type":"delta","text":...}  streamed assistant text
//   {"type":"done","text":...,"tool_calls":[{id,name,arguments}],"session_id":...}
//   {"type":"error","message":...}
// Claude Code's built-in tools are disabled. Astrid's tools are declared via an
// in-process MCP server so the model can call them, but every call is denied
// here and returned to Astrid, whose runtime runs it under its own approvals.
import { query, createSdkMcpServer, tool } from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";

const SERVER = "astrid";
const PREFIX = `mcp__${SERVER}__`;
const emit = (value) => process.stdout.write(JSON.stringify(value) + "\n");

async function readRequest() {
  let raw = "";
  for await (const chunk of process.stdin) {
    raw += chunk;
    if (raw.length > 8 * 1024 * 1024) throw new Error("request exceeds 8 MiB");
  }
  return JSON.parse(raw);
}

/** Converts one Astrid JSON Schema object into the zod raw shape tool() takes. */
export function shape(parameters) {
  const out = {};
  const required = new Set(parameters?.required ?? []);
  for (const [name, schema] of Object.entries(parameters?.properties ?? {})) {
    const field = z.fromJSONSchema(schema);
    out[name] = required.has(name) ? field : field.optional();
  }
  return out;
}

async function main() {
  const request = await readRequest();
  const calls = [];
  const server = createSdkMcpServer({
    name: SERVER,
    version: "1.0.0",
    tools: request.tools.map((t) =>
      tool(t.name, t.description ?? "", shape(t.parameters), async () => ({
        // Never reached: canUseTool denies every call first.
        content: [{ type: "text", text: "Astrid executes this tool." }],
        isError: true,
      })),
    ),
  });
  let text = "";
  let sessionId = null;
  const stream = query({
    prompt: request.prompt,
    options: {
      model: request.model,
      systemPrompt: request.system,
      tools: [], // no built-in Claude Code tools
      mcpServers: { [SERVER]: server },
      allowedTools: [],
      settingSources: [], // ignore user/project CLAUDE.md and settings
      maxTurns: 1,
      includePartialMessages: true,
      cwd: request.cwd,
      env: { ...process.env, CLAUDE_AGENT_SDK_CLIENT_APP: request.client ?? "astrid" },
      canUseTool: async (name, input, { toolUseID }) => {
        if (name.startsWith(PREFIX)) {
          calls.push({ id: toolUseID, name: name.slice(PREFIX.length), arguments: JSON.stringify(input) });
        }
        return { behavior: "deny", message: "Handled by Astrid.", interrupt: true };
      },
    },
  });
  for await (const message of stream) {
    sessionId = message.session_id ?? sessionId;
    if (message.type === "stream_event") {
      const e = message.event;
      if (e.type === "content_block_delta" && e.delta?.type === "text_delta") {
        text += e.delta.text;
        emit({ type: "delta", text: e.delta.text });
      }
    } else if (message.type === "assistant" && message.error) {
      emit({ type: "error", message: `Claude Code: ${message.error}` });
      return;
    } else if (message.type === "result") {
      if (message.subtype !== "success" && calls.length === 0) {
        emit({ type: "error", message: `Claude Code turn ended: ${message.subtype}` });
        return;
      }
      break;
    }
  }
  emit({ type: "done", text, tool_calls: calls, session_id: sessionId });
}

main().catch((error) => {
  emit({ type: "error", message: String(error?.message ?? error).slice(0, 2000) });
  process.exitCode = 1;
});
