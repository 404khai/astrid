# Phase 7 — Model routing review

Date: 2026-10-15
Review: Solo source/diff/invariant review, not independent adversarial review.
Baseline: main `2deea5f` plus preserved headless M01/Phase 6/user changes and the
uncommitted Phase 7 slice on `feat/phase-6-closure-phase-7-routing`. No M02 import.
Disposition: **Recommend closure with explicit limitations**; implementation complete.
Maintainer Phase 7 acceptance/Phase 8 transition are not inferred.

## Exit evidence

| Criterion | Evidence | Result / limitation |
| --- | --- | --- |
| Explicit ModelCapability/ModelRoute/RoutingPolicy | `src/routing.rs`, provider/runtime integration | Narrow direct-inference seam, ordered configured boundaries; classes are caller intent |
| Multiple model classes in one run | [live synthetic notes sample](../evaluations/phase-7-live-2026-10-15.json), `tests/routing.rs` | Sol/general then Terra/retrieval (live), Sol/general then Terra/coding (scripted); no competence claim from labels |
| Visible why/model/effort/outcome | `ModelRouteSelected`, CLI projection, per-call trace summary | Requested/effective dispatch config plus reason and replay; server-resolved identity/default effort not invented |
| Effort support | Exact Sol/Terra model table and adapter preflight/dispatch validation | Low/high/none/omitted covered; minimal and unknown explicit support rejected; no silent downgrade or fallback |
| Cross-model continuation | Accepted ADR 0015, adapter visible view | Private reasoning excluded from requests; original private messages retained; exact preflight agrees with wire bytes |
| Permissions/cancellation | Focused routed real-tool tests plus foundation suites | Denied writes/shell never start; cancelled batch skips later side effects; no extra stage after final |
| Persistence | Solo review found missing source-model provenance | Approved one-shot/inspection-only guard; save/serialize/resume rejection and previous-snapshot retention pass |
| Quality and metrics | Separately graded live sample, existing traces | One notes rubric pass; observed usage/time only, cost/true TTFT/decode null; no superiority claim |

The controlled live CLI sample used existing subscription auth and copied frozen
synthetic notes, read Allow/write Deny/execute Deny. All three cited actions were
correct, completed washer excluded, boiler date unknown. Coverage qualified the
complete glob scans and reads. Three calls/four tool calls, zero approvals/changed
paths; observed usage 3,313 input/278 output/0 cached. About 15.12 seconds wall.
This confirms request effort and handoff protocol acceptance for this one sample,
not a latency gain, quality ranking or statistical reliability result.

## Solo review findings

1. Initial focused test incorrectly assumed missing usage meant no Usage object.
   Existing accounting represents this as null counters inside Usage. Corrected
   test assertion; did not change accounting or convert missing usage to zero.
2. Adapter accounting had to count visible text/tool items rather than opaque
   continuation items when locating summaries. Corrected both modes; exact-wire
   acceptance checks serialization against received HTTP byte lengths.
3. Unknown backend feature support must fail closed. Default capabilities advertise
   no explicit effort/visible-history support; stage policy validates adapter support.
4. Consequential compatibility issue found: mixed original private history has
   no source-model provenance in current snapshots. Maintainer approved one-shot restriction
   with explicit save/serialization/resume rejection before dependent changes.
   Guard added without schema migration; failed saves retain prior snapshots.
   Resumable routing remains deferred. Original handoff approval did not silently
   authorize changing the existing persistence contract.

## Retrospective

What did we learn? Deterministic routing can use the existing sequential tool loop,
context accounting and traces. Model-call success, rubric success and model ability
remain separate. A visible handoff is feasible with the current upstream endpoint.

Which assumptions were wrong? Keeping the original session unchanged is not by
itself sufficient: later serialization/resumption must account for mixed model
provenance. Also missing usage has a structured representation, not necessarily
an absent object.

Which abstractions helped? Existing typed exchanges, pure request measurement,
permission handler, event replay and bounded telemetry. Typed stage decisions and
adapter capabilities make the additional behavior inspectable without new crates.

Which are premature? Semantic/ML classification, broad provider universality,
external coding-agent adapters, workers and price-derived subscription cost.

What debt would distort Phase 8? Resumable routed-session persistence remains deferred; it must not be
inherited as a safe guarantee. Call-number stages are configuration, not semantic
task detection. One provider/two identifiers and one notes sample do not establish
routing benefit or coding quality. Explicit effort support is deliberately narrow.

What should not carry forward? Opaque cross-model replay, inferred billing/default
effort, scripted quality claims, or automatic worker/scheduler advancement. No
Phase 8 authorization is inferred from completion of this work.

## Validation

Phase 6 pre-routing baseline: 203 passed/0 failed/1 ignored, fmt and strict clippy.
Focused routing: five passed before the additional stage-failure invariant test;
strict all-target clippy passed for that slice. Full post-routing regression is
running. Final accepted contract and commands/results must be recorded here before
closure. M02 remains Paused/unaccepted; 5/13 Deferred; GPUI remains Phase 11 research.

## Validation findings resolved before final run

- The added pre-provider route event invalidated two tests' queue-position
  assumptions. One asserted text had begun when cancellation now preceded the
  provider; another waited indefinitely for preparation while the route event
  itself was backpressured. Changed synchronization to the actual route/text
  boundaries and bounded preparation waiting. Both focused regressions passed.
  The superseded hanging attempt was terminated explicitly; it is not pass evidence.
- A subsequent full run passed routing/runtime but failed one immediate session
  store reopen with WouldBlock. Focused seven-test replay passed unchanged.
  Investigation reproduced a concrete owner-lifetime gap with a duplicated flock
  descriptor: closing the owner alone retains the lock until the duplicate closes.
  Added explicit owner-drop unlock plus a deterministic duplicate-descriptor test,
  including verification that closing an old duplicate cannot unlock a new owner.
  This preserves the existing exclusive-owner contract and snapshot schema. The
  exact scheduling cause of the original transient failure was not established;
  the reproduced lifetime gap is now covered rather than hidden behind a retry.

Final full regression/fmt/strict-clippy must use this post-fix source. No additional
live run is needed: fixes affect local lock release/test synchronization and session
save/resume rejection; fresh live request serialization and routing are unchanged.

## Final evidence and disposition

On the final post-fix source: `cargo test --locked` **211 passed, 0 failed,
1 ignored** across 22 test groups (including doc tests). `cargo fmt --check`,
`cargo clippy --locked --all-targets -- -D warnings` and whitespace passed.
[Validation record](../evaluations/phase-7-validation-2026-10-15.json) pins relevant
source hashes and commands. [Routed execution report](../evaluations/phase-7-mocked-2026-10-15.json)
records per-call decisions/outcomes and unavailable usage/cost honestly; runner hash
matches current source. [Evaluation replay](../evaluations/phase-7-evaluation-replay-2026-10-15.json)
passes all ten cases (five tasks × two context configurations), current runner hash
verified. The six routing tests and duplicate-descriptor owner-drop test pass;
existing native snapshot restoration/private continuation tests remain green.
Intermediate failures and their resolutions above remain part of this review.

P7-01–04 are complete for the accepted bounded contract. The heterogeneous-run
exit criterion is evidenced by both scripted real-tool acceptance and the separately
graded live notes sample. Recommend closure with these explicit limitations:
fresh one-shot routing only, call-number stages rather than semantic classification,
caller intent classes rather than measured competence, one provider/two verified
effort identifiers, limited live quality evidence and unavailable subscription cost.
Resumable routing requires a separate provenance decision; no such migration was
implemented. Existing unbounded retained-history/heuristic context, broad permitted
shell authority and cooperative cancellation limitations remain.

No Phase 8 implementation or authorization is inferred. No staging/commit, branch
switch, worktree, recovery-data removal, desktop import or user-artifact cleanup.
