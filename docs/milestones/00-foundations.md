# M00 — Foundation reconciliation

Updated: 2026-10-12
Authorization: Documentation and read-only validation authorized by the maintainer's
2026-10-12 canonical-roadmap/checkpoint request
Implementation: Complete (documentation only)
Acceptance: Passed with documented scope/authorization limitations, 2026-10-12
Dependencies: Existing AGENTS.md, ADRs 0001–0014 and foundation reviews

## Objective and scope

Create one delivery sequence and one resumable checkpoint grounded in actual code,
without changing historical phase IDs, accepted runtime contracts or successor
authorization. Existing personal-agent/Tauri assessments supply evidence, not
competing active queues. No runtime code, GUI, dependency or persisted schema changes.

## Vertical slices

### M00-01 — Record the factual baseline

Status: Done

- [x] Inspect runtime/model/permission/workspace/session boundaries and UI reference.
- [x] Preserve Phase 2–4 administrative closure and the Phase 4 active marker.
- [x] Distinguish accepted ADRs, superseded native direction and proposed Tauri host.
- [x] Record that non-Git folders already work; directory-bound sessions remain.
- [x] Separate private idle snapshots from active-job durability and metadata traces.

Evidence: [architecture](../architecture.md), [foundation closure](../reviews/foundation-closure.md),
[ADR index](../adr/README.md), and the source links in M01/M02. No live provider,
Tauri application, authenticated Android client or multi-agent execution is verified.

### M00-02 — Publish roadmap and checkpoint

Status: Done

- [x] Canonical roadmap has a historical ledger and separate authorization,
  implementation and acceptance fields.
- [x] M01/M02 specify small observable slices, dependencies, tests, risks and evidence.
- [x] CURRENT.md states active planning scope, exact next action and validation.
- [x] Development entry point and previous proposals direct readers to the new plan.
- [x] Local Markdown links resolve and changed files pass whitespace checks.

### M00-03 — Verify and hand off

Status: Done

- [x] Record fresh Rust validation with exact baseline/results and limitations.
- [x] Review the documentation diff against the requested scope.
- [x] Record authorization gates without accepting ADR 0014 or starting successor code.

## Acceptance and validation

Another session can find the active scope, actual guarantees, first draft code
slice, its authorization gate and commands using CURRENT.md. Historical proposals
are labeled without removing their decision history. Unavailable telemetry remains
unavailable, and prototypes are not reported as live capabilities.

Run `cargo test --locked` for the current baseline; `git diff --check` and local
Markdown target validation for documentation. Format/clippy checks can establish
a baseline but no Rust code change is required here. Historical counts must not
be presented as current test results.

## Risks and decisions

The brief's standing advancement instruction and AGENTS.md's current restriction
remain distinct. Record the contradiction; successor code needs explicit scope
resolution. Existing dirty/untracked user files must be preserved. A checkpoint
at a dirty revision must name that fact rather than implying a clean commit.

## Completion evidence

Verified on macOS, 2026-10-12, at `c6873e079c0a0923d238d983def8c5c5f7031b9a`
plus pre-existing worktree changes and this documentation update:

- `cargo test --locked`: 189 passed, 0 failed, 1 ignored (optional recording-overhead
  experiment). No live provider request or desktop application was exercised.
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `git diff --check`: passed. Local target/heading and whitespace validation checked
  70 Markdown targets across the 10 touched planning/navigation files; passed.
- Solo document review checked source boundary claims, authorization versus
  implementation status, historical preservation and later-milestone exclusions.
  No independent agent review is claimed.

Created roadmap, checkpoint and three nearest milestone plans; added navigation
to AGENTS.md, the product brief, development workflow and two previous proposals.
No runtime code, dependency, phase marker, ADR status, Git index or branch changed.
Next proposed code slice is M01-01; its implementation authorization remains pending.
