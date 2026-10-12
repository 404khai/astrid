# CLI workspace evidence noise fix

Date: 2026-10-12
Scope: authorized follow-up to M01; presentation fix only
Review: Solo implementation and diff review

## Problem and behavior

After a read-only notes answer, the CLI printed hundreds of build-cache paths as
`modified_or_unavailable`. Snapshot content exceeding the per-file or aggregate
budget cannot establish a content change, but the renderer displayed every
uncertain path as an individual change detail.

The renderer now summarizes unverified content comparisons in one count and keeps
the evidence-incomplete label. It prints at most 20 other workspace change details,
with an explicit count for additional entries retained in runtime evidence.
Confirmed text patches, binary changes and observed mode changes remain visible;
per-tool native mutation evidence remains independent of this presentation limit.
Runtime snapshots/reports, permission policy, run outcomes and stored schemas are
unchanged. No cache folders are silently excluded from observation.

## Validation

Two deterministic presentation regressions cover:

- A report with 1,000 unavailable cache files plus a real text edit and a changed
  binary produces a short summary, preserves known changes and incomplete coverage,
  and leaves the full report unchanged.
- Twenty-five known changes print 20 details plus an omission count; native
  mutation evidence for a path beyond that limit still displays its patch.

`cargo test --locked --bin astrid workspace_`: 3 passed (two new regressions and
existing context/workspace patch projection).

Full validation: `cargo test --locked`, `cargo fmt --check`,
`cargo clippy --locked --all-targets -- -D warnings`, and `git diff --check`.
All checks passed: **201 passed, 0 failed, 1 ignored** in the existing checkout
(including separate uncommitted M01 tests); format, strict clippy and whitespace
checks passed. The ignored test is the existing optional recording-overhead experiment.

The validation checkout also contains separate, uncommitted M01 implementation
and user planning/app files. This PR includes only the presentation fix, its two
regressions and this review. It does not stage those other changes or advance M02.
Large actual changes can still contain substantial text patches; this fix bounds
workspace detail entries rather than replacing the existing patch-byte limits.
