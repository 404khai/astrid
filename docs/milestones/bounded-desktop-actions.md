# CU-02 — Bounded desktop actions

Date: 2026-10-21. Maintainer requested separately authorized click, typing, scroll
and key actions with stale-frame checks and execution limits, then a PR.
Branch: `feat/bounded-desktop-actions`, from fetched main `2daf811c`, same checkout.
No worktree, new conversation claim, PR merge or live operation.

[ADR 0023](../adr/0023-bounded-desktop-actions.md) prepares the separate per-action
grant, native-coordinate/freshness and bounded-execution contract. Accepted on
2026-10-21 after the explicit Option 1 decision and all four clarifications. [Public API audit/review](../reviews/bounded-desktop-actions-2026-10-21.md)
records why a direct desktop action endpoint must not be invented.

Implemented slice: a Rust headless action controller using
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

Publication: [PR #25](https://github.com/404khai/astrid/pull/25) now contains the
accepted contract and mocked controller, targeting main and left unmerged.


## Clarified implementation boundary

`src/desktop.rs` supplies one run-owned controller, separate approval/grant and
one-method mock transport seam. Trusted metadata installation is capped at two,
including replacements, without resetting action/window budgets. The host must
retain one controller per run/target; this is not a global registry or protection
against a malicious in-process host constructing duplicate owners. Metadata is
not a verified screenshot receipt and does not enforce CU-01 byte/decode admission.
Future real integration must mint receipts only from its successfully admitted,
validated observations and preserve all screenshot/operation limits.

The 5 s freshness/approval bound is experimental. Human approval can exceed it;
reject the stale frame with no silent recapture or reused action approval. A
practical live strategy remains a separate decision. Acknowledged input is not
proof of UI effect. Post-admission cancellation, timeout/error or dropped caller
leaves unknown outcome and closes the controller. No automatic retry/continuation.

Acceptance contract commit: `b452ef23`, before dependent controller implementation.
No production transport, runtime/model registration, image input/persistence,
background execution, permission/limit expansion, live trial or PR merge.


## Validation and disposition

**327 Rust passed, 0 failed, 1 ignored; 18 existing Python regression tests passed;
fmt, strict all-target Clippy and whitespace passed.** Nineteen new Rust tests
cover the controller and persisted metadata privacy. The accepted mocked-controller
slice is complete with the documented host-trust, experimental freshness and
cooperative async transport limitations. No live screenshot/input, runtime tool,
model image disclosure or next milestone is implemented. Full runtime/live exit
requires the specific contracts in the [review](../reviews/bounded-desktop-actions-2026-10-21.md).
[Source/command evidence](../evaluations/bounded-desktop-actions-2026-10-21.json).
