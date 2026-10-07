# Phase 2 adversarial plan review

Date: 2026-10-07
Scope: proposed Phase 2 plan, ADRs 0003–0005, Phase 1 runtime/tools/events/CLI and invariant tests
Disposition: implementation reviewed; material execution/evidence findings resolved
Decision status: maintainer accepted the three recommended contracts on 2026-10-07

## Prioritized findings

### Critical: output delivery must never suspend process cleanup

`src/runtime.rs` currently dispatches with `tokio::join!(e.flush(),
tools.execute(...))`. This allows the tool to run while start-event delivery is
blocked. A streaming refactor that awaits public event delivery inside a pipe
reader would lose that guarantee: a full channel could prevent both pipe draining
and cancellation cleanup.

Use a bounded producer queue with nonblocking insertion and explicit omission
accounting. Independently poll execution/cleanup while runtime delivery waits.
Only the runtime assigns event identities and sequence numbers. Accepted events
remain lossless; unaccepted raw output may be omitted under the proposed ADR.
Join all readers before tool execution returns. Terminal event delivery may still
wait for the consumer under the proposed contract.

Test with a handshake child, a stalled public receiver, and cancellation. Verify
the process group stops before receiver resumption; then drain and validate a
contiguous event sequence and exactly one terminal outcome. A timeout around the
whole run cannot prove this distinction.

### High: cancellation currently cannot retain output

`ToolExecution::Cancelled` and `EventKind::ToolCallCancelled` have no payload.
`Tools::shell` returns `None` on cancellation, and timeout currently returns null
stdout/stderr. Extending successful results alone cannot satisfy P2-02/P2-03.

Carry bounded observed partial output through the execution seam and cancellation
terminal event. Keep cancellation distinct from timeout and never append a new
conversation tool result for a cancelled operation. A tool committed before
cancellation must keep its completed outcome.

Test partial output before timeout/cancellation, cleanup failure, signal exit,
and cancellation after a committed write. Counters describe bytes actually read;
unread bytes after pipe closure are unavailable rather than a fabricated count.

### High: source bounds require changing production, not trimming JSON

`Tools::invoke` reads whole files, collects every directory entry, reads each grep
file wholly, and collects every match. `Tools::files` collects the entire recursive
inventory. Capping returned strings leaves production memory unbounded.

Produce bounded records incrementally; check cancellation during traversal and
reading. Specify oversized-line/record behavior and search coverage explicitly.
An oversized input skipped by search is not evidence that it had no matches.
Bound inventory storage separately from serialized content. Preserve sorted
selection with bounded lexicographic selection or explicitly revise deterministic
ordering; truncating an arbitrary filesystem-order prefix violates the existing
contract. Sorting a full enormous directory first is also unbounded.

Use fixtures with oversized files and lines, many matches/entries, multibyte text,
and different creation orders. Assert retained bounds and deterministic selected
records. Output bounds alone do not claim whole-runtime memory bounds:
conversation history, model output, mutation inputs, and repository instructions
have separate existing limits or limitations.

### High: permission enforcement must reside at dispatch

Current permissions cover only shell requests. P2-05 needs runtime policy evaluation
for supported native read/write and broad shell classes, including automatic
allow/deny events. Denial must bypass the executor. Repository instructions must
never alter policy. Validate policy before provider execution; unknown tools must
not inherit broad authority by default.

Recheck cancellation after approval and before dispatch. Keep effective policy
evaluation separate from human waiting and execution start. Shell read/network/
destructive restrictions must either have real enforcement or fail configuration;
command spelling cannot provide that guarantee.

Test an executor spy across allow/ask/deny, no permission input, invalid policy,
approval racing cancellation, and cancellation while approval delivery is blocked.

### High: Git observation can execute code or overstate evidence

Read-only Git collection needs more than avoiding `git diff` helpers. Repository
configuration can enable fsmonitor hooks; status can refresh index metadata.
Suppress fsmonitor and optional locks, disable external diff/textconv where
relevant, and bound time/output. Parse NUL-delimited paths; lossy filename decoding
can collapse distinct paths. Excluded or unavailable paths need explicit coverage
metadata.

Compare working-tree content to the invocation-start snapshot, with index status
reported separately. If either inventory is incomplete, do not infer additions or
deletions merely from missing entries. Bound retained baselines and rendered
patches separately. Final collection failure must not rewrite successful mutation
outcomes. Shell/external edits are observed changes, not proven agent authorship.

Test dirty staged/unstaged/untracked state, nested roots, unborn/detached/non-Git
repositories, a configured fsmonitor hook that writes a marker, unusual and
non-UTF-8 paths, oversized inventories/files, binary/mode changes, ignored native
edits, and cancellation after a committed edit. Verify HEAD, index, and initial
user edits remain unchanged by collection.

### Medium: bytes, Unicode, and omission accounting need separate contracts

Applying `String::from_utf8_lossy` independently to arbitrary read chunks corrupts
valid characters split between chunks. Use raw-byte chunks or an incremental
decoder retaining only the incomplete trailing sequence. Sanitize terminal controls
when rendering and test redirected output as well as terminal rendering.

Track capture omissions separately from live-delivery omissions. A chunk can remain
in the retained result while being dropped from the live queue. Suppress duplicate
CLI rendering of bytes already streamed, while still exposing omission and partial
result summaries. Do not claim a true stdout/stderr total order.

## Scope and defaults

The proposed single-provider, sequential runtime scope is appropriate. Do not add
spooling, context budgets, checkpoints, worktrees, command classification, or OS
containment incidentally while implementing this plan.

The proposed 8 KiB shell read chunks, 64 KiB queued output, and 64 KiB retained/live
prefix per stream are reasonable initial limits. Document whether queue bounds
include a currently pending runtime chunk and public-channel contents. Public
channel capacity bounds events, not bytes; bound each output event too.

The proposed 1 MiB per-file, 16 MiB total snapshot, and 10,000-entry inventory limits
need deterministic exclusion and incomplete-evidence semantics. Numeric defaults
can change based on fixtures without changing the attribution contract.

Native output caps should cover serialized records and metadata rather than only
the main string. A bounded returned result is separate from elapsed search work;
state any scan/file limits and incomplete coverage honestly. Glob/list record
selection and oversized-line behavior are reversible local choices once recorded.

## Required acceptance evidence

1. Child output is observable before completion using a handshake, not a sleep.
2. Finite and infinite dual-pipe floods obey capture/queue/event bounds and preserve
   truthful exit/timeout/cancellation outcomes.
3. Cancellation cleanup completes with the observer stalled, before event delivery
   resumes; event replay still reconstructs the terminal run correctly.
4. Denied operations never execute; approval/cancellation races cannot dispatch.
5. Native read/search production is bounded and coverage/order are explicit.
6. A dirty starting workspace retains user work and reports run-relative content
   evidence on completion, failure, timeout, and cancellation.
7. Collection failures/limits cannot imply a clean tree or overwrite tool outcomes.
8. Full deterministic tests, formatting, and Clippy pass; manual terminal and live
   provider evidence remain separately labeled, including unavailable validation.

This review supplies implementation requirements; the maintainer's separate
acceptance authorizes the selected contracts and Phase 2 implementation.

## Implementation review follow-up

The initial integration uses independently polled shell execution and public
output publication, raw byte events with incremental text decoding, bounded native
inspection, explicit policy events, and bounded snapshots. These address the main
architectural findings above. Integration validation must still prove runtime
cleanup and permission invariants end to end.

Independent tests in `tests/phase2_adversarial.rs` verify split UTF-8 decoding and
that Git observation cannot invoke fsmonitor or modify the index. Both passed on
the initial integration. The review also found an evidence bug: changing ignore
rules can remove or introduce inventory membership without deleting or creating a
file. Regression tests cover both directions; the initial implementation falsely
reported deletion when an existing user file became ignored. Inventory membership
must carry enough provenance to distinguish exclusion from filesystem absence.

Additional requested checks: include Git metadata in total report-bound claims;
render native evidence for ignored paths in the CLI; keep model-visible shell
descriptions consistent with configurable authority; account for incomplete
terminal Unicode suffixes when converting raw bytes to rendered text.

A further independent fixture found that Git status can execute repository clean
filters when a modified tracked file has the same byte length as the indexed
content. Different-size modifications shortcut content comparison and hide this
bug. The collector now audits filter configuration and disables clean/process
drivers before status, refusing observation if the bounded audit is unavailable.
The fixture verifies both clean and process markers remain absent.

After the collector corrections, all five independent adversarial tests passed:
both directions of ignore-rule changes, fsmonitor/index preservation, clean and
process filter suppression, and split/invalid UTF-8 decoding. These focused checks
supplement the runtime integration suite; they do not replace its cancellation,
permissions, bounded-output, and acceptance checks.

## Final review

The final source review confirmed independent execution polling during output
publication, joined native producers with bounded retained results and cancellation
checkpoints, runtime policy enforcement before dispatch, partial cancelled output,
and bounded change evidence with explicit attribution and coverage limitations.
Git filter/fsmonitor suppression and ignore-membership regressions are resolved.
The CLI now renders native mutation patches for paths excluded from the general
baseline and reports unavailable Unicode suffix bytes; provider-visible shell
instructions reflect per-run policy.

The final projection consistency finding is resolved. Execution state retains
the configured policy, validates tool capability and evaluated action against it,
requires Ask for human permission waiting, and permits execution only after
Allow or a granted Ask. The regression
`replay_rejects_tool_start_without_matching_policy_and_approval` rejects missing
evaluation/approval, forged Allow, and forged capability without partially
mutating state. Final review has no remaining material findings.

Independent review does not expand the supported trust contract: approved shell
commands retain broad account authority; detached descendants and hostile
concurrent path swaps remain excluded. Output limits apply to individual observed
streams/native inspection results rather than all conversation/state memory.
Workspace observation cannot prove shell or external-process authorship.

The root implementation record owns full-suite, manual-terminal, live-provider,
and live-cancellation evidence. The independent review's measured evidence is the
five focused regression tests above; reported live results were supplied by the
root agent and were not independently rerun by this reviewer.
