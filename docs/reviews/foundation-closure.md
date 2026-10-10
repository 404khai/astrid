# Phase 2–4 foundation closure audit

Date: 2026-10-10
Baseline: `cdebe18` on `experiment/mobile`, plus the closure worktree changes
Review: Solo implementation, contract, test and Git review
Disposition: Administratively closed with documented limitations
Maintainer closure decision: Approved on 2026-10-10, conditional on verified exit
criteria and accurate limitations; the matrices and validation below evidence them
Active phase: Phase 4; no successor implementation authorized

## Scope and decision history

Read current AGENTS.md, development workflow, accepted ADRs 0001–0012, Phase
2–4 plans/reviews and their adversarial records, and the personal-agent roadmap.
Inspected the current tool/workspace/output/permission/change boundary, context
selection and adapter accounting, checked runtime events, recorder/reader/stats,
session storage and CLI integration, together with their deterministic fixtures.
The per-phase current exit-criteria matrices are in [Phase 2](phase-2.md),
[Phase 3](phase-3.md) and [Phase 4](phase-4.md).

The original phase review dates, counts, branch/merge instructions and live
observations are historical. Phase 3 authorization on 2026-10-07 and Phase 4
authorization on 2026-10-08 do not establish earlier closure acceptance. ADRs
0010–0012 explicitly extend single-run/ephemeral history and permission presets;
their implemented sessions are not a Phase 4 trace feature or a background service.
The current marker remains Phase 4. ADR 0013 was separately accepted on 2026-10-10;
its implementation remains Not Started and separately unauthorized.

No open implementation ticket or unresolved material finding in the accepted
Phase 2–4 scope was identified. P3-00 and P4-00's remaining closure governance
was resolved by the explicit 2026-10-10 decision, not by passing checks alone.

## Fresh validation

macOS (Darwin), 2026-10-10, Rust/Cargo 1.96.0, debug/test profile, baseline above:

- `cargo test --locked`: **189 passed, 0 failed, 1 ignored** after the fixture fix.
  Covers all library, CLI/PTY, tool, provider/local HTTP, context, runtime,
  permission, Git/change, observability, session and session-store suites.
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `git diff --check`: passed. Local Markdown validation checked 162 file/heading
  links across 38 Markdown files; all passed. Repaired the historical P3-02 anchor.

The first full-suite run failed
`timeout_retains_partial_output_without_inventing_completion_or_exit_code`:
its 150 ms timeout produced empty stdout rather than the expected `before`.
This fixture assumed startup/observation would occur within that small window;
empty observed output is permitted by the runtime contract. Its timeout now allows
2 seconds, still far below the child’s 30-second wait. Exact stdout/stderr,
observed bytes, timeout, incomplete coverage and unavailable exit-code assertions
remain. Production timeouts, output capture and cleanup were not changed. This
is a scheduling-sensitive real-process test, not a timing benchmark or a promise
that output is always observed before any deadline.

No fresh live provider request or manual terminal acceptance was performed.
Phase 2’s recorded live tests remain historical evidence. Phase 3/4 local HTTP
fixtures exercise the actual adapter without establishing live subscription usage
availability. The historical recorder overhead experiment remains measured evidence
at its documented baseline; it was not rerun and is not a current performance claim.

## Explicit limitations and debt dispositions

| Area | Current limit / disposition | Trigger for further work |
| --- | --- | --- |
| Execution authority | Retain ADR 0004/0011 trusted-local scope. Shell approval/allow grants broad account authority; no independent network/destructive shell restriction, hostile path-swap confinement or detached-descendant guarantee | A concrete requirement for containment or a new capability requires a security decision; cross-device use requires authenticated principal/target/grant contracts |
| Change evidence | Retain ADR 0005 bounded observations, incomplete coverage and limited attribution; no automatic rollback/staging, ignored general content or guaranteed optimized patch | Stronger authorship/isolation requirements; do not infer authorship from shell changes |
| Context | Retain ADRs 0006/0007 estimate plus exact bytes and whole exchanges, explicitly incomplete/stale condensation, cooperative synchronous cancellation | Exact accounting or model summaries need actual provider evidence or a separately accepted replacement contract |
| Memory | Request/tool/storage bounds do not bound total retained runtime history, model response assembly or synchronous copies | Bound future client projections at Mac A; measure retained-memory behavior before low-memory runtime hosting, without rewriting the current history contract here |
| Trace metrics | Retain unavailable true TTFT, prefill/decode, generation rate, memory/KV metrics and cost. Report first visible text and inclusive/operation timings honestly | A backend exposes measured boundaries; cost additionally needs a known billing basis. No pricing lookup can establish subscription billing |
| Live telemetry | Documented closure limitation: no live subscription telemetry verification; deterministic missing/zero/partial/invalid fixtures establish handling | A separately scoped live check can verify what an actual call supplies, but cannot guarantee perpetual availability |
| Recording | Retain ADR 0009 metadata privacy, incomplete prefixes and independent diagnostics; full store rejects new traces, lock contention may reject recording, blocked syscall worker can outlive finalization | Profiling justifies capacity/worker optimization, or an accepted retention/content-mode decision changes the contract |
| Session durability | Retain ADR 0012 private idle snapshots, byte ceilings, one writer/workspace, explicit failed/uncertain saves; no replay or incomplete-batch continuation | Background B needs service/reconnect/crash outcome decisions. Idle restoration is not active-run recovery |

These limitations and future triggers accompany the maintainer's administrative
closure decision, conditional on verified criteria and accurate documentation.
They preserve the existing accepted ADR contracts; no missing telemetry is marked
verified and no stronger runtime guarantee is introduced.
Absence of live evidence or unavailable metrics is not an implementation blocker
under those contracts. Requiring additional evidence as a closure gate is a maintainer
choice and must be recorded explicitly.

## Lessons across the foundations

Request admission, original history, durable idle snapshots and metadata traces
are four different boundaries. Later accepted storage does not make earlier
memory or context estimates exact. Observation is evidence with coverage, not
proof of task success or shell authorship. Operation timing and downstream delivery
waiting must remain distinct. Tests involving real processes need startup allowance
without weakening lifecycle assertions. Future clients should consume these
guarantees rather than replace them with UI-owned execution.

## Git preservation and next milestone

Initial user changes were AGENTS.md, README.md, docs/adr/README.md and
docs/development.md, with untracked ADR 0013 and the personal-agent roadmap.
They were preserved; this task adds closure evidence/navigation, clarifies historical
plan status and changes one test deadline. No production subsystem, dependency,
phase marker, Git index or branch was changed by the audit. The subsequent
maintainer decision below separately changes closure records and ADR acceptance.

The recommended next milestone remains **Mac A**, following the
[roadmap reconciliation](../plans/personal-agent-roadmap.md). Foundation closure
and ADR 0013 acceptance are now separately recorded. Prepare the smallest native
SwiftUI/AppKit client and bundled Rust helper plan in a new thread, inspecting
the desktop reference as its primary layout specification; implementation still
requires separate authorization. No native Mac implementation, service, remote trust, local
inference, routing or scheduler is authorized by this audit.

## Historical recommended maintainer decision (before approval)

Context:
All Phase 2–4 implementation criteria are evidenced; historical closure remains
unrecorded after subsequent implementation authorizations.

Existing constraints:
AGENTS.md and docs/development.md separate closure from successor authorization.
Accepted ADRs 0003–0009 and 0010–0012 define the implemented guarantees and limits.
ADR 0013 remains proposed; the active marker is Phase 4.

Options:
Close Phases 2–4 with the limitations above, or hold closure for an explicitly
named additional acceptance requirement such as live subscription telemetry.

Recommendation:
Close Phases 2–4 with the documented limitations, including unverified live
subscription telemetry, while retaining Phase 4 as the current marker pending
separate successor authorization.

Why:
Current deterministic evidence satisfies the accepted contracts; no required
implementation gap remains. Extra containment, metrics or recovery guarantees
would change those contracts rather than finish an omitted ticket.

Cost of changing later:
Low for the closure record; changing authority or persisted semantics still
requires its own architectural decision.

Decision required:
Accept closure of Phases 2, 3 and 4 with these documented limitations, without
authorizing a successor milestone or accepting ADR 0013?

## Maintainer resolution (2026-10-10)

The maintainer's explicit "Decision: Phase closure and ADR 0013" approves
administrative closure of Phases 2, 3 and 4 provided existing exit criteria have
been verified and outstanding limitations accurately documented. The per-phase
matrices, 189 passing tests and repository checks above supply that verification
within the accepted contracts. Historical/live/unavailable evidence remains
accurately labeled; the conditional approval does not convert it into verified work.

The maintainer separately accepts ADR 0013 with ten clarifications, now recorded
in that ADR. Acceptance does not authorize implementation or advance Phase 4.
Mac A remains Not Started. Planning and genuine blockers belong in a new thread;
later separately authorized implementation must use vertical slices, deterministic
protocol tests and native application validation.

Decision-record validation: documentation-only updates; `git diff --check` and
164 local file/heading links across 38 Markdown files passed. The active Phase 4
marker, Accepted ADR 0013 / Not Started implementation fields and desktop image
target were checked. Runtime validation remains the preceding 189-test evidence;
no Mac implementation or tests are claimed by this decision update.
