# CU-02 — Bounded desktop actions

Date: 2026-10-21. Maintainer requested separately authorized click, typing, scroll
and key actions with stale-frame checks and execution limits, then a PR.
Branch: `feat/bounded-desktop-actions`, from fetched main `2daf811c`, same checkout.
No worktree, new conversation claim, PR merge or live operation.

[ADR 0023](../adr/0023-bounded-desktop-actions.md) prepares the separate per-action
grant, native-coordinate/freshness and bounded-execution decision. Proposed until
the specific decision is accepted. [Public API audit/review](../reviews/bounded-desktop-actions-2026-10-21.md)
records why a direct desktop action endpoint must not be invented.

Smallest independently testable exit: a Rust headless action controller using
mocked transport and host-attested frame metadata, covering four action families,
exact per-call grants, stale-frame rejection, fixed execution limits, cancellation
and no uncertain retries. No action tool/CLI registration or image disclosure.

Full runtime/live CU-02 acceptance depends on CU-01's gated observation transport,
production decoding and runtime receipts, plus exact action transport review and
separately approved exact existing target/action trial. No target has been supplied.
Private v2, original history/results, provenance/continuation, complete exchanges,
context admission and ADRs 0008/0009/0021/0022 remain binding. No new persistence,
remote workers, Host/mobile networking, Phase 9, daemon, PTY or desktop client.

Stop after this slice and its unmerged PR. The next plan item MO-01 is future work;
do not begin it or declare the unresolved runtime/live gates satisfied.
