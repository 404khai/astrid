# ADR 0001: Phase 0 execution contract

Date: 2026-10-06

Status: Accepted by the maintainer; frozen for Phase 0.

## Context

Astrid begins with Phase 0: the smallest real model/tool execution loop.
The repository has no existing implementation. These decisions resolve the
architectural questions raised before implementation and supplement AGENTS.md.
They do not advance the project to another phase.

## Decisions

### Language and package boundary

Use Rust and target macOS only. Start with one Cargo package containing a
library and CLI binary. Introduce a multi-crate workspace only when real
architectural boundaries justify it.

### Provider

Use one OpenAI provider, preferring ChatGPT subscription-backed authentication
over usage-metered API access. Keep authentication separate from the model
provider. Do not use an agent SDK or read, copy, or depend on private Codex
session tokens or undocumented authentication mechanisms. The model identifier
must be configurable rather than architecturally hard-coded.

Implement the documented open-source Sign in with ChatGPT registration flow
and use app-owned OAuth credentials with the public Responses API where
available. If direct integration is unavailable, use a supported subscription-
backed client/backend during development. Do not add a second provider or
design a universal authentication framework in Phase 0.

This supersedes the original API-key-only decision following the maintainer's
explicit change on 2026-10-06. Official registration documentation now describes
`dynamic_agent_client` registration without a partner secret, followed by an
issued app-specific client ID. Actual account access still needs a completed
sign-in and inference request. See [registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in)
and [models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference).
Authentication credential persistence is required by that documented flow;
conversation persistence remains excluded.

Implementation evidence on 2026-10-06: the authorization server rejected an
initial `astrid-<random>` host identifier with `invalid_authorize_request` on
`ext_agent_host_id`. The official [host-ID documentation](https://developers.openai.com/siwc/token-sharing-open-source)
specifies supported identifier formats. Use a persisted UUIDv4 URI
(`urn:uuid:<hyphenated UUID>`) instead. The invalid identifier may be replaced
only before a registration succeeds; registered host identity must remain
stable. This corrects the implementation, without changing the provider or
authentication policy. Direct sign-in and live inference were subsequently verified; see the live
streaming compatibility evidence below.

### Conversation lifetime

Expose `astrid run "<task>"`. Each process handles one fresh task. Do not
implement an interactive follow-up loop or conversation persistence.

### Execution authority and workspace

The invocation directory is the workspace root, including when it is inside
a larger repository. Reads, searches, and workspace-local file writes and
edits may execute automatically. Paths outside that workspace are denied.
Shell commands require explicit confirmation. Test Phase 0 in disposable
repositories.

### Shell execution

Use noninteractive command strings through one fixed shell. Each command
starts fresh in the workspace directory. Capture stdout, stderr, and exit
status. Shell state does not persist between calls.

Provide a simple configurable command timeout as an explicit Phase 0
exception. Advanced cancellation remains deferred to Phase 2.

### Streaming and tool dispatch

Stream assistant text immediately. Buffer tool calls and execute them only
after the provider reports successful response completion. Execute multiple
tool calls sequentially. Partial model responses must never cause side
effects.

### Loop termination and failures

Return recoverable tool failures to the model as structured tool results.
Provider and protocol failures terminate the run cleanly. Never automatically
retry mutating operations.

A successful assistant response containing no tool calls ends the run.
Use an initial configurable ceiling of 20 model calls.

### File mutations

Keep `write_file` and `edit_file` distinct. `edit_file` performs exact text
replacement and succeeds only when the target text occurs exactly once.
Zero or multiple matches fail without mutation. Defer unified patch
application.

### Repository instructions

Use a small built-in Astrid system prompt plus `AGENTS.md` from the invocation
directory when present. Do not discover parent or nested instruction files.

### Acceptance and deterministic validation

Create a tiny deterministic fixture repository with a failing test. Astrid
must inspect it, search and read files, edit an existing file, create one
required file, run tests, inspect their result, and terminate normally.

Separately provide mocked protocol/runtime tests covering:

- fragmented streaming and tool calls;
- tool failures;
- interrupted responses;
- edit ambiguity;
- path escape;
- shell rejection;
- the model-call ceiling.

Critical runtime tests must not depend on a live model API. The live
acceptance demonstration exercises the real provider integration separately.

## Consequences and scope

Phase 0 includes workspace path enforcement and a fixed shell confirmation
boundary to satisfy the selected execution authority. This does not authorize
a configurable permission framework, process sandbox, or other Phase 2 work.
Starting a shell in the workspace directory is not itself process confinement.

The initial implementation remains sequential, has no persisted conversation,
and treats provider failures as terminal. Runtime dispatch and termination
must be controlled by code rather than instructions embedded in the prompt.

All other phase exclusions in AGENTS.md remain in force. Reversible local
implementation choices can be resolved within this contract without changing
these decisions.

## Change procedure

Do not reopen these decisions without implementation evidence that a change
is necessary. Before proceeding with a materially changed decision, record
the evidence, changed decision, and rationale in this document or a
superseding ADR. Decisions requiring maintainer resolution follow the
decision-question procedure in AGENTS.md.

## Live streaming compatibility evidence

A live ChatGPT-backed request on 2026-10-06 returned HTTP success and a
valid SSE body without a Content-Type header. Requiring that header rejected
the response before parsing. Astrid now requests SSE explicitly and permits
an absent Content-Type, while rejecting an explicitly different media type.
The event decoder and successful response.completed requirement remain
mandatory; interrupted responses cannot dispatch tools. This adjusts transport
validation without changing the frozen execution authority or completion rule.

The same live stream also returned an empty terminal output array after
streaming text and completed output items. Astrid assembles ordered
response.output_item.done items when that array is empty, validates them with
the same completed-output parser, and releases them only after successful
response.completed. Missing indices, duplicate items, mismatched text or
arguments, and interruption still fail closed. Continuation retains the
assembled items for subsequent model calls.

Validation: the live greeting fixture completed normally with gpt-5.6-luna
in six model calls and ten tool calls. It inspected files, observed the initial
two failing tests, edited the existing function, created GREETING.txt, and
observed two passing tests. Each cargo command received explicit confirmation.
The offline suite also covers both compatibility cases and interrupted
streams with completed items, ensuring those items alone cannot mutate files.
