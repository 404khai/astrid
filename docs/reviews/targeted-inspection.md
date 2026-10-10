# Targeted native inspection review

Date: 2026-10-09
Baseline: `037b26f` on `main`
Branch: `feat/tools`
Authorization: Maintainer requested line ranges, numbered reads, explicit
truncation, surrounding grep lines, file filters, bounded pagination, and a PR.
Scope: Existing native read-only tools; Phase 4 remains active.
Review: Solo implementation and contract review.

## Behavior and acceptance

- `read_file` accepts nullable/omittable 1-based inclusive line bounds. It scans
  through a fixed buffer, retains only the requested range, returns raw content
  and numbered lines, and reports output/scan truncation and partial final lines.
- `grep` adds workspace-relative include/exclude globs before candidate selection,
  independently bounded before/after context, and offset/limit pages in path/line
  order. A known additional fitting record provides `next_offset`; page count and
  byte stops are distinguished from incomplete scan coverage.
- Existing calls without new arguments remain accepted. Provider strict schemas
  expose new controls as required nullable fields. The actual HTTP mock/tool/model
  round trip verifies arguments, filtering, numbered results, and page continuation.
- CLI expanded read previews show source line numbers. Compact results report
  read truncation, additional search pages, incomplete coverage, and context loss.
- Workspace validation, read permissions, cancellation, lifecycle events, and
  metadata trace privacy continue through their existing seams.

## Limits and tradeoffs

Read results retain both raw content for compatibility and numbered line records.
This duplicates text within the bounded result: use targeted ranges. Raw retained
content is capped at 32 KiB/1,024 lines and further reduced to fit 64 KiB JSON.
Prefix scanning is capped at 16 MiB plus one EOF-detection byte. It is not a
random-access line index; a distant range can be unreachable within that scan.

Grep keeps existing bounded inventories, file/aggregate scans, and match-line
limits. Context has 4 KiB per direction; oversized context is explicitly omitted.
Indivisible matches exceeding the 32 KiB page budget are omitted with a counter.
Each page rescans and offsets are valid only for an unchanged query and files;
there is no cursor persistence or snapshot isolation. A null next offset does not
prove full repository coverage. Cancellation during native IO is cooperative.
No whole-runtime memory bound or agent task-success improvement is claimed.

The context acceptance fixture now admits 28,000 request bytes rather than 18,000
because numbered results retain both forms. It still asserts eviction, compaction,
wire-byte accounting, and call-ID safety. Early-exit assertions now precede waiting
for all mock replies, exposing admission failures without a stalled test.

## Validation

- `cargo test --locked`: 180 passed, 1 normally ignored overhead experiment;
  full deterministic suite including source/output bounds,
  range/EOF/UTF-8 cases, filter-before-inventory behavior, page continuity, oversized
  context/matches, provider round trip, and CLI projection.
- `cargo fmt --check` and `cargo clippy --locked --all-targets -- -D warnings`.
- `git diff --check` and local documentation link validation.
- No live provider test or performance benchmark is claimed.

No material blocker found in the solo review. Session-persistence work in the
original checkout is excluded from this branch and PR.
