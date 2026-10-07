# How Astrid advances a phase

This is the working process for turning the roadmap into reviewed, testable
changes. AGENTS.md controls scope; accepted ADRs control architectural contracts;
code and tests establish what actually exists. A planning document does not
advance the active phase.

Start here for the current phase:

- [Phase 3 plan and tickets](plans/phase-3.md): accepted context contracts, implementation, and evidence.
- [Phase 3 review](reviews/phase-3.md): completed implementation, evidence, and limitations.
- [Phase 3 adversarial review](reviews/phase-3-adversarial.md): plan critique and first-slice review.
- [Phase 1 review](reviews/phase-1.md): evidence, lessons, and outstanding questions.
- [Phase 2 plan and tickets](plans/phase-2.md): accepted scope, completed tickets, and evidence.
- [Phase 2 review](reviews/phase-2.md): closure assessment and validation results.
- [ADR index](adr/README.md): accepted decisions and proposals awaiting resolution.

## The repeatable cycle

| Step | Work | Exit condition |
| --- | --- | --- |
| 1. Review | Inspect implementation, existing decisions, tests, and prior acceptance evidence. | Every previous-phase exit criterion has evidence or a named gap. |
| 2. Grill | Challenge the next phase's hardest assumptions with concrete failure scenarios. | Each significant question has a decision, experiment, or explicit blocker. |
| 3. Decide | Write small ADRs for consequential boundaries. | Required decisions are accepted by the maintainer, with rationale recorded. |
| 4. Slice | Write tickets for observable behavior, including failure paths. | Dependencies, acceptance criteria, and validation are concrete. |
| 5. Authorize | Record the maintainer's phase decision and update AGENTS.md's current phase. | Implementation scope is explicit. |
| 6. Implement | Take the first Ready ticket; build the smallest complete vertical slice. | Reviewable change, invariant tests, docs, and acceptance evidence. |
| 7. Close | Run the phase acceptance scenario and review what was learned. | Exit criteria evidenced; remaining debt assigned, accepted, or deferred. |

Do not require a new approval for ordinary implementation choices inside an
accepted phase/ADR/ticket. Escalate only decisions that change that contract.
If evidence changes an assumption, update the plan and affected decisions before
continuing dependent implementation. Do not wait until phase close to expose it.

## Grilling protocol

Use short rounds of two or three related questions. For each, present **Context,
Options, Recommendation, Consequences, Decision question**, as required by
AGENTS.md. Start with the highest-impact uncertainty; do not ask the maintainer
to choose details that can be decided locally.

For each proposed guarantee, ask:

1. What exact behavior does this promise, and to whom?
2. Show the happy path and one concrete failure/cancellation path.
3. Who owns the state and who can change it?
4. What happens before a side effect, during it, and after it commits?
5. How does the event stream prove what happened? What remains unknowable?
6. What happens when a dependency hangs, disappears, or floods output?
7. What is bounded: bytes, time, tasks, retained history? Where is it unbounded?
8. Which current ADR or test would this change?
9. What simpler design would meet the same requirement?
10. Which deterministic test would falsify the proposal?

Record answers in the phase plan's decision table and the relevant ADR; unanswered
questions remain open. If neither option has adequate evidence, create a bounded
investigation ticket with a question, fixture, comparison, and stop condition.
Do not label a hypothesis a finding or turn an investigation into implementation.

Stop grilling when the remaining choices are reversible local details and every
architectural blocker has a disposition. The goal is a usable contract, not an
exhaustive questionnaire.

## Tickets

Use stable IDs `P<phase>-<number>`, e.g. `P2-03`. Initially keep tickets in the
phase plan, using the [ticket template](templates/ticket.md). Split a ticket when
its acceptance criteria describe independently reviewable behaviors, not merely
because it touches several files. No mandatory points, deadlines, or new crates.

States: **Draft → Ready → In progress → Review → Done**. **Blocked** records the
exact missing decision/dependency; **Deferred** names the later phase or reason.

Ready requires an authorized phase, accepted prerequisite decisions, completed
dependencies, testable acceptance criteria, and known exclusions. Assign an
owner when moving to In progress. A ticket can be well specified and still Draft
while its phase is unauthorized.

Done requires:

- Observable behavior and failure semantics meet the acceptance criteria.
- Relevant deterministic tests pass, including affected runtime invariants.
- `cargo test --locked`, `cargo fmt --check`, and
  `cargo clippy --locked --all-targets -- -D warnings` pass for code changes.
- Documentation matches behavior; significant deviations update the ADR.
- Evidence records revision, commands, results, and limitations. Live model
  acceptance is separate from mocked acceptance and only required when relevant.
- A review checks the diff against scope, identifies remaining risks, and records
  resolution of material findings. A solo review is labeled as such.

Documentation-only tickets need link/content validation, not invented runtime
tests. External issues are optional mirrors: if used, link them to these IDs and
designate one status source. Do not maintain two conflicting backlogs.

## ADRs

Use an ADR for decisions that constrain component boundaries, externally visible
semantics, authority, lifecycle, or difficult-to-reverse data contracts. Use a
ticket note for local refactors, naming, and routine implementation choices.

Use the [ADR template](templates/adr.md). Status is **Proposed**, **Accepted**,
**Rejected**, or **Superseded**. Implementation state is a separate field.
Only record Accepted after the maintainer resolves the question; include date
and decision evidence. A draft recommendation is not consent.

One decision per ADR where practical. When changing an accepted contract, identify
the exact clauses affected and link the superseding decision in both directions.
Preserve history instead of silently rewriting why the old design existed.
Not every phase requires an ADR.

## Phase closure

Use the [phase review template](templates/phase-review.md). Include an exit-criteria
matrix, evidence, six retrospective questions from AGENTS.md, unresolved debt,
and a recommendation: close, close with explicit limitations, or hold.

The maintainer's closure decision and authorization for the next phase are
separate facts. Neither passing tests nor finishing all tickets automatically
authorizes the next phase.
