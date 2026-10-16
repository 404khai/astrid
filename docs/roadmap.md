Authorization update — 2026-10-15: the maintainer now explicitly requests Phase 6
closure review followed by Phase 7 in a new main-based branch/thread. This supersedes
earlier pending-Phase-7 statements below, conditional on the Phase 6 evidence review.
See [current handoff](CURRENT.md) and the new AGENTS authorization entry.

# Astrid canonical engineering roadmap

2026-10-16: Authorized Phase 7 follow-ups on a new main-based branch:
[context meter → quality comparisons → resumable routing](plans/phase-7-followups.md).
V2 provenance/migration contract explicitly approved in the implementation thread.
No Phase 8 transition.

Updated: 2026-10-15. This section supersedes the historical milestone delivery
order retained below. Personal-agent goals (Meister, Astrix, groups, persistent
tasks, coding and cross-device assistance) remain long-term goals, not phase gates.
The CLI is the primary client; the Rust runtime remains authoritative and independent.

| Phase | Current disposition / next contract |
| --- | --- |
| 0–4 | Foundations retained; 2–4 administratively closed with limitations; see [closure](reviews/foundation-closure.md) |
| 5 — Local inference | Deferred explicitly, not completed; no exit-criteria claim |
| 6 — Evaluation | Closed with explicit limitations after main-based comparison/review; [plan](plans/phase-6.md), [review](reviews/phase-6.md) |
| 7 — Model routing | Initial routing and authorized [follow-ups](plans/phase-7-followups.md) implemented; [current review](reviews/phase-7-followups.md), accepted ADR 0016 v2 resumability; closure recommended with limitations; no Phase 8 authorization |
| 8 — Scheduler and workers | Pending separate authorization; explicit ownership and communication |
| 9 — Git worktree isolation | Pending; coding worker isolation, not general task requirement |
| 10 — MCP and external tools | Pending; adapter preserves native runtime semantics |
| 11 — Interface | Pending; GPUI selected direction, separately evaluate and scope desktop |
| 12 — Experimental runtime research | Pending; hypothesis, baseline, measurement and result |
| 13 — Phalanx | Deferred alongside Phase 5 unless separately authorized |

M00/M01 evidence is retained. [M02](milestones/02-desktop.md) is Paused with
partial implementation and acceptance Pending; [pause audit](reviews/direction-revision-2026-10-15.md).
[ADR 0013](adr/0013-native-mac-helper-boundary.md) remains Superseded;
[ADR 0014](adr/0014-tauri-desktop-runtime-boundary.md) is now Superseded, preserving
its acceptance history. ADRs 0001–0012 remain binding, with ADR 0012’s v1-only format revised by accepted
[ADR 0016](adr/0016-persistent-routing-provenance.md). No Tauri IPC, web renderer,
embedded desktop host or Liquid Glass assumption becomes a future GPUI requirement.

At Phase 11, investigate then-current upstream GPUI capabilities/API stability,
macOS/Windows/Linux support, accessibility/platform integration, efficient streaming
conversation rendering, animated characters, transparent overlays, background host
and client lifecycle, and compatibility with existing events/permissions. These
are investigation questions, not verified capabilities. No new desktop ADR yet.

[CURRENT.md](CURRENT.md) records actual evidence; [development process](development.md)
controls review, tickets and authorized transitions. The subsequent 2026-10-15
new-thread instruction authorized Phase 6 closure and Phase 7 implementation after
evidence review; that work is now reviewed. No Phase 8 transition is authorized.


# Historical milestone roadmap (superseded delivery order)

Updated: 2026-10-14
Planning baseline: `c6873e079c0a0923d238d983def8c5c5f7031b9a` plus existing worktree changes
Status: Canonical delivery plan; M01 accepted with limitations, M02 work authorized

Start each development session at [CURRENT.md](CURRENT.md). This document owns
delivery order; [architecture.md](architecture.md) describes implemented contracts;
[AGENTS.md](../AGENTS.md) controls authorized scope. The original
maintainer product brief (`MILESTONE_ROADMAP.md`, retained locally outside this PR) supplies long-term requirements, not a
second active milestone list. Earlier delivery proposals remain historical:
[personal-agent reconciliation](plans/personal-agent-roadmap.md) and
[Tauri assessment](plans/desktop-tauri-reconciliation.md).

## Product and boundary contracts

Astrid is a personal agent powered by its own inference-aware Rust harness.
Coding is a first-class capability; clients and tasks need not be coding tools.
Meister is its primary assistant persona across workspace, floating, mobile and
messaging presentations. The floating window is a presentation, not a separate
agent or inference process. Meister appears in the first desktop conversation;
advanced delegation arrives later.

Astrix are persistent specialized agent identities with instructions, preferences
and scoped capabilities. An identity is neither a model, session, task, worker
nor run. Cubot can lead one group without becoming Astrid's global coordinator.
Group roles do not confer tool authority. Start group coordination with one user
and multiple Astrix; collaboration between human principals requires a separate
sharing/trust decision.

The runtime owns execution, context, permissions, cancellation and events. Clients
own input and presentation. Model backends provide inference; agent backends
execute delegated work; execution targets perform authorized actions. Keep these
integrations distinct. No new crate or service follows merely from a concept name.
Normal run completion is not proof of task success. Summaries and animations must
project evidence without inventing execution or private reasoning.

## Authorization, implementation and advancement

Track these independently on every milestone:

- **Authorization:** Documentation only, Implementation authorized (with dated
  scope evidence), or Implementation pending. Framework selection and ADR
  acceptance are separate records; neither alone starts implementation.
- **Implementation:** Not started, In progress, or Complete with linked evidence.
- **Acceptance:** Pending, Passed with limitations, or Failed with named gaps.

Tickets use [the development workflow](development.md): Draft, Ready, In progress,
Review, Done, Blocked or Deferred. Ready requires implementation authorization,
accepted prerequisite contracts, completed dependencies and testable criteria.
An authorization gate does not prevent preparing documentation or fixtures in
a plan; it prevents implementing the gated capability.

The initial 2026-10-12 request authorized roadmap/checkpoint documentation.
The maintainer subsequently explicitly authorized M01-01 through M01-03 as a
scoped extension, recorded in AGENTS.md, on `feat/m01-personal-tasks` in the
existing checkout and a new implementation thread. M01 implementation is
Authorized / Not started / Acceptance pending at handoff. M01 was subsequently
implemented and passed acceptance with limitations on 2026-10-12; see
[its completion evidence](milestones/01-personal-tasks.md#completion-evidence).
This does not accept
ADR 0014, change the historical Phase 4 marker or authorize M02. The brief's
standing advancement proposal has not replaced milestone-specific authorization.

On 2026-10-13 the maintainer authorized scoped M02 work in a new thread on
`feat/m02-tauri-desktop`, in the existing checkout without a worktree, using the
Figma exports in `references/` and macOS Liquid Glass. The host contract was initially
Proposed; after its concrete recommendation, the maintainer instructed implementation
to start on 2026-10-14, accepting option 1/clauses 1–10. M02 implementation is in
progress; see [the review](reviews/m02-desktop.md). Later milestones remain unauthorized.

Until the maintainer explicitly records a replacement advancement policy, finish
authorized scope and prepare the next milestone, but do not start its code.
If standing implementation authorization is subsequently adopted, record its
scope and date here and in AGENTS.md; automatic advancement still requires passed
acceptance, completed dependencies and accepted consequential decisions. Routine
reversible decisions within authorized scope require no repeated approval.

## Historical phase ledger

Phase numbers remain references to their original contracts, not the new delivery
order. No phase is renumbered or closed by this plan.

| Original phase | Implemented/disposition | Evidence and treatment |
| --- | --- | --- |
| 0 — Loop | Implemented foundation | [ADR 0001](adr/0001-phase-0-execution-contract.md); retain validated model completion and sequential tools |
| 1 — Runtime model | Implemented; closure recorded | [Phase 1 review](reviews/phase-1.md); retain headless execution, IDs, ordering and cancellation |
| 2 — Execution environment | Administratively closed with limitations, 2026-10-10 | [Foundation closure](reviews/foundation-closure.md), [Phase 2 review](reviews/phase-2.md) |
| 3 — Context engine | Administratively closed with limitations, 2026-10-10 | [Phase 3 review](reviews/phase-3.md); estimates, byte ceilings and incomplete summaries remain explicit |
| 4 — Trace/observability | Administratively closed with limitations; current marker retained | [Phase 4 review](reviews/phase-4.md); unavailable metrics remain unavailable |
| 5 — Local inference | Not delivered as a phase | Independent measured workstream; not a desktop/mobile prerequisite |
| 6 — Evaluation | No standalone phase closure | Begin minimum personal/coding baseline at M01; expand with each capability |
| 7 — Routing | Not delivered | Evidence-driven M05; configuration can precede automatic selection |
| 8 — Scheduler/workers | Not delivered | Separate host lifetime (M04) from parallel assignments (M08) |
| 9 — Worktrees | Not delivered | Conditional coding isolation after concurrent workers; not a general execution environment |
| 10 — MCP | Not delivered | One justified adapter at M07 may draw from this scope when authorized; no blanket rollout |
| 11 — Interface | CLI presentation exists; original phase not closed | Bring Tauri forward at M02; prototypes are not runtime integration |
| 12 — Experimental research | Not delivered as a phase | Hypothesis/baseline/measurement/result per later experiment |
| 13 — Phalanx | Not delivered | Independent future inference integration through the provider boundary |

Accepted ADRs 0001–0012 remain binding, with ADR 0012’s v1-only format revised by accepted
[ADR 0016](adr/0016-persistent-routing-provenance.md). ADR 0013 is Superseded, with its former
acceptance retained as history. [ADR 0014](adr/0014-tauri-desktop-runtime-boundary.md)
is Accepted (2026-10-14): option 1 and clauses 1–10 authorize the embedded host boundary.
Idle session snapshots are implemented; durable active jobs are not.

## Canonical delivery order

Only M00–M02 have detailed engineering plans. Later rows are outcomes and gates,
to be refined when their prerequisites supply evidence. Every later milestone
currently has Implementation pending / Not started / Acceptance pending.

| ID / objective | Dependencies | Required capability / first outcome | Acceptance evidence and main risk |
| --- | --- | --- | --- |
| [M00 — Foundation reconciliation](milestones/00-foundations.md) | Existing source, ADRs and reviews | Canonical plan, factual baseline and resumable checkpoint | Link checks and recorded validation; stale/contradictory authority |
| [M01 — Personal task foundation](milestones/01-personal-tasks.md) | M00 complete; implementation authorized 2026-10-12 | Read-only notes-to-checklist in a non-Git directory; minimum evaluation baseline | Denied mutation, source/coverage checks and separately recorded live quality; coding assumptions and content disclosure |
| [M02 — Tauri desktop foundation](milestones/02-desktop.md) | M00; ADR 0014 acceptance and scope authorization; M01 for product acceptance | Embedded Rust host, one active run, real sessions, streams, approvals and cancellation | Mocked IPC/lifecycle/privacy tests and macOS acceptance; renderer authority, backlog and quit cleanup |
| M03 — Astrix identities | M02; scoped identity/configuration contract | Two persistent configured Astrix with separate conversations, initially sequential | Restart/configuration isolation tests; identity mistaken for authority, persistence changes |
| M04 — Independent runtime host | M02; service authority/lifetime ADR | Accepted task continues after GUI exit; reconnect to status, results and pending approval | Process detach/reconnect/host-death tests; uncertain effects, split writers and orphaned approval |
| M05 — Model configuration and routing | M01 evaluation; concrete backend choices and continuation contract | Requested/effective effort, explicit override, then deterministic capability-aware selection | Unsupported-effort and route trace tests plus comparative tasks; incompatible provider continuation or invented capabilities |
| M06 — Android thin client | M04; pairing/revocation and remote-principal ADR | Authenticated submission, bounded incremental history, reconnect and approvals | Low-memory hardware measurements and revocation/reconnect tests; secret leakage, stale authority and unbounded payloads |
| M07 — Scoped execution environments | M01; one concrete workflow and target/adapter authority ADR | One browser, computer or application action adapter with explicit target and result | Denial/cancellation/uncertain-outcome fixtures and real acceptance; external custody and side effects |
| M08 — Assignments, workers and Meister delegation | M03, M04; task ownership/budget contract | Independently assigned work with bounded concurrency, ownership, dependencies and evidence-based reporting | Deterministic duplicate/cancel/dependency/budget tests; shared mutable context and conflicting writes |
| M09 — Group coordination | M03, M08; group context/access contract | One user, multiple Astrix, contextual lead, assignments and artifact references | Membership/context isolation and evidence-reference tests; simulated progress and cross-group leakage |
| M10 — Floating Meister | M02; successful overlay feasibility experiment | Compact animated view of the same assistant/state | macOS focus, pointer, monitor, accessibility and state consistency acceptance; platform behavior and animation overhead |
| M11 — Messaging client | M04; channel identity/approval contract | One authenticated messaging adapter, initially Telegram | Duplicate delivery, principal binding and reconnect tests; account/channel authority confusion |
| M12 — Advanced personal capabilities | Relevant earlier evidence and specific scope decisions | One memory, schedule or trigger workflow at a time; broader ADE only when warranted | Retention/deletion, missed-trigger and verified-outcome tests; privacy and unsolicited side effects |

M04 is prioritized ahead of M05 after Astrix delivery because mobile requires an
independent host, while routing does not. M05 and M07 can proceed independently
of M06 once their own prerequisites and authorizations are satisfied. Shared
code dependencies must be recorded before selecting an implementation ticket.

### Parallel workstreams and bounded experiments

- **Evaluation:** deterministic invariant tests from the foundations; small
  known-answer personal tasks at M01; coding regression baseline; broaden for
  routing, delegation, reconnect and recovery as those features arrive. Record
  success criteria, unwanted effects, approvals, latency, calls/tokens, coverage
  and cost only when its billing basis is known.
- **Local inference / Phalanx:** add one backend at a time when authorized; compare
  the same tasks and trace representation. No new phase completion is inferred
  from endpoint test seams or unavailable throughput metrics.
- **Mobile feasibility:** early bounded-history/payload and reconnect design work
  can inform M02/M04. Production remote access waits for M06's trust decision.
  Android validation must use stated hardware and measured memory, with numerical
  acceptance limits set before the experiment. The current iOS demo is not evidence.
- **Overlay feasibility:** after desktop hosting works, test one small macOS
  window for transparency, topmost behavior, focus, pointer interaction, shortcut,
  monitors and animation cost. It need not wait for groups. No desktop observation
  or control privilege follows from displaying a window.
- **Avatars:** inspect portable definitions and package/asset licenses before
  integration. Start desktop with original/static assets; no avatar studio or
  dependency is required for M02. React Native rendering requires separate proof.
- **Worktrees / external agent harnesses:** justified by a real coding assignment;
  distinct from inference routing and personal device capability execution.

## Completion and maintenance

For each implemented slice record revision plus worktree scope, commands/results,
manual/live evidence separately, failure findings, accepted decisions and remaining
limitations. A test count alone is not product acceptance. Update the milestone
and CURRENT.md before ending the session. At closure, review lessons and debt,
then verify authorization before moving to another milestone. Preserve historical
reviews and explicitly supersede changed contracts rather than rewriting history.

### M02 host acceptance — 2026-10-14

After the concrete ADR 0014 contract was presented, the maintainer instructed
implementation to start. ADR 0014 option 1/clauses 1–10 are Accepted; scoped
M02-01–04 implementation is in progress. Earlier pending records are history.
Material/accessibility/product acceptance is not inferred from this decision.
Historical Phase 4 remains unchanged and no successor milestone is authorized.
