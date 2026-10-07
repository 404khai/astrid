# Phase 2 review: execution environment

Date: 2026-10-07
Baseline: `4b8e93f` plus the current implementation worktree
Status: Implementation complete; acceptance and independent adversarial review passed
Maintainer closure decision: Pending disposition of this completed review
Next-phase authorization: Not requested; AGENTS.md remains Phase 2

## Exit criteria and evidence

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

Assessment: Phase 2 meets the accepted trusted-repository contract. Recommend
closing with the explicit limitations below. Phase 3 remains a separate decision.

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
