# Phase 2: execution environment plan

Status: Implementation complete; acceptance and adversarial review passed
Closure review: [Phase 2 review](../reviews/phase-2.md), 2026-10-07
Baseline: `4b8e93f`, reviewed 2026-10-06
Historical implementation phase: Phase 2, authorized by maintainer on 2026-10-07
Owner: Maintainer; ticket implementers assigned when work starts

Current disposition (2026-10-10): implementation criteria re-audited against
`cdebe18`; administrative closure approved by the maintainer with documented
limitations, conditional on verified criteria (evidenced in the current review).
Phase 4 remains active. Original exclusions and terminal follow-ups below record
the Phase 2 delivery history; later accepted context/trace/session ADRs extend them.
See [current review](../reviews/phase-2.md) and
[foundation audit](../reviews/foundation-closure.md).

## Outcome

A user can run a small task in a real local repository, see tool output while it
runs, stop execution, understand the effective authority, and review changes
against the starting workspace without losing pre-existing work. The accepted
trust model is trusted local repositories with explicit broad shell authority.

Preserve one package/provider, sequential tools, ephemeral sessions, and existing
runtime identities. No context engine, persisted traces, model routing, workers,
worktrees, MCP, new dashboard, or automatic Git workflow. Existing timeouts,
cancellation, and path checks are foundations to extend, not rebuild.

## Adversarial review

An independent adversarial reviewer inspected the plan and implementation on
2026-10-07. See [the findings](../reviews/phase-2-adversarial.md). Implementation
must address the following additions to the acceptance contract:

- Tool execution/cleanup remains polled while lifecycle delivery is stalled.
- Cancellation terminal payloads retain bounded partial output without adding
  cancelled tool results to conversation history.
- Native limits constrain production and inventories, with deterministic bounded
  selection and explicit search coverage for oversized input.
- Git inspection disables optional locks and fsmonitor/helpers; NUL-delimited
  filenames are parsed without lossy collisions. Incomplete inventories cannot
  establish additions/deletions or a clean workspace.
- Raw output bytes require incremental text decoding; live omission and capture
  omission counters describe different facts.

## Grilling decisions (resolved 2026-10-07)

Use [the protocol](../development.md#grilling-protocol), with these three rounds:

| Round | Questions to settle | Recommendation | Decision record |
| --- | --- | --- | --- |
| 1. Authority | Trusted local repos or hostile-code containment? What does shell allow authorize? | Explicit broad shell authority; no command-classification sandbox claims | [ADR 0004](../adr/0004-execution-authority.md), Accepted |
| 2. Output and stopping | Can output be discarded with explicit accounting? Must process cleanup proceed when a consumer stops draining? | Bounded output, omission metadata, cleanup independent of event delivery | [ADR 0003](../adr/0003-bounded-tool-output.md), Accepted |
| 3. Change evidence | What can we prove with a dirty starting tree and arbitrary shell? | Bounded before/after comparison, distinguish observation from attribution | [ADR 0005](../adr/0005-workspace-change-evidence.md), Accepted |

Authority was resolved first because enforced hostile-code containment would
change the entire plan. All three recommended contracts were accepted. Numeric
limits can be adjusted with fixture evidence without reopening the architectural
choice. The actual resolutions are recorded in the ADRs and this table.

## Delivery order and gates

| Ticket | Deliverable | Dependencies | State |
| --- | --- | --- | --- |
| P2-00 | Close Phase 1 review and authorize Phase 2 | Maintainer review | Done |
| P2-01 | Resolve execution-environment contracts | P2-00; ADRs 0003–0005 | Done |
| P2-02 | Bound shell output and preserve partial results | P2-01 | Done |
| P2-03 | Stream bounded tool output through the runtime and CLI | P2-02 | Done |
| P2-04 | Bound native read/search results | P2-01 | Done |
| P2-05 | Configure and explain permission decisions | P2-01 | Done |
| P2-06 | Verify execution and workspace boundaries | P2-03, P2-04, P2-05 | Done |
| P2-07 | Capture read-only Git/workspace baseline | P2-01 | Done |
| P2-08 | Expose bounded run-relative change evidence | P2-06, P2-07 | Done |
| P2-09 | Acceptance, documentation, and Phase 2 review | P2-08 | Done |

Dependencies define prerequisite behavior, not a request for parallel agents.
Recommended single-maintainer order is the table order. First implementation
slice is P2-02: one noisy shell command with bounded results and honest timeout
output. P2-03 then carries those bytes through events to the CLI.

P2-00 and P2-01 are planning/governance exceptions to the implementation Ready
gate: discussion and ADR resolution preceded Phase 2 code. Authorization and
all three decisions were recorded on 2026-10-07 before dependent implementation.

## P2-00: Close Phase 1 and record the phase boundary

Problem: implementation is complete, but completion and next-phase scope should
be evidenced separately. Review [Phase 1 findings](../reviews/phase-1.md).

Acceptance:

- [x] Maintainer reviews exit-criteria evidence and records close/hold with limits.
- [x] Outstanding findings have tickets or an explicit accepted limitation.
- [x] Explicit Phase 2 authorization is recorded; only then update AGENTS.md §17.

Validation: compare review claims to existing tests and ADR 0002; preserve the
distinction between current deterministic checks and historical live acceptance.
No runtime changes. Completion evidence: the maintainer reported Phase 1 complete
and explicitly requested adversarial review and Phase 2 implementation on
2026-10-07. The Phase 1 review remains the evidence baseline.

## P2-01: Resolve contracts before modifying public boundaries

Problem: output delivery, authority, and change attribution constrain public APIs.

Acceptance:

- [x] Resolve the three grilling rounds using options and consequences in each ADR.
- [x] Record maintainer decisions, dates, and any superseded ADR clauses.
- [x] Update ticket scope if containment or stronger attribution is required.
- [x] Confirm the acceptance scenario below matches the accepted guarantees.

Validation: walk a noisy command, denied mutation, and dirty-worktree edit through
their proposed events/outcomes. No production implementation in this ticket.
Completion evidence: maintainer explicitly selected all three recommended
contracts on 2026-10-07; adversarial findings added to acceptance requirements.

## P2-02: Bound shell capture and retain partial results

Owner: Codex (primary)
State: Done

Problem: `Tools::shell` uses unbounded `read_to_end`; timeout loses captured output.
Scope: tool execution/result semantics in `src/tools.rs`, plus necessary runtime
terminal payload handling; no live CLI streaming yet.

Acceptance:

- [x] Both pipes drain concurrently with bounded capture and explicit limits.
- [x] Exceeding a limit cannot block child progress or silently claim full output.
- [x] Timeout/cancellation retains bounded observed output and truthful status.
- [x] Cleanup failure and exactly-one-terminal semantics remain intact.

Validation: deterministic finite/infinite flood, partial timeout, cancellation,
pipe failure, and nonzero exit fixtures. Assert retained/queued byte bounds
directly; do not claim reduced memory usage from timing alone. Existing process
cleanup tests remain required. Completion evidence (2026-10-07, baseline `4b8e93f` plus implementation worktree):
`src/output.rs` and shell lifecycle in `src/tools.rs`; finite dual-pipe flood and partial timeout assertions in `tests/phase2.rs`, stalled cancellation output in `tests/environment.rs`. See [the validation record](../reviews/phase-2.md).

## P2-03: Stream output without coupling cleanup to consumers

Owner: Codex (primary), native_bounds test worker
State: Done

Problem: stdout/stderr become visible only after completion.
Scope: tool output seam, checked runtime events, projections, and CLI rendering.

Acceptance:

- [x] A waiting child can emit output visible before it exits, with stream and tool IDs.
- [x] Per-stream ordering and byte/UTF-8 boundaries are handled honestly.
- [x] Stalled/disconnected consumers obey accepted omission and lifecycle semantics.
- [x] Cancellation kills/reaps the child even while public delivery is blocked.
- [x] CLI sanitizes terminal control sequences and does not duplicate streamed output.
- [x] Headless execution uses the same output/result contract.

Validation: handshake-based child fixture proves visibility before completion;
slow-reader fixture proves cleanup before receiver resumption, then checks event
sequence and one terminal result. Exercise redirected CLI output as well as
normal rendering. No persisted output spool. Completion evidence (2026-10-07, baseline `4b8e93f` plus implementation worktree):
Runtime nonblocking admission/concurrent publication, raw-byte events, CLI stream rendering/decoding; handshake and stalled-consumer tests plus live terminal acceptance. See [the validation record](../reviews/phase-2.md).

## P2-04: Bound file and search output at the source

Owner: native_bounds worker, Codex integration
State: Done

Problem: shell limits alone leave read_file/grep/glob/list_directory unbounded.

Acceptance:

- [x] Large files, lines, match sets, and directory inventories produce bounded results.
- [x] Results distinguish empty, truncated, unavailable, and failed operations.
- [x] Production avoids collecting an entire result before truncation; cancellation
  is checked during long traversals where practical.
- [x] Existing deterministic ordering, workspace checks, and tool schemas remain
  compatible or have an explicit documented change.

Validation: oversized line/file and many-entry fixtures; assert limits and
metadata without a model API. Do not introduce token budgets or compaction.
Completion evidence (2026-10-07, baseline `4b8e93f` plus implementation worktree):
`src/native.rs` and seven `tests/native_bounds.rs` fixtures; actual serialized/source limits and joined cooperative execution. See [the validation record](../reviews/phase-2.md).

## P2-05: Configure and expose permission policy

Owner: Codex (primary), native_bounds test worker
State: Done

Problem: the only request type is a shell confirmation; native policy is implicit.
Scope: typed policy, runtime enforcement, per-run configuration, CLI input/rendering.

Acceptance:

- [x] Supported capability classes accept allow/ask/deny with documented defaults.
- [x] Effective decisions and reasons are observable, including automatic decisions.
- [x] Denied tools never execute; asking without an input handler denies.
- [x] Cancellation after approval but before dispatch cannot start execution.
- [x] Invalid policies fail before a run; repository content cannot grant permissions.
- [x] Unsupported shell restrictions are rejected, never presented as enforced.

Validation: policy matrix with executor spies, headless and no-terminal paths,
approval/cancellation races, and existing permission lifecycle projections.
No shell parser or persisted approval store. Completion evidence (2026-10-07, baseline `4b8e93f` plus implementation worktree):
`src/permissions.rs`, runtime dispatch, CLI flags; permission matrix, approval cancellation, CLI invalid-option and forged-replay tests. See [the validation record](../reviews/phase-2.md).

## P2-06: Verify the execution boundary adversarially

Owner: Codex (primary), independent reviewer
State: Done

Problem: features must preserve the existing path and cleanup contracts together.

Acceptance:

- [x] Existing traversal/symlink/hard-link/atomic-edit protections still hold.
- [x] Nested workspace roots and rejected operations leave unrelated files unchanged.
- [x] Timeout, cancellation, output pressure, and permission denial preserve
  event ordering, actual mutation outcomes, and no-start-after-cancellation rules.
- [x] Supported trust assumptions and exclusions match ADR 0004 and the README.

Validation: extend focused invariant fixtures where coverage is missing; retain
the full existing suite. Record hostile concurrent path swaps and detached
descendants as outside the contract if that is the accepted decision; do not
write a test that falsely asserts confinement. Completion evidence (2026-10-07, baseline `4b8e93f` plus implementation worktree):
Existing path/cleanup invariants remain green; nested invocation-root test, stalled-consumer cleanup and combined acceptance tests pass. See [the validation record](../reviews/phase-2.md).

## P2-07: Capture Git state and a bounded workspace baseline

Owner: change_evidence worker, independent reviewer
State: Done

Problem: no initial branch/dirty inventory or file baseline is available.

Acceptance:

- [x] Before tool mutations, record Git availability, branch/detached/unborn state,
  staged/unstaged/untracked status, and bounded in-scope content baselines.
- [x] Invocation-root boundaries, binary files, ignored files, and size/inventory
  exclusions have explicit representation.
- [x] Git operations have time/output bounds and cannot run external diff helpers.
- [x] Non-Git, missing executable, timeout, and partial inventory are explicit
  unavailable/incomplete states; they do not silently imply a clean tree.

Validation: temporary real Git repositories, nested roots, unusual filenames,
missing Git and oversized inventories. Check that HEAD, index, and user files are
unchanged by collection. No writes to Git state. Completion evidence (2026-10-07, baseline `4b8e93f` plus implementation worktree):
`src/changes.rs` with bounded Git/content/presence metadata; dirty/index, nested/unborn/detached/non-Git, fsmonitor/filter and oversized inventory fixtures pass. See [the validation record](../reviews/phase-2.md).

## P2-08: Show change evidence on every terminal path

Owner: change_evidence worker, Codex integration
State: Done

Problem: users cannot inspect the run's changes independently of assistant claims.

Acceptance:

- [x] Results/events and CLI expose bounded before/after evidence relative to the
  initial workspace, not just HEAD.
- [x] Native mutation evidence is correlated to tools; shell/external changes are
  labeled as observations with attribution limits.
- [x] Add/delete/edit/mode/binary/oversized cases are represented honestly.
- [x] Completed, failed, cancelled, and ceiling-limited runs attempt bounded final
  collection; unavailable collection does not rewrite committed tool outcomes.
- [x] Pre-existing staged/unstaged changes survive; no automatic rollback occurs.

Validation: dirty-tree fixture with native edits, shell edits, unrelated external
edits, and cancellation after a committed mutation. Verify bounded diff display
and coverage metadata. No automatic staging or commits. Completion evidence (2026-10-07, baseline `4b8e93f` plus implementation worktree):
Native mutation events and final reports in runtime/CLI; ignored/unignored, binary/mode/deletion/coverage fixtures; cancelled edits retain evidence; live dirty-repo checks preserve user content/index. See [the validation record](../reviews/phase-2.md).

## P2-09: Prove the phase end to end and close it

Owner: Codex (primary), independent reviewer
State: Done

Acceptance scenario: start a disposable repo with an existing user edit and a
failing test. A scripted provider inspects it, emits noisy shell stdout/stderr,
makes an allowed edit, encounters a denied operation, fixes the target, reruns
tests, and finishes. Independent variants timeout and cancel during noisy shell
execution and cancel after a committed edit.

Acceptance:

- [x] Live output precedes command completion; limits and omissions are visible.
- [x] Denial prevents side effects; timeout/cancellation cleans up within the
  accepted process contract; event history reconstructs actual outcomes.
- [x] Original user work survives and change evidence uses the correct baseline.
- [x] Run completion is still distinct from independently verified task success.
- [x] Full test/fmt/Clippy checks pass; README and architecture reflect behavior.
- [x] Record a manual terminal check and a separate live-provider fixture result,
  or explicitly record unavailable live validation for maintainer disposition.
- [x] Write Phase 2 review with measured findings, remaining limits, and closure
  recommendation. Do not start Phase 3 automatically.

Validation: deterministic integration first, then manual/live checks in disposable
repositories. Include revision, commands, actual results, and limitations.
Completion evidence (2026-10-07, baseline `4b8e93f` plus implementation worktree):
90 deterministic tests, fmt/Clippy/build, live8-model/15-tool fixture, controlled SIGINT/PTY check, updated README/architecture/ADRs, and completed Phase 2 review. See [the validation record](../reviews/phase-2.md).

## Progress and completion records

The table above is the local status source. Each ticket inherits the owner and
phase from this plan until individually assigned, and records review/validation
evidence under its own section when completed. If external issues are later
created, add links here and choose which system owns status.

## P2-10 — Terminal startup and transcript visibility

Status: Done. Owner: Codex. Authorized by the maintainer's request to make bare
`astrid` open the CLI, support vertical transcript scrolling, and color text.

Bare invocation now prompts for a model (unless `ASTRID_MODEL` is set) and a
task, then uses the existing one-run runtime and saved authentication. Default
rendering appends to terminal scrollback, with semantic colors and `NO_COLOR`
support. The existing fixed viewport remains opt-in. This is a presentation
follow-up; it does not introduce persistent conversations or advance the phase.

Acceptance: bare startup without login; existing explicit commands preserved;
scrollback preserved without default screen clearing or scroll margins; colored
replies, approvals, completion, and failures; plain redirected output.

Validation (2026-10-07): 91 offline tests passed, formatting and Clippy passed,
and the updated CLI was installed. A controlling-terminal PTY check verified
model/task prompts, blank task rejection, and no login or screen clearing. Solo
diff review found no runtime/permission changes. Scrollback navigation uses the
terminal emulator's own controls and retention limit.

P2-10 design follow-up: restore the original blue/cyan pixel logo and
side-by-side runtime metadata on both bare startup and direct runs. The bare
startup task composer sits below this shared header; the run does not print
another copy. Maintainer requested retaining the earlier design. The full
91-test suite passed before adding a header regression test; all 10 binary
tests then passed, including color, narrow layout, and absence of screen-clear
or scroll-margin controls. Formatting, Clippy, and an installed-binary PTY check
passed. The transcript still uses ordinary terminal scrollback rather than a
fixed footer that would interfere with scrolling.

P2-10 composer follow-up: the maintainer requested a shaded task input, slash
command discovery, account model selection, and remembering the last model.
Startup uses `ASTRID_MODEL`, otherwise `~/.config/astrid/last-model`, otherwise
the first listed account model. Every explicit run and `/model` choice saves the
selection atomically. `/` reveals commands as it is typed; `/model` presents a
filterable account catalog with arrow-key selection and Enter confirmation;
Esc cancels. The slash-command menu also supports arrow keys. Composer redraws only its own
four lines, leaving the transcript in scrollback. No runtime or routing policy
changes. Terminal state is restored on completion, EOF, and Ctrl-C.

Composer validation: all 92 offline tests, formatting, and Clippy passed.
A live terminal check exercised slash-menu arrow navigation, Enter selection,
account-catalog loading, model-name filtering, arrow selection, and atomic
persistence of the chosen model. The installed binary includes these changes.

P2-10 input ownership correction: one Composer instance owns the input region
across task entry, commands, and model selection. Transitions redraw the owned
region rather than append another composer. Help remains inside the command
menu; cancellation returns to the same input; dropping the composer clears
only its owned lines before runtime output. A terminal check verified entering
`/model`, account-catalog display, and Esc returning to the same anchor, with
cursor-up redraws across the transition. The 92-test suite passed, followed by
all 11 binary tests including the new owned-region cleanup regression.
Formatting and Clippy passed; installed CLI updated.
