# Phase 2 review: execution environment

Current audit: 2026-10-10, `cdebe18` plus closure worktree changes
Status: Administratively closed with documented limitations; solo current audit
Maintainer closure decision: Approved on 2026-10-10, conditional on verified exit
criteria and accurate limitations; the current audit below supplies that evidence
Current phase: Phase 4; separately authorized 2026-10-08
Historical review: 2026-10-07, `4b8e93f` plus implementation worktree;
acceptance and independent adversarial review passed

## Current exit-criteria audit

The exit criterion is safe repository modification with visibility into changes
and executed commands, under the accepted trusted-local-repository contract.

| Requirement | Current code and rerun evidence | Assessment / limit |
| --- | --- | --- |
| Timeout, cancellation and exit status | [tools.rs](../../src/tools.rs), [runtime.rs](../../src/runtime.rs); tools, environment, runtime and phase2 tests | Met; process-group cleanup, partial timeout/cancel output and truthful unavailable status; detached descendants excluded |
| Separate streamed stdout/stderr, bounded output | [output.rs](../../src/output.rs), runtime; dual-pipe flood, handshake-before-exit, stalled-consumer cleanup, decoder fixtures | Met; accepted events lossless, raw-output omissions explicit; final delivery can await attached consumer |
| Workspace boundaries/path validation | [workspace.rs](../../src/workspace.rs), atomic writes in tools; traversal/symlink/hard-link/nested-root and exact-edit tests | Met under trusted-filesystem assumptions; cwd does not confine shell |
| Configurable permissions | [permissions.rs](../../src/permissions.rs), runtime; executor-spy matrix, approval/cancel race, event replay and sessions preset tests | Met for read/write/execute; independent network/destructive shell policies intentionally unavailable under ADR 0004 |
| Git branch and starting dirty files | [changes.rs](../../src/changes.rs); staged/unstaged/untracked, unborn/detached/nested/non-Git, filter/fsmonitor/index fixtures | Met; read-only collection and explicit bounded/unavailable metadata |
| Run-relative patches/change visibility | changes/runtime and CLI; repository acceptance, ignored native edits, binary/mode/deletion/ignore-membership and cancelled-commit fixtures | Met; native evidence correlated, shell/external changes observed with limited attribution |
| Commands and actual outcomes remain observable | [events.rs](../../src/events.rs), runtime and CLI/PTY tests | Met in live events/results; metadata-only persisted traces deliberately omit shell command text |

Recommendation: **close Phase 2 with explicit limitations**. No remaining required
implementation work was found. The tight timeout fixture was repaired without
changing runtime behavior. See [shared validation and debt dispositions](foundation-closure.md)
for the 189-test rerun, first-run failure, checks, Git preservation and exact
maintainer decision. The current audit is solo; the independent review below is
historical and its regression tests were rerun in the full suite.

Current retrospective: preserve the six lessons below. The historical claim that
conversation is not budgeted or persisted is superseded for selected requests
and idle sessions by ADRs 0006/0007/0012. Total retained memory remains unbounded.
Do not carry shell confinement, complete inventory, rollback or agent-authorship
assumptions into the native client. A client must preserve output coverage,
approval ordering and cleanup even when its rendering stalls. No new Phase 2
architectural decision is required; maintainer closure disposition is the only
remaining phase gate.

Closure resolution (2026-10-10): the maintainer explicitly approved administrative
closure of Phases 2, 3 and 4 provided existing exit criteria were verified and
limitations accurately documented. The current matrix and shared validation
record satisfy those conditions within the accepted contracts. No unverified live
telemetry, unavailable metric or new runtime guarantee is marked complete.
Phase 4 remains the active marker. ADR 0013 is separately accepted, with Mac A
implementation Not Started and separate implementation authorization required.
The remaining decision/status statements below preserve the historical review.

## Historical exit criteria and evidence (2026-10-07)

| Criterion | Implementation | Evidence | Assessment |
| --- | --- | --- | --- |
| Timeouts, cancellation, truthful exit status | [tools.rs](../../src/tools.rs), [runtime.rs](../../src/runtime.rs) | Timeout partial-output, stalled-consumer cleanup, committed-edit cancellation, existing cleanup-failure tests | Preserved and extended |
| Separate live stdout/stderr with bounded output | [output.rs](../../src/output.rs), tools/runtime, [console.rs](../../src/console.rs) | Dual-pipe flood, early-output handshake, UTF-8 split/suffix tests, live terminal sample | Implemented with explicit capture/live omissions |
| Source-bounded native inspection | [native.rs](../../src/native.rs) | Large file/line, escaped Unicode JSON, match floods, reverse-order inventory fixtures | Implemented; incomplete coverage cannot prove absence |
| Workspace/path boundaries | [workspace.rs](../../src/workspace.rs), existing atomic mutation boundary | Existing traversal/symlink/hard-link tests plus nested-workspace regression | Preserved under trusted-local-filesystem assumptions |
| Configurable permissions | [permissions.rs](../../src/permissions.rs), runtime, CLI | Read/write/execute allow/ask/deny spy matrix, no-input rejection, approval/cancellation, CLI parsing, forged replay tests | Implemented; defaults read/write allow, shell ask |
| Git awareness and dirty baseline | [changes.rs](../../src/changes.rs) | Staged/unstaged/untracked, unborn/detached/nested/non-Git, fsmonitor/filter marker and index-preservation tests | Implemented with explicit metadata limits |
| Reviewable changes relative to run start | changes/runtime/events/console | Native and shell changes, ignore membership, binary/mode/deletion/oversized/incomplete inventories; live dirty-repo repair | Implemented; native evidence correlated, shell/external attribution limited |
| Honest terminal history | [events.rs](../../src/events.rs) | Independent projection, serialization, exactly-one-terminal tests, policy/start validation | Preserved; configuration/action/grant consistency is checked |

Historical assessment: Phase 2 meets the accepted trusted-repository contract.
Recommended closing with the explicit limitations below; Phase 3 authorization
was then a separate decision and has since been recorded in AGENTS.md.

## What we learned

**What did we learn?** Output admission and output observation are different
facts. A bounded queue can stop accepting bytes while readers must continue
draining pipes. Capture omissions, live omissions, unread bytes, and bytes
unavailable for text rendering need distinct representations. Event delivery
and process cleanup must also be independently polled.

**Which assumptions were wrong?** Read-only Git invocation can execute configured
clean/process filters. An independent same-size content-change fixture proved
this; suppressing fsmonitor alone was insufficient. Likewise, membership in
`git ls-files` does not prove existence: changing ignore rules caused false
creation/deletion reports until a separate bounded presence inventory was added.
Finally, a correct runtime dispatch path did not automatically make replay reject
an Ask-policy start without a grant; checked state now enforces that consistency.

**Which abstractions proved useful?** The existing execution IDs, controlled state
transitions, and independent projections supported extending the runtime without
moving execution into the CLI. A small typed PermissionPolicy and a tool-output
seam were sufficient. Separate content and path-presence evidence made coverage
limits explicit instead of hiding them in a diff string.

**Which abstractions are premature?** Disk output spooling, persisted traces,
isolated execution/worktrees, shell command classifiers, layered configuration,
extra crates, and context selection remain outside this phase's requirements.

**What debt would distort the next phase?** Conversation history, model output,
mutation inputs, and repository instructions still grow outside individual tool
output budgets. Context work should use their real representations and provenance,
not assume these output limits constitute a context engine. Git baseline content
is evidence data; it should not automatically become model context. Tool truncation
means missing coverage, not a token-aware relevance decision.

**What should not be carried forward?** Do not promise that all observed changes
were caused by the agent, that a green run result proves task success, or that
permission approval/cwd confines a shell. Do not replace an unavailable metric or
incomplete inventory with an empty/zero value. Preserve the adversarial fixtures
when changing permission, Git, or output behavior.

## Deterministic validation

On macOS, 2026-10-07, in this implementation worktree:

- `cargo test --locked`: **90 passed, 0 failed** (11 library, 8 binary, 7 changes,
  5 environment, 15 loop, 7 native bounds, 4 Phase 2 acceptance, 5 independent
  adversarial, 19 runtime, 9 tool tests).
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo build --locked`: passed.

Independent review is recorded in
[phase-2-adversarial.md](phase-2-adversarial.md). The reviewer authored and ran
separate regression fixtures, reproduced the Git filter/ignore issues before
their fixes, then verified the permission replay correction. All material
findings are resolved; this is not a claim of formal security verification.

## Live and terminal acceptance

The real subscription provider's catalog listed `gpt-5.6-luna`; the built CLI
was invoked against a disposable Git copy of the greeting fixture with
`--shell-policy allow --max-model-calls 12 --shell-timeout 30`.

Observed terminal output showed `stream-first` on stdout and `stream-error` on
stderr before a one-second sleep and `stream-last`. The run inspected the fixture,
observed two failing tests, edited the existing greeting function, created the
required file, and observed two passing tests. It completed in **8 model calls
and 15 tool requests**. Native patches and the final two-path report rendered.

Independent post-run checks verified:

- `USER.txt` working content remained `unstaged user edit to preserve\n`.
- Its staged content remained `staged user edit\n`.
- The index SHA-256 remained
  `9b6a7bf1711ea91bcb3928f17a8eb3b0781286fef9c128adae8de5aa7bf399fd`.
- `GREETING.txt` contained exactly `Hello, Astrid!\n`; fixture tests were unchanged.

Fixture retained at
`/private/var/folders/c0/32vfjvv9233c0nmghgqt_mp40000gn/T/astrid-phase2-live-9t6q4r74`.

A separate real-provider PTY fixture sent SIGINT after an active shell wrote its
PID and emitted `before-cancel`, while it was waiting on `sleep 60`. The CLI
cancelled after one model call/one tool request, exited 1, retained 13 observed
stdout bytes and their coverage metadata, rendered final workspace evidence,
and an independent PID check confirmed the shell leader was reaped.
This exercises the same SIGINT handler used by terminal Ctrl-C. The PTY used
append-only rendering; existing terminal sizing/resize fixtures cover the viewport.
Fixture retained at
`/private/var/folders/c0/32vfjvv9233c0nmghgqt_mp40000gn/T/astrid-phase2-signal-g5c3d7vm`.

An earlier live shell probe was allowed to reach its 30-second timeout instead
of being signalled. It retained `before-cancel` and returned a recoverable timeout,
which the model reported before normal termination. That probe is timeout evidence,
not cancellation evidence; the separate controlled SIGINT fixture establishes
cancellation behavior.

## Limits and disposition

- Trusted local repositories; shell allow/approval grants broad account authority.
  Detached descendants and hostile concurrent filesystem swaps remain outside
  the process/path contract.
- Policies govern native tool dispatch and shell execution. Runtime instruction
  loading and bounded evidence collection are bookkeeping reads, including when
  native read tools are denied.
- Accepted public events remain lossless; cleanup is independent of a stalled
  consumer, while terminal notification/return can wait for draining or detachment.
- Individual tool output and evidence are bounded. Total conversation/state is
  not budgeted, compacted, persisted, or resumable.
- Git filter suppression may conservatively differ from ordinary Git status.
  Metadata errors, path-membership changes, oversized/binary content, and report
  limits remain explicit. A large ignored tree can exhaust path-presence coverage.
- Filesystem scan deadlines and cancellation are cooperative during local regular
  file IO; no hostile filesystem/sandbox guarantee is claimed.
- Patches use bounded whole-file replacement, suitable for review but less compact
  than an optimized diff. Reports describe observations rather than arbitrary-shell
  authorship, and no automatic rollback or Git mutation is performed.

All [Phase 2 tickets](../plans/phase-2.md) have implementation and acceptance
evidence. No required Phase 2 implementation work remains under the accepted
contract. This review is the closure artifact for maintainer disposition.
