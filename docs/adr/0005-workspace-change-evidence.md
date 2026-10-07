# ADR 0005: Git baseline and workspace change evidence

Date: 2026-10-06
Status: Accepted
Implementation: Implemented; validated 2026-10-07
Related: Phase 2; P2-07 and P2-08; ADR 0001 invocation-directory workspace boundary
Decision evidence: Maintainer explicitly selected the recommended contract in
conversation on 2026-10-07.

## Context

Astrid lacks Git awareness. A working-tree diff against HEAD includes pre-existing
changes and cannot by itself establish which edits a run caused. Shell and other
processes can modify files outside the native write/edit boundary.

## Options

1. Report branch/status and final HEAD diff, labeling it as combined workspace
   state. Simple, but does not provide run-relative change evidence.
2. Record a bounded before/after baseline plus native mutation evidence, with
   explicit uncertainty for shell/external changes.
3. Isolate all mutations and checkpoint full workspaces. Stronger attribution,
   but introduces worktree/checkpoint machinery assigned to later phases.

## Accepted decision

Choose option 2. Capture branch or detached HEAD, initial dirty paths, and bounded
file baselines before the first tool side effect. Recheck after termination,
including failure/cancellation once cleanup permits it. Never stage, stash, reset,
commit, or revert. A non-Git workspace remains usable with Git metadata unavailable.

Preserve the invocation directory as mutation scope even inside a larger repo.
Expose repository identity as metadata; restrict file baselines/diffs to workspace
paths. Git commands must be read-only, bounded, cancellable, and run without
external diff/textconv helpers. Handle missing Git, unborn branches, detached HEAD,
unusual filenames, command failure, and oversized results explicitly.

For in-scope tracked and non-ignored untracked regular files, use a proposed
1 MiB per-file and 16 MiB total in-memory content budget and a 10,000-entry
inventory cap. Deterministically mark excluded/oversized/unknown entries; do not
silently equate an incomplete inventory with no changes. Treat these limits as
initial values to validate. Ignored files and shell writes outside the workspace
are not covered by the general baseline. Native mutations can still provide
bounded before/after evidence for their directly touched paths.

Compare start content with end content where both exist, rather than using HEAD
as the run baseline. Represent new/deleted files, mode changes, binary content,
and unavailable/truncated diffs explicitly. Preserve pre-existing staged and
unstaged edits; report index status separately from working-tree content deltas.
All retained data and rendered patches need bounds, including native mutation
evidence. Do not solve Phase 4 persistence here.

Distinguish **native tool mutation evidence** (correlated to a ToolCallId under
the documented no-hostile-concurrent-writer assumption) from **workspace changes
observed during the run**. The latter includes shell changes but cannot prove
causation. Never label every changed file as agent-authored. Bounded change
collection failure must report unavailable evidence without relabeling a
successfully committed tool operation as failed.

## Consequences

Users can review known before/after differences while seeing coverage limits.
Full arbitrary-shell attribution remains unavailable without isolation. Runtime
results/events carry the report; the CLI only renders it. Capture/report work
must have its own bounded cleanup behavior even when run cancellation is set.
There is no rollback, automatic Git workflow, worktree creation, or persisted trace.

## Decision question

Accept bounded run-relative evidence with explicit attribution limits, rather
than promising every observed workspace change was caused by Astrid?

## Validation

Begin with staged, unstaged, and untracked user changes; perform native and shell
edits; verify the initial work remains intact and the report distinguishes the
two baselines. Test cancellation, oversized/binary/ignored files, inventory limits,
non-Git workspaces, nested invocation directories, and concurrent external edits.

## Resolution

Accepted on 2026-10-07. Invocation-directory scope from ADR 0001 remains
authoritative; change evidence is explicitly bounded and attribution limited.

## Implementation evidence and adversarial corrections

`changes.rs` captures Git metadata and bounded content before dispatch and at
termination, and the runtime emits/returns bounded reports. Native mutation
evidence is captured per-call, including ignored paths. Path labels preserve
UTF-8 versus raw hex identity. Patches are bounded whole-file replacements.
The final report limit includes both Git metadata and changes; Git metadata has
a 128 KiB report sub-budget. A five-second scan deadline and individually bounded
Git subprocesses make failure/unavailability explicit; final observation uses
a fresh cancellation handle after task cancellation.

An independent reviewer demonstrated that fsmonitor suppression alone was
insufficient: `git status` ran a configured clean filter on a same-size changed
file. Observation now audits configured clean/process/required filter keys and
neutralizes them for status; audit failure makes status unavailable. Submodules
are ignored and optional index writes are disabled. Filter suppression may
conservatively change status classification from ordinary Git semantics.

The reviewer also demonstrated false creation/deletion when ignore rules changed.
A separate bounded inventory of path presence now includes ignored names without
reading ignored content; membership changes become `coverage_changed`. Large
ignored trees can exhaust presence coverage, making missing-path evidence unknown.
Non-Git incomplete inventories retain a filesystem traversal prefix rather than
promising globally deterministic selection; retained reports remain sorted.

Evidence: `tests/changes.rs`, independent `tests/phase2_adversarial.rs`, and
committed-edit cancellation coverage in `tests/environment.rs`. See the
[adversarial review](../reviews/phase-2-adversarial.md).
