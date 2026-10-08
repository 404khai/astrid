# AGENTS.md — Astrid

## 1. Project

Astrid is an experimental, inference-aware AI agent harness.

The primary purpose of Astrid is not to compete feature-for-feature with mature coding agents. It exists to explore and implement the systems underneath capable agents:

- agent execution loops
- model/provider abstraction
- tool execution
- context management
- inference observability
- local and remote inference
- scheduling
- model routing
- failure recovery
- evaluation
- eventually multi-agent execution

Coding repositories are Astrid's first environment, not necessarily its final domain.

The project should prioritize **learning, correctness, observability, and architectural clarity** over feature count.

---

# 2. Core Philosophy

Astrid should make normally invisible agent-runtime behavior visible.

A user should eventually be able to answer questions such as:

- What is the agent doing right now?
- Why did it choose this tool?
- What information is currently in context?
- What was removed from context?
- How much of the prompt was cached?
- Where was time spent?
- How much did each model invocation cost?
- Was latency dominated by inference or tools?
- Which files influenced the run?
- Why was a particular model selected?
- Why did a run fail?
- Could a smaller/local model have completed this step?

Astrid should therefore be designed around:

1. **structured execution**
2. **explicit state**
3. **instrumentation**
4. **reproducibility**
5. **replaceable components**

Do not hide important runtime behavior behind opaque abstractions.

---

# 3. Non-Goals

Until explicitly introduced by a later phase, Astrid is NOT:

- an IDE
- a VS Code extension
- a desktop application
- an agent marketplace
- a hosted SaaS platform
- a collaboration platform
- a generic chatbot
- a multi-agent swarm framework
- an autonomous software company
- a replacement for every existing coding agent
- a GUI-first product

Do not add features merely because existing coding agents have them.

Every major feature must support the central goal of understanding or improving agent execution.

---

# 4. Engineering Principles

## 4.1 Build primitives before conveniences

Prefer implementing:

- execution state
- event streams
- context accounting
- cancellation
- tool boundaries
- tracing

before:

- aliases
- themes
- dashboards
- plugins
- convenience commands

---

## 4.2 Explicit state over hidden state

Important state should have concrete representations.

Examples:

- `RunId`
- `SessionId`
- `TurnId`
- `ToolCallId`
- `ModelCallId`
- `ContextSnapshot`
- `ExecutionEvent`

Avoid designs where critical runtime state lives only inside a prompt string.

---

## 4.3 Events are first-class

Important runtime behavior should emit structured events.

Example event families:

```text
RunStarted
RunCompleted
RunFailed

TurnStarted
TurnCompleted

ModelRequestStarted
ModelFirstToken
ModelRequestCompleted

ToolCallRequested
ToolCallStarted
ToolCallCompleted
ToolCallFailed

ContextItemAdded
ContextItemEvicted
ContextCompacted

PermissionRequested
PermissionGranted
PermissionDenied
```

Exact names may change.

The principle should not.

---

## 4.4 Instrument before optimizing

Do not claim an optimization improves Astrid without measurements.

Measure before and after whenever practical.

Relevant measurements may include:

- total latency
- time to first token
- prefill latency
- decode latency
- output tokens/sec
- prompt tokens
- generated tokens
- cached tokens
- tool latency
- context size
- compaction frequency
- cost
- task success rate

---

## 4.5 Correctness before autonomy

Astrid should become more autonomous only when its execution model can be reasoned about.

Prefer:

```text
simple + observable + reliable
```

over:

```text
autonomous + complicated + mysterious
```

---

# 5. Architecture Direction

Astrid should evolve toward roughly these boundaries:

```text
astrid/
├── crates/
│   ├── astrid-core/
│   ├── astrid-agent/
│   ├── astrid-models/
│   ├── astrid-tools/
│   ├── astrid-context/
│   ├── astrid-runtime/
│   ├── astrid-trace/
│   └── astrid-cli/
│
├── docs/
├── examples/
├── tests/
└── AGENTS.md
```

This structure is directional rather than mandatory.

Do not create crates simply to match this diagram.

A new crate should exist only when a real architectural boundary has emerged.

Early phases may intentionally use fewer crates.

---

# 6. Core Domain Model

Astrid should eventually have explicit concepts for:

```text
Agent
Run
Session
Turn

Message
ContentBlock

Model
ModelProvider
ModelRequest
ModelResponse
ModelStream

Tool
ToolCall
ToolResult

Context
ContextItem
ContextBudget
ContextSnapshot

ExecutionEvent
Trace

Permission
```

Later phases may add:

```text
Task
Worker
Scheduler
ModelRoute
Checkpoint
Worktree
Evaluation
```

Do not introduce later-phase concepts prematurely.

---

# 7. Provider Boundary

Model providers must eventually sit behind a common Astrid abstraction.

Conceptually:

```rust
trait ModelProvider {
    async fn generate(
        &self,
        request: ModelRequest,
    ) -> Result<ModelStream>;
}
```

The actual interface should be derived from real requirements rather than copied blindly from this example.

Provider-specific behavior must not leak unnecessarily throughout the runtime.

At the same time, the common abstraction must not erase useful provider capabilities.

Astrid should eventually support capability discovery such as:

```text
streaming
tool_use
structured_output
prompt_caching
reasoning
vision
max_context
```

Avoid designing the universal provider abstraction before at least two meaningfully different providers exist.

---

# 8. Tool Boundary

Tools should be explicit typed capabilities.

Initial tool set:

```text
read_file
write_file
edit_file
list_directory
glob
grep
shell
```

Tools should return structured results where practical.

Every tool invocation must be observable.

Tools must not silently mutate unrelated state.

Potentially destructive actions should pass through a permission boundary.

---

# 9. Safety and Permissions

The runtime must distinguish between:

```text
read-only operations
workspace mutations
external side effects
potentially destructive operations
```

Astrid should eventually support permission policies such as:

```text
allow
ask
deny
```

Do not make permanent destructive actions implicit.

Shell execution must eventually support:

- timeout
- cancellation
- output limits
- working-directory isolation
- exit-status capture
- stdout/stderr separation

---

# 10. Error Handling

Do not use panics for ordinary runtime failures.

Errors should retain enough context to answer:

```text
what failed?
where?
during which run?
during which model/tool call?
is retry possible?
```

Prefer typed error categories where they improve decisions.

Do not create a giant error taxonomy prematurely.

---

# 11. Testing

Tests should focus heavily on runtime invariants.

Important areas include:

- event ordering
- tool-call lifecycle
- provider parsing
- cancellation
- context accounting
- persistence
- retries
- permission enforcement

Prefer deterministic tests.

Model APIs should be mockable.

Critical runtime tests should not depend on live network APIs.

---

# 12. Documentation

Architecture decisions that constrain future phases should be documented.

Use ADRs for decisions that are:

- architecturally significant
- difficult to reverse
- likely to be questioned later
- based on meaningful trade-offs

Example:

```text
docs/
├── architecture.md
├── concepts.md
└── adr/
    ├── 0001-agent-execution-model.md
    └── 0002-runtime-event-model.md
```

Do not create ADRs for trivial implementation details.

---

# 13. Phase Discipline

Astrid is deliberately phased.

Agents MUST NOT implement features assigned to later phases unless the user explicitly changes the roadmap.

When implementing a phase:

1. inspect the current repository
2. determine what already exists
3. identify the smallest vertical slice
4. implement it
5. test it
6. document architectural decisions when necessary
7. stop

Do not "helpfully" begin the next phase.

---

# Phase 0 — The Loop

## Goal

Build the smallest real harness.

Astrid must be able to:

```text
user request
     ↓
model
     ↓
tool request
     ↓
tool execution
     ↓
tool result
     ↓
model
     ↓
...
     ↓
final response
```

## Deliverables

Implement:

- basic CLI, logo is available to get the pixelated version for the cli
- one model provider
- streaming output
- message representation
- model request/response representation
- tool-call representation
- agent loop
- file reading
- file writing/editing
- file search
- shell execution
- graceful model/tool errors

Example interface:

```bash
astrid
```

or:

```bash
astrid run "find and fix the failing test"
```

## Constraints

Do NOT implement:

- subagents
- MCP
- worktrees
- RAG
- embeddings
- semantic repository indexing
- GUI
- TUI dashboard
- automatic model routing
- complex planning systems

## Exit Criteria

Astrid can receive a small repository task and autonomously cycle through:

```text
reason
inspect
edit
execute
inspect
finish
```

without runtime-specific logic being hardcoded into the prompt.

---

# Phase 1 — Runtime Model

## Goal

Turn the prototype loop into an explicit runtime.

## Deliverables

Introduce stable concepts for:

- run
- session
- turn
- model call
- tool call
- execution state

Introduce unique identifiers where useful.

Define runtime events.

Implement cancellation.

Implement clean propagation of failures.

Separate:

```text
agent policy
runtime execution
model provider
tool execution
```

## Event Stream

A run should produce enough structured data that its execution can later be reconstructed.

Example:

```text
RunStarted
TurnStarted
ModelCallStarted
ModelFirstToken
ModelCallCompleted
ToolCallStarted
ToolCallCompleted
TurnCompleted
RunCompleted
```

## Exit Criteria

The CLI is primarily a consumer of the runtime rather than the runtime itself.

A second UI could theoretically consume the same runtime without rewriting agent execution.

---

# Phase 2 — Execution Environment

## Goal

Make Astrid safe and reliable enough to work inside real repositories.

## Deliverables

Implement:

- command timeouts
- cancellation propagation
- stdout streaming
- stderr streaming
- exit-code capture
- output truncation policies
- workspace boundaries
- path validation
- configurable permissions
- git awareness
- patch/diff visibility

Consider:

```text
read
write
execute
network
destructive
```

as capability classes.

## Git Integration

Astrid should understand enough Git state to expose:

- current branch
- dirty files
- diffs caused by the run

Do not automatically build a sophisticated Git workflow yet.

## Exit Criteria

A user can safely let Astrid modify a repository while retaining visibility into what changed and what commands were executed.

---

# Phase 3 — Context Engine

## Goal

Make context a deliberate runtime subsystem rather than an ever-growing message array.

## Deliverables

Introduce:

```text
ContextItem
ContextSource
ContextBudget
ContextSnapshot
ContextSelection
```

Every context item should ideally retain provenance.

Examples:

```text
conversation
file
tool result
repository metadata
system instruction
summary
```

Implement:

- token estimation/counting
- context budget
- message pruning
- compaction
- file relevance heuristics
- context inspection

Possible command:

```bash
astrid context
```

Possible output:

```text
ACTIVE CONTEXT

system                     812 tokens
conversation              2,148
src/runtime.rs            1,483
src/tools.rs                931
cargo test result            87

TOTAL                     5,461 / 16,384
```

## Important Requirement

Record why context was:

```text
added
retained
compacted
evicted
```

where practical.

## Experiments

Begin comparing strategies such as:

- recency
- file references
- lexical relevance
- repository dependency information
- tool-use history

Do not prematurely build vector search.

## Exit Criteria

Astrid can inspect, budget, compact, and explain its working context.

---

# Phase 4 — Trace and Inference Observability

## Goal

Make Astrid an inference-aware harness.

This phase establishes a major part of Astrid's identity.

## Deliverables

Capture where providers expose enough information:

- prompt tokens
- completion tokens
- cached tokens
- time to first token
- total model latency
- token generation rate
- tool execution time
- context utilization
- provider/model
- monetary cost when calculable

Local backends may additionally expose:

- prefill throughput
- decode throughput
- model loading time
- memory usage
- KV-cache characteristics

Create persistent traces.

Possible commands:

```bash
astrid trace
astrid trace <run-id>
astrid stats
```

## Trace Representation

A trace should allow reconstruction similar to:

```text
run
├── model call
│   ├── prefill
│   └── decode
├── tool: grep
├── tool: read
├── model call
├── tool: edit
├── tool: shell
└── model call
```

## Constraint

Do not fabricate metrics unavailable from a provider.

Missing telemetry must be represented as unavailable.

## Exit Criteria

A developer can inspect a completed run and determine where context, tokens, time, tools, and model calls were spent.

---

# Phase 5 — Local Inference

## Goal

Treat local inference as a first-class execution target rather than a compatibility afterthought.

## Deliverables

Add one practical local backend.

Possible initial targets include whichever backend best fits the environment at implementation time.

Do not build multiple integrations simultaneously.

Measure:

```text
TTFT
prefill throughput
decode throughput
memory
context size
```

Compare local and hosted models using the same Astrid task/trace representation.

## Architecture Requirement

The rest of Astrid must not need to know whether inference is:

```text
remote API
local server
embedded runtime
```

except where backend capabilities genuinely differ.

## Exit Criteria

The same Astrid run model can operate using both a hosted provider and a local inference backend.

---

# Phase 6 — Evaluation

## Goal

Stop evaluating changes based only on whether demonstrations look impressive.

## Deliverables

Create a small reproducible task suite.

Tasks should measure capabilities such as:

- repository navigation
- targeted bug fixing
- localized implementation
- test repair
- multi-file reasoning
- context retrieval

Capture:

```text
success/failure
wall-clock time
model calls
tool calls
input tokens
output tokens
cached tokens
cost
context compactions
```

Avoid building a giant benchmark suite.

Start with tasks that are easy to inspect manually.

## Exit Criteria

Changes to context policy, models, prompts, and agent policy can be compared empirically.

---

# Phase 7 — Model Routing

## Goal

Explore heterogeneous model execution.

Astrid should be able to choose different models for different classes of work.

Possible roles:

```text
cheap/local model
    ↓
classification / retrieval / summarization

coding model
    ↓
implementation

strong reasoning model
    ↓
hard planning / diagnosis
```

## Deliverables

Introduce:

```text
ModelCapability
ModelRoute
RoutingPolicy
```

Routing decisions must be visible in traces.

Record:

```text
which model was selected
why it was selected
what the call cost
how long it took
whether the step succeeded
```

## Initial Policies

Begin deterministic.

Example:

```text
task type → configured model
```

Do NOT begin with an ML-based router.

## Exit Criteria

Astrid can execute one run using multiple model classes under an observable routing policy.

---

# Phase 8 — Scheduler and Workers

## Goal

Move from one sequential agent loop toward structured parallel work.

## Deliverables

Introduce concepts only when needed:

```text
Task
TaskId
Dependency
Worker
WorkerId
Lease
TaskState
```

Support a task DAG.

Example:

```text
                ┌─ inspect API ──┐
plan ───────────┤                ├─ integrate ─ test
                └─ inspect UI ───┘
```

Workers should not silently share mutable conversational state.

Define explicit communication.

## Constraint

Do not call every model invocation a "subagent."

A worker should represent a meaningful independent unit of execution.

## Exit Criteria

Two independent tasks can execute concurrently and their outputs can be safely reconciled.

---

# Phase 9 — Git Worktree Isolation

## Goal

Allow concurrent coding workers to modify repositories without colliding.

## Deliverables

Explore:

```text
worker
   ↓
task
   ↓
isolated worktree
   ↓
changes
```

Implement:

- worktree creation
- lifecycle management
- task ownership
- diff collection
- cleanup
- conflict reporting

Do not automatically merge ambiguous conflicts.

## Exit Criteria

Parallel workers can independently change the same repository without sharing a working tree.

---

# Phase 10 — MCP and External Tools

## Goal

Allow Astrid to consume external capabilities through a standardized protocol.

## Deliverables

Implement MCP support for appropriate:

- tools
- resources
- prompts if useful

Do not make MCP semantics infect Astrid's internal tool model.

Use an adapter boundary:

```text
MCP
 ↓
Astrid Tool
 ↓
Agent Runtime
```

## Exit Criteria

An MCP tool can participate in the same execution and tracing model as a native Astrid tool.

---

# Phase 11 — Astrid Interface

## Goal

Build the interface warranted by everything learned so far.

This should initially remain terminal-first.

Potential TUI regions:

```text
┌──────────────────────────────────────────────┐
│ Astrid                           run 82ac1   │
├──────────────────────────────────────────────┤
│                                              │
│ conversation / execution                    │
│                                              │
├───────────────────────┬──────────────────────┤
│ context               │ runtime              │
│ 9.2k / 32k            │ model qwen...        │
│ cache 71%             │ TTFT 420ms           │
│                       │ decode 46 tok/s       │
├───────────────────────┴──────────────────────┤
│ >                                            │
└──────────────────────────────────────────────┘
```

The UI should visualize existing runtime primitives.

Do not invent runtime concepts purely to make the UI visually impressive.

---

# Phase 12 — Experimental Runtime Research

## Goal

Use Astrid itself as an environment for experiments.

Potential research directions:

### Context caching

Treat agent context as a cache hierarchy:

```text
L0 current task
L1 active files
L2 summaries
L3 repository knowledge
L4 external retrieval
```

Compare eviction/promotion policies.

### Adaptive context

Experiment with dynamically deciding:

```text
what to retrieve
what to preserve
what to summarize
what to discard
```

### Inference scheduling

Explore scheduling based on:

```text
latency
cost
model capability
context length
local resource availability
```

### Speculative agent execution

Investigate whether certain independent tool actions can safely execute ahead of model demand.

### Cache-aware routing

Consider whether prompt-prefix reuse should influence model/provider selection.

### Failure recovery

Experiment with checkpoints and resumable agent runs.

Every experiment should define:

```text
hypothesis
measurement
baseline
result
```

---

# Phase 13 — Phalanx Integration

## Goal

Integrate Astrid with a custom inference runtime.

Phalanx should appear behind Astrid's model-provider boundary.

Conceptually:

```text
Astrid
   │
   ├── Hosted Provider
   │
   ├── Local Provider
   │
   └── Phalanx
          │
        Model
```

Astrid must not receive special-case architectural damage solely to accommodate Phalanx.

If the provider abstraction cannot represent Phalanx cleanly, determine whether:

1. the abstraction is insufficient, or
2. Phalanx exposes genuinely different semantics.

Document the decision.

## Exit Criteria

Astrid can execute a model through Phalanx while receiving whatever inference telemetry Phalanx makes available.

---

# 14. Agent Working Procedure

Use [docs/development.md](docs/development.md) for the repeatable phase-review,
grilling, ticket, and ADR workflow. Proposed plans and ADRs do not authorize a
phase transition; the current phase below remains authoritative.

Before implementing substantial work:

1. Read this file.
2. Determine the active phase.
3. Inspect relevant existing code.
4. Identify architectural decisions already recorded.
5. Do not assume the roadmap is implementation truth if the repository has evolved beyond it.
6. Prefer the smallest complete vertical slice.
7. Add or update tests.
8. Run relevant validation.
9. Report what changed.
10. Stop when the requested task is complete.

---

# 15. Decision Questions

When an implementation exposes an unresolved architectural question that will significantly affect future work:

DO NOT silently choose the easiest answer.

Instead provide:

```text
Context:
What prompted the decision.

Options:
The meaningful alternatives.

Recommendation:
The recommended option and why.

Consequences:
What this decision enables or constrains.

Decision question:
The smallest question the user needs to resolve.
```

Do not interrupt implementation for trivial decisions.

Prefer resolving reversible local choices independently.

---

# 16. Phase Review

Before beginning a new phase, review the previous phase.

Ask:

```text
What did we learn?

Which assumptions were wrong?

Which abstractions proved useful?

Which abstractions are premature?

What technical debt would distort the next phase?

What should explicitly NOT be carried forward?
```

The roadmap may be changed based on evidence.

It is not sacred.

---

# 17. Current Phase

Current phase:

```text
Phase 3 — Context Engine
```

Phase 3 was authorized by the maintainer on 2026-10-07 with the instruction to
critique its plan using an adversarial model, then begin implementation. This
authorization does not resolve the open design questions in docs/plans/phase-3.md
or supply a separate Phase 2 closure decision.

The maintainer subsequently accepted both recommended context contracts on
2026-10-07: estimated admission plus a byte ceiling and whole-exchange selection
(ADR 0006), and deterministic incomplete compaction (ADR 0007). The previous
phase's separate closure disposition remains pending.

Unless the user explicitly changes this value, implementation should remain within Phase 3.

---

# 18. Definition of Success

Astrid succeeds if building it produces a deeper understanding of:

```text
agent runtimes
LLM inference
context engineering
tool execution
observability
scheduling
local inference
evaluation
```

A smaller system whose behavior is deeply understood is preferable to a larger system assembled from abstractions the maintainer cannot explain.

# Maintainer Autonomy Protocol

Astrid is primarily developed through an AI coding agent.

The maintainer should not be required to answer routine implementation
questions or repeatedly restate architectural context.

The agent is expected to inspect the repository, existing ADRs, tests,
Git history where useful, and the current roadmap before asking the
maintainer for a decision.

## Default decision policy

Resolve a decision independently when it is:

- local to one module,
- easily reversible,
- covered by an existing ADR or project principle,
- an implementation detail rather than a product decision,
- naturally implied by existing types/tests,
- or unlikely to constrain a later phase.

For these decisions:

1. choose the simplest defensible option,
2. implement it,
3. test it,
4. briefly document it when useful.

Do not interrupt the maintainer.

## Escalation policy

Ask the maintainer only when a decision:

- materially changes Astrid's product direction,
- creates a difficult-to-reverse public contract,
- changes security/permission semantics,
- changes the meaning of persisted state,
- changes cross-device trust semantics,
- introduces a major external dependency,
- conflicts with an existing ADR,
- invalidates a frozen phase decision,
- substantially expands the active phase,
- or has multiple reasonable options with different long-term consequences.

When escalation is necessary, present exactly:

Context:
Why this decision exists.

Existing constraints:
Relevant ADRs, code, tests, or project principles.

Options:
Only materially different options.

Recommendation:
One recommended answer.

Why:
Concrete reasoning.

Cost of changing later:
Low / Medium / High.

Decision required:
One precise question.

Do not ask open-ended questions when a concrete decision can be proposed.

---

# Architectural North Star

Astrid is not ultimately a CLI coding agent.

Astrid is a personal agent runtime capable of coordinating:

- models,
- tools,
- context,
- devices,
- applications,
- local compute,
- remote compute,
- background tasks,
- and user interaction surfaces.

The CLI is the first client of the runtime.

Future clients are expected to include:

- terminal UI,
- Telegram or similar messaging bridges,
- native mobile applications,
- native desktop applications,
- background/daemon processes,
- compact overlay/assistant interfaces,
- and potentially other remote interfaces.

Therefore:

> No core runtime capability should depend on the terminal being present.

The runtime should be usable headlessly.

UI-specific behavior belongs in clients/adapters.

---

# Client / Runtime Boundary

Treat every user interface as a client of Astrid.

Conceptually:

                    ┌── CLI
                    ├── TUI
                    ├── Telegram
                    ├── Mobile

Astrid Runtime ─────┼── Desktop
├── Overlay
└── Remote API

The runtime owns:

- runs,
- sessions,
- turns,
- model execution,
- tool execution,
- context,
- permissions,
- task state,
- cancellation,
- events,
- device capability routing.

Clients own:

- presentation,
- input,
- notifications,
- local interaction conventions,
- rendering permission prompts.

Do not allow terminal concepts such as stdin, ANSI output, terminal
dimensions, or Ctrl-C to leak into core runtime abstractions.

---

# Interaction Abstraction

User interaction should eventually be representable independently of UI.

Examples include:

- text input,
- permission requests,
- choices,
- confirmations,
- notifications,
- progress updates,
- task completion,
- cancellation.

Do not prematurely build a universal UI framework.

However, avoid APIs that assume an interaction must happen through stdin.

For example, prefer a runtime-facing permission interface over:

    read_line_from_terminal()

because permission may later come from:

- CLI,
- phone notification,
- desktop overlay,
- Telegram,
- web client.

---

# Long-Running Task Principle

Astrid's long-term runtime must support:

    submit task
        ↓
    client disconnects
        ↓
    Astrid continues working
        ↓
    result becomes available later

Do not equate client lifetime with run lifetime.

Foreground CLI behavior may choose to cancel when appropriate, but that is
a client policy rather than a core runtime invariant.

---

# Thin Client Principle

Astrid should remain useful on low-resource devices.

Mobile clients must not require frontier-model inference locally.

Expensive work should be delegatable to:

- another user device,
- local desktop compute,
- a remote Astrid node,
- hosted inference,
- or another available execution backend.

Prefer:

- small request payloads,
- streaming,
- resumable sessions,
- background execution,
- capability delegation,
- incremental rendering.

Do not design the core runtime around high-end mobile hardware.

---

# Device Model Direction

Future Astrid nodes may advertise capabilities.

Conceptually:

Device

- id
- type
- connectivity
- capabilities

Example capabilities might eventually include:

    filesystem.read
    filesystem.write
    shell.execute
    git
    browser
    notifications
    camera
    microphone
    clipboard
    android.intent
    gui.observe
    gui.act
    inference.local

Do not implement this model until the active phase requires it.

When introducing it, capabilities should describe what a device can do
rather than hard-coding device classes throughout the runtime.

---

# External Project Evaluation

Astrid may learn from or integrate existing open-source projects.

Do not copy a project wholesale merely because it solves a related problem.

When evaluating an external project, produce:

1. What problem it solves.
2. Which subsystem overlaps Astrid.
3. Which concepts are reusable.
4. Which code could be reused legally.
5. Runtime/language/dependency implications.
6. Whether integration should be:
   - inspiration only,
   - adapted code,
   - library dependency,
   - subprocess,
   - service/RPC backend,
   - protocol adapter,
   - or no integration.
7. What coupling it introduces.
8. Whether building the capability ourselves provides important learning value.

Prefer clean capability boundaries over embedding unrelated frameworks
inside Astrid's core.

Record a formal ADR only when Astrid actually commits to a significant
integration.

---

# Phase Execution Workflow

For every phase:

1. Read AGENTS.md.
2. Read the current phase state.
3. Read relevant ADRs.
4. Inspect the actual implementation.
5. Compare implementation with the phase contract.
6. Identify unresolved blockers.
7. Resolve reversible blockers autonomously.
8. Escalate only qualifying architectural decisions.
9. Implement the smallest coherent vertical slices.
10. Add deterministic tests.
11. Run validation.
12. Update documentation.
13. Perform a phase audit.
14. Do not advance phases until exit criteria are met.

When the phase is complete:

- summarize what actually exists,
- list deviations from the original roadmap,
- list technical debt relevant to the next phase,
- record lessons learned,
- update the current-phase marker,
- then begin the next phase only when instructed or when the repository's
  workflow explicitly permits automatic advancement.

---

# ADR Policy

Create an ADR when a decision is:

- architecturally significant,
- difficult to reverse,
- cross-cutting,
- likely to be questioned later,
- or establishes semantics other components will depend on.

Do not create ADRs for ordinary implementation choices.

Before creating an ADR, search existing ADRs for overlap.

Never create two ADRs describing the same decision.

ADRs should contain:

- Status
- Context
- Decision
- Alternatives considered
- Consequences
- Compatibility/migration implications when relevant

If implementation proves an ADR wrong, supersede it explicitly rather
than silently violating it.

---

# External Research Policy

When evaluating a protocol, framework, SDK, runtime, or open-source
project whose behavior may have changed:

- inspect the current upstream documentation/repository,
- distinguish documented behavior from assumptions,
- record relevant version/commit information where important,
- avoid designing around undocumented behavior.

Research should answer a concrete architectural question.

Do not spend time collecting technologies without an immediate reason.

---

# Learning Constraint

Astrid is also a learning project.

Do not outsource important core concepts to dependencies merely to reduce
implementation work.

Before adding a dependency that implements a major Astrid subsystem,
consider:

> Is understanding or implementing this subsystem part of the project's
> learning objective?

Infrastructure commodities may be reused.

Astrid-defining runtime behavior should generally remain understandable
and owned by this repository.

---

# Scope Discipline

Future requirements should influence interface boundaries, not cause
premature implementation.

Knowing that Astrid will eventually have:

- mobile,
- Telegram,
- desktop,
- GUI automation,
- multiple devices,

does NOT mean those systems should be implemented during the current
phase.

Design today's boundary so tomorrow remains possible.

Do not build tomorrow today.


# Authorized Interactive Session and Permission Extension

On 2026-10-08, the maintainer requested `/sessions`, switchable `ask`, `auto`,
and `unbound` modes, visible active mode, and continued conversation after each
run. This is a scoped extension within Phase 3, not authorization to advance
phases. ADR 0010 extends the earlier single-run session restriction; ADR 0011
records permission presets over ADR 0004's existing boundary.

Required behavior:

- Bare `astrid` returns to input after completed, failed, or cancelled runs.
  Each message is a new run in the selected session, retaining committed history.
- `/new` creates a fresh session; `/sessions` lists and switches sessions in the
  current process. Sessions are ephemeral and workspace-bound. Initial client
  limit: 32 sessions. Request budgets do not bound total retained session memory.
- `astrid run` remains a one-shot interface.
- `/mode` selects a mode; `/mode ask`, `/mode auto`, and `/mode unbound` select it
  directly. The active mode is shown in input and execution status.
- `ask`: reads allowed, writes and shell execution ask.
- `auto` (default): workspace reads/writes allowed, shell execution asks.
- `unbound`: read/write/execute allowed without permission prompts, including
  broad account-level shell authority. Path validation, timeouts, cancellation,
  and output limits still apply. Do not invent shell containment guarantees.
- Mode changes apply to subsequent runs, are client-controlled, and are not
  persisted or reset/elevated by session switching or repository instructions.
- In `unbound`, upper/middle/lower logo arch bands use `#F94447`, `#EC1A1D`,
  `#F94447`; eyes use `#F7C600`. Plain output and `NO_COLOR` remain supported.
- Session-aware execution is a headless runtime API. UI selection, menus,
  styling, and foreground cancellation remain client behavior.
- A cancelled run with unmatched tool requests cannot be continued. Preserve its
  committed state, reject the follow-up explicitly, and direct the user to
  `/new`; never silently replay side effects or manufacture tool results.

Session restoration after process exit, persisted transcripts, background
workers, and checkpoint recovery remain outside this extension.
