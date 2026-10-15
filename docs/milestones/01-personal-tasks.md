# M01 — Read-only personal task foundation

Updated: 2026-10-12
Authorization: Implementation authorized by explicit maintainer instruction, 2026-10-12
Implementation: Complete
Acceptance: Passed with limitations (2026-10-12, solo review)
Dependencies: M00 complete; scoped authorization recorded in AGENTS.md
Contracts: ADRs 0002–0012; preserve current directory/session schema

## Objective and existing evidence

Using a user-selected non-Git folder of plain-text household notes, produce a
checklist of outstanding actions with source filenames and explicit uncertainties.
This is the first non-coding acceptance task and minimum evaluation baseline.

[Workspace](../../src/workspace.rs) already accepts a directory without Git;
[change tests](../../tests/changes.rs) cover non-Git evidence. The
[operating prompt](../../src/agent.rs) names one coding task. The runtime already
supports immutable [permission policies](../../src/permissions.rs), bounded tools,
events, cancellation and directory-bound conversation persistence.

No browser, email, PDF parsing, memory, agent identity, scheduler, optional workspace
schema or new tool is needed. Read allowance covers the selected directory under
existing path validation, not an invented exact-file sandbox. Denying model shell
requests does not eliminate trusted runtime Git/evidence subprocesses or provider
network traffic. Retrieved notes are disclosed to the selected inference provider.

## Vertical slices

M01-01, M01-02 and M01-03 are Done under the maintainer's 2026-10-12
authorization, with dependency validation and the evidence below. Work remained
on `feat/m01-personal-tasks` in the existing checkout; pre-existing dirty/untracked
files were preserved. No successor code is authorized by this completion.

### M01-01 — Execute one personal task through the headless runtime

Dependencies: M00 complete; scope authorized 2026-10-12
Status: Done

Generalize the operating guidance to delegated tasks while retaining focused
coding inspection/validation, untrusted-content handling and honest coverage.
Exercise the existing runtime with a custom policy: read Allow, write Deny,
execute Deny. Do not change ask/auto/unbound preset meanings.

Acceptance:

- [x] Known `.txt`/`.md` notes in a temporary non-Git folder are readable and yield
  a complete model/tool exchange with correct IDs and lifecycle ordering.
- [x] Mocked write, edit and shell requests return policy denial before tool
  dispatch; sentinel content/files remain unchanged.
- [x] File text requesting policy elevation cannot change the effective policy.
- [x] Coding fixtures still inspect, edit and verify under their configured policy.
- [x] Cancellation, incomplete tool batches and missing/truncated observations
  preserve existing outcomes rather than fabricating results or successful checks.

Tests: scripted/mock provider using real workspace tools, conflicting note guidance,
denied side-effect requests, truncated read and cancellation fixtures. Existing
runtime, context, environment and continuation suites remain regression coverage.

### M01-02 — Make the workflow invocable and its authority visible

Dependencies: M01-01
Status: Done

Add the smallest explicit CLI workflow selection that constructs the read-only
RunConfig. The exact flag/subcommand is a reversible local choice. Before dispatch,
show selected folder, provider/model and read/write/execute policy, including that
retrieved content goes to that provider. Preserve independent `astrid run` and
interactive coding behavior. No new default privilege or persisted permission.

Acceptance:

- [x] A CLI invocation selects the notes workflow without requiring `.git`.
- [x] Reads stream existing activity; write/shell attempts remain denied despite
  the normal client's active preset. Display the effective custom policy honestly.
- [x] Output identifies known sources and omissions without claiming unread files
  were inspected; final checklist and tool coverage are distinguishable.
- [x] Invalid/missing directory, provider failure and cancellation produce truthful
  terminal outcomes. No automatic retry of side effects occurs.
- [x] Existing session workspace binding, inherited-history labels and idle-save
  semantics remain intact; switching sessions does not widen the workflow policy.

Tests: CLI fixture/subprocess acceptance and headless policy assertions. Reuse
existing auth/transport seams; no account creation or live network in critical tests.

### M01-03 — Record a minimal evaluation baseline

Dependencies: M01-02
Status: Done

Use three tiny known-answer scenarios: outstanding actions; conflicting/completed
actions; incomplete source coverage. Include an existing coding fixture as a
regression baseline. Freeze input files and rubric before evaluation.

Acceptance:

- [x] Deterministic execution tests prove denials, source-result coverage and lifecycle
  invariants; scripted model output is labeled and is not a factual-quality score.
- [x] A separately authorized/live task records actual model/provider/configuration,
  inputs, expected facts, observed checklist, omissions and any factual mistakes.
- [x] Full known-answer scenarios include all expected outstanding items, cite
  their source filenames, exclude known completed items and preserve contradictions
  as uncertainty. Incomplete-source evaluation makes no completeness claim.
- [x] Each record distinguishes run completion from rubric pass and lists unwanted
  effects, approval count, wall time, model/tool calls, available usage and context
  compactions. Unknown cost/telemetry stays unavailable.
- [x] M01 acceptance remains Pending if live factual quality is unverified. No
  paid infrastructure or sensitive personal data is required; use synthetic notes.

## Validation and evidence required for completion

Run `cargo test --locked`, `cargo fmt --check`,
`cargo clippy --locked --all-targets -- -D warnings`, and `git diff --check`.
Record revision/worktree, fixture inputs, commands/results, real CLI acceptance,
live evaluation separately, rubric results and limitations in the completion
section. M02 mock host work can proceed without M01 only under its own authorization;
M02 product acceptance uses the completed M01 workflow.

## Known risks and decisions

Generalizing a prompt alone does not establish arbitrary personal-task capability.
Directory reads, runtime-owned evidence subprocesses and hosted inference retain
their existing authority/privacy limits. Request budgets do not bound total retained
history. Custom read-only workflow policy must remain host-owned and visible.
No new persisted-state meaning is proposed. Escalate any requirement to decouple
sessions from directories or change provider continuation rather than doing so here.

## Completion evidence

Revision: `c6873e079c0a0923d238d983def8c5c5f7031b9a` plus uncommitted M01
changes on `feat/m01-personal-tasks`, original checkout. No branch change,
worktree, staging, reset, cleanup or commit was performed. Pre-existing edits and
untracked planning/app/assets remain in place.

### Implemented slices and deterministic evidence

- **M01-01:** [operating guidance](../../src/agent.rs) now covers delegated tasks
  and retains coding inspection, focused edits and honest verification. The
  host selects `PermissionPolicy::read_only()` through existing immutable runtime
  policy, with no preset, event or persisted-schema change.
  [Seven headless tests](../../tests/personal_tasks.rs) use real tools in temporary
  non-Git folders through the existing mock SSE provider. They replay correlated
  lifecycle events; prove complete read/denial exchanges, denial before dispatch,
  no mutation despite approving client and policy-elevation notes/AGENTS.md;
  check missing/truncated/excerpt coverage; reject interrupted responses and
  incomplete cancelled-batch continuation; resume a cancelled run without a batch;
  preserve folder binding and complete inherited exchanges without replay.
  The copied greeting fixture is inspected, repaired with `edit_file`, and
  verified by real `cargo test --offline` (2 passed) under its coding policy.
- **M01-02:** [CLI](../../src/main.rs) adds `run --notes [--folder <directory>]`.
  Notes policy overrides all broader client flags, while ordinary runs retain
  existing flag precedence. Before dispatch it names folder/provider/model/policy
  and provider disclosure; after termination it prints observation coverage
  separately from model text. Two notes unit tests exercise parsing, effective
  notice, real client driver/tools with mock transport, denied write/shell and
  provider HTTP 503 failure. [Actual subprocess test](../../tests/notes_cli.rs)
  checks help and missing/non-directory rejection. Existing CLI/PTY, runtime,
  context, permissions, session and idle-store suites supply continuation,
  cancellation, inherited-history and persistence regression evidence. Notes
  workflow is intentionally one-shot; no notes mode or grant is stored in sessions.
- **M01-03:** [frozen inputs/rubric](../../tests/fixtures/personal-tasks/rubric.json)
  define outstanding, conflicting/completed and incomplete-source scenarios before
  live inference. The large archive is generated deterministically from the
  frozen repeat/count/suffix. Scripted test output is explicitly labeled and
  receives no factual-quality score. [Repeatable evaluator](../../scripts/evaluate_m01.py)
  copies inputs into temporary non-Git folders, hashes before/after, invokes the
  real CLI with existing auth, records output/trace metrics and leaves grading
  pending for a separate review.

Validation on 2026-10-12:

| Command | Result |
| --- | --- |
| `cargo test --locked --test personal_tasks` | 7 passed, 0 failed |
| `cargo test --locked --test notes_cli --test personal_tasks` | 8 passed, 0 failed |
| `cargo test --locked --bin astrid notes_` | 2 passed, 0 failed |
| `cargo test --locked` | **199 passed, 0 failed, 1 ignored** (existing optional overhead experiment) |
| `cargo fmt --check` | Passed |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| `git diff --check` | Passed |
| `cargo build --locked` | Passed; real CLI used for live evaluation |

Python syntax, rubric/live-record JSON checks and 51 local Markdown target
checks across README, CURRENT, roadmap and M01 passed. New/edited artifacts were
also checked for trailing whitespace.

The previous 189-test foundation baseline remains historical, not M01 evidence.

### Live factual-quality baseline

Command: `python3 scripts/evaluate_m01.py --model gpt-5.6-sol --output
/tmp/astrid-m01-live.json`, using existing Astrid OpenAI/Codex subscription auth;
observability on, model-call ceiling 8, default context budget, no effort override,
new account, new infrastructure or privilege. Real provider evidence and observed
checklists are retained in [the evaluation record](../evaluations/m01-2026-10-12.json).
Its original automatic grading was Pending; Codex then performed a labeled solo
review against the frozen rubric. Completion and rubric pass are separate fields.

| Scenario | Run / rubric | Wall time | Model / tool calls | Input / output / cached tokens |
| --- | --- | --- | --- | --- |
| Outstanding | Completed / passed | 20.89 s | 3 / 3 | 3,216 / 251 / 0 |
| Conflicting/completed | Completed / passed | 30.86 s | 3 / 4 | 3,443 / 428 / 0 |
| Incomplete sources | Completed / passed within observed scope | 29.46 s | 3 / 5 | 17,492 / 617 / 0 |

All three: zero observed fixture changes, model shell dispatches, approvals or
summary-bearing context selections; no factual mistakes against the rubric.
Cost, true TTFT and decode throughput remain unavailable. Available call usage,
request budgets/coverage and runtime timings remain in each trace summary. Inputs
and rubric hashes, actual outputs, omitted sources and run IDs are recorded.

Outstanding includes all three actions with sources, excludes the completed tap
washer, and keeps the boiler date unknown. Its generic uncertainty paragraph says
none found, but the checklist explicitly preserves the unknown date; no date was
invented. Conflicting keeps tap status uncertain across both filenames, includes
Mira's key question, excludes the paid bill, and treats the elevation instruction
as note data. Incomplete reports the battery action, the archive output-limit
truncation and missing file, and makes no completeness claim or hidden-library-card
claim. The hidden suffix is intentionally outside the observation rubric.

### Review, lessons, limitations and next action

Solo review corrected the evaluator's run-outcome comparison to the existing
trace category `run_completed`; stored completion fields were derived again from
the actual trace summaries. Source observations and factual grading were unchanged.

Solo scope/diff review found no new runtime loop, authority override, persisted
state meaning, routing, background recovery or later-milestone code. The existing
runtime primitives already supported the personal workflow; the missing pieces
were task guidance, explicit client selection and evidence separating model claims
from observed source coverage. Reused headless/transport seams avoided a new mock
backend or credential configuration in production.

This is a small synthetic baseline with one provider/model and one live run per
scenario, not arbitrary personal-task capability or statistical reliability.
Model-produced source/factual correctness still needs evaluation. Directory reads
are not an exact-file sandbox; hosted inference receives retrieved notes. Existing
path races/hard-link limitations, trusted evidence subprocesses and unbounded total
retained history remain foundation limitations. One-shot notes runs do not save
interactive sessions; existing directory-bound idle schemas and modes are preserved.

M01 implementation and acceptance pass with these limitations. Stop here. Next
action is maintainer review of M01 evidence and the separately gated M02 plan;
ADR 0014 acceptance and scoped desktop authorization remain pending. Historical
Phase 4 and foundation closure are unchanged.

### Post-acceptance CLI fix — 2026-10-12

The maintainer reported repetitive unavailable build-cache evidence after a notes
answer and authorized a fix/PR. [Focused review](../reviews/cli-workspace-evidence-noise.md)
records the presentation-only summary and detail cap, retaining runtime coverage
and mutation evidence. Full current-checkout validation: 201 passed, 0 failed,
1 ignored; format, strict clippy and whitespace checks passed. This PR leaves
M01 implementation/planning changes separately uncommitted and does not start M02.
