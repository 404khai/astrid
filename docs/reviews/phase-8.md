# Scheduler and workers implementation review

Date: 2026-10-16. Solo review; no delegated or independent review claimed.
Authorized branch: `feat/task-scheduler-workers`, main baseline `16d755c4`.
Accepted contracts: ADRs 0017–0019. Phase 7 closure remains recommended with
limitations; the maintainer explicitly authorized this implementation transition.

## Slice record

P8-01 implementation: one coordinator validates admission, owns task state,
assigns one attempt and retains each slot until execution and its internal event
collector have joined. Workers use fresh `runtime::run` sessions and independent
cancellation tokens. Parent/task read policies intersect with a hard read-only
ceiling; fresh handlers deny Ask. Existing run events and idle v2 are unchanged.
The new lifecycle projection uses nonblocking send and detaches on full/closed
receivers, while retaining the bounded complete lifecycle log in memory.

Auth prerequisite reproduced first with a real held synthetic file lock: aborting
an async caller left its `spawn_blocking` waiter alive until lock release. The
reproduction explicitly releases/joins that operation. Replaced only lock waiting
with nonblocking exclusive attempts separated by cancellable async waits; the
account lock still covers credential load, refresh, account/scope validation and
save. A second fixture polls/drops a contended wait, proves the held lock is still
exclusive, then checks simultaneous synthetic unexpired-credential consumers.
No real credential values or network inference are used by these tests.

First focused tests prove two inference calls meet a three-party barrier before
either finishes, distinct worker/run/session identities, full lifecycle replay,
projection detach, root cancellation with queued work never assigned a RunId,
and executor-spy denial of write/edit/shell under an unbound parent. Read Ask and
Deny also prevent dispatch without terminal interaction. Invalid configurations
and aggregate call reservations reject before generation.

Review findings resolved so far: two strict-Clippy collapsible-if findings and
three unused test imports. P8-01 full validation passed: 224 Rust passed, 0 failed, 1 ignored; fmt,
strict all-target clippy and whitespace passed. See [pinned evidence](../evaluations/phase-8-slice-1.json).
No completed Phase 8 or final exit recommendation is claimed here.

## Limits retained

Shared reads are not a filesystem snapshot or OS confinement. Provider Send+Sync
does not prove server isolation, throughput or remote cancellation. Cancellation
is cooperative; callers must await the batch through cleanup rather than drop its
future. Native blocking reads retain the existing cooperative lifetime. Results
are unverified data, and private child sessions are released after bounded visible
result/diagnostic extraction. No task persistence, mutation, shell, worktrees,
retries, daemon, desktop or later phase is included.

## Dependency and cleanup review (P8-02/P8-03)

The next coherent slice enables the static DAG and exercises the existing ownership
state machine under dependency/cancellation failures. Graph validation rejects
unknown/self/duplicate edges and cycles before generation. Caller order breaks
ready ties and determines the source bundle; declared edge order determines typed
inputs despite reversed publication. Inputs include source BatchId/TaskId/RunId,
execution outcome, declared predecessor IDs, exact bytes/digest and bounded-final
validation status. Only accepted committed final text is rendered in the original
User task; no raw response/session/tool requests or private history is exported.
Existing context selection protects that task or fails explicitly. A dedicated
multi-turn fixture actually compacts and checks every mandatory predecessor in all
four requests. A tiny-budget fixture fails before dependent generation.

Failure tests preserve original contradictory outputs when synthesis disagrees or
fails, skip descendants of failed/oversized/uncommitted work, and continue unrelated
work. Publication overflow preserves the child's actual Completed outcome. Call
ceiling exhaustion never publishes provisional batch text. Cancellation tests use
barriers/notifications, prove task-only isolation and zero active provider guards at
return, retain completed work under late repeated cancellation, distinguish task
and batch deadlines from user cancellation, preserve cleanup failure precedence,
and catch/join a provider panic without retry. Real held-auth contention also joins
before the fixture releases its account lock. Simultaneous synthetic expired-token
consumers use one loopback mock refresh and observe the saved rotated token; no
paid/network service is required.

Intermediate findings: provider-return notification was insufficient evidence of
publication order, so the fixture now releases A only after B's lifecycle completion.
A held-lock fixture failed only alongside concurrent subprocess tests and passed
alone, consistent with transient duplicated fork descriptors. Explicit owner unlock
now closes account ownership at guard drop; the fixture explicitly releases its
own held lock. This follows the existing idle-store lock-lifetime pattern, without
removing exclusion or weakening scope/account checks. A compaction privacy assertion
also initially conflated the dependent's legitimate own private continuation with
source continuation because the fixture reused one sentinel; the dependent now
emits no such private item, so the assertion specifically detects source leakage.

Focused scheduler checks: 16 passed, 0 failed. Full validation: 237 Rust passed,
0 failed, 1 ignored; fmt and strict all-target clippy passed.
[Pinned evidence](../evaluations/phase-8-slice-2.json). This is not a phase-exit
or factual-quality claim.


## Recording review (P8-04)

Separate schema-1 batch records retain only generated identity/topology, effective
policy, monotonic lifecycle offsets, categorical child outcomes/recording coverage
and numeric lengths/counts/budgets. Lost-worker call counts stay unavailable. The
flat `batch-<UUID>` namespace is a reversible implementation choice replacing the
proposed subdirectory: both families use one generic bounded writer and the same
store lock/byte/file quotas. Existing run records remain unchanged. Dedicated
recorder threads retry nonblocking lock contention for at most 50 ms (also capped
by finalization timeout); failure/saturation remains explicitly incomplete or
unavailable. No lock or quota is removed and no deletion is automatic.

Checked replay validates sequences, dependencies, ownership, immutable terminals,
publication and aggregate failure/cancellation precedence. Metadata reader checks
schema, bounded complete/partial records, topology/concurrency/call ceilings and
footer coverage. Real CLI inspection handles partial metadata separately from run
inspection, without authenticating or creating sessions. Child completeness is
independent of batch completeness. Run statistics enumerate only run-family files,
so child usage is counted once and unknown usage/cost is retained.

New tests cover complete replay/child links, content/digest/private sentinels across
both file families, malformed/future/partial/symlink records, forged success,
concurrent shared byte/file quotas and saturated/failed writers. A malicious-source
fixture actually attempts write/edit/shell and path escapes through the dependent
worker; mutation spies remain empty and native reads deny both parent escape and
symlink targets. An auth duplicate-descriptor regression confirms explicit owner
unlock and stale-descriptor closure cannot release the next owner's lock.

Full validation: 246 Rust passed, 0 failed, 1 ignored; fmt and strict all-target
clippy passed. [Pinned evidence](../evaluations/phase-8-slice-3.json). No live
inference or phase-exit recommendation yet.

## CLI and bounded acceptance (P8-05)

`batch_manifest` reads a regular caller file once within 1 MiB, rejects unknown
fields/schema, and converts only explicit tasks/configuration. Graph/config/budget
validation precedes authentication and task-tool dispatch. `batch_cli` is a thin
headless client: Ctrl-C requests cancellation and awaits owned work; rendering starts
after join, so blocked stdout cannot hold active workers. Its text/JSON report
retains original results/statuses and separates unverified synthesis. Explicit
JSON output is not automatic metadata/session persistence. Invocation-only recorder
diagnostics are now exposed in explicit reports, never added to persisted records.
Observed batch recording finalization is separate from cancel-to-join cleanup.

Deterministic CLI-driver tests use the existing notes and code fixtures with real
native reads and a barrier for the independent sources. Notes and code each take
five scripted model calls/two reads; dependent source envelopes are exact and
originals remain available. The code case distinguishes declared tests from tests
executed. Other cases cover failed dependency, interruption, a broken output writer
after join and no active provider guards at report time. Actual subprocess tests
cover bounded/versioned snapshots, pre-auth invalid graphs, failed JSON reports,
partial metadata inspection and real SIGINT during two held-account-lock waits.
No credentials are needed by critical tests, and existing PTY/session tests remain.

One live notes batch used the frozen rubric/manifest and existing subscription auth:
**29.000902 seconds**, **6 model calls**, **4 tools**, within 12 reserved calls and
the declared deadlines. Solo grading against actual fixture bytes passed: bulbs,
basil on unspecified-calendar Saturday, boiler inspection date unknown, completed
washer excluded, source filenames/uncertainty retained. Both original findings and
their exact attributed inputs remain separate from synthesis. Fixture/rubric hashes
were unchanged. [Original live record and grading](../evaluations/phase-8-live-2026-10-16.json).
This is one protocol/factual sample, not a quality ranking, speed comparison or
proof of provider parallel throughput. Barrier tests establish runtime overlap.

Live recording coverage is explicitly limited: batch metadata complete, one child
complete, two unavailable. Missing child trace causes were not retained in that
pre-diagnostic CLI report; no specific cause is established. A subsequent eight-way
first-use directory test passed, so no directory-race fix is invented. Optional
writer admission/finalization remains fallible under contention/load. Future explicit
CLI reports retain writer diagnostics; no second live batch was spent. Known usage
is only the checklist child (3358 input/273 output/0 cached, two of six calls);
whole-batch usage/cost/true TTFT/decode remain unavailable.

## Exit-criteria review

| Criterion | Evidence | Disposition |
| --- | --- | --- |
| Two meaningful independent owned tasks overlap | Three-party inference barrier, distinct worker/run/session IDs, bounded slots and collector joins | Met deterministically; no provider throughput claim |
| Explicit bounded static DAG | Duplicate/unknown/self/cyclic graph, counts/edge/concurrency/call/input/deadline admission tests; actual CLI pre-auth cases | Met; no dynamic spawning/retry/lease |
| Independent least authority/conversation | Unbound parent and Ask denial spies, malicious dependency/path fixtures, source/private sentinels, fresh session entry point | Met within existing trusted local-account/native path boundary |
| Accepted-result dependencies and reconciliation | Reversed committed order, exact bytes/digests/IDs, source conflicts, synthesis failure, oversized/uncommitted publication, four-call actual compaction | Met; factual interpretation remains unverified |
| Cancellation/failure ownership | Root/task barriers, deadlines, late cancellation, cleanup-failure precedence, panic, real auth contention, synthetic serialized refresh, actual SIGINT subprocess | Met cooperatively; no remote billing or hard-kill guarantee |
| Inspectable lifecycle/correlation/privacy | Checked reducer, saturated/closed projection, returned full log, separate metadata reader/CLI, malformed prefixes/schemas, cross-family byte/file quotas and privacy sentinels | Met with fallible recording; live child coverage explicitly 1/3 |
| CLI and frozen acceptance | Notes/code real-tool driver cases, snapshot/preflight/failure/SIGINT subprocesses, one frozen live notes sample separately graded | Met for this bounded sample; no general quality or speed conclusion |

Final source review also identified that reused policy/context serde types could
ignore unknown nested manifest fields. New-manifest parsing now rejects unsupported
permission/budget keys before authentication without changing existing session serde
contracts. A routed-child integration case checks configured per-call model/effort
trace evidence and visible-only final transfer into a fresh dependent task.

## Six retrospective questions

What did we learn? Scheduling is chiefly ownership, authority, communication and
cleanup. Actual inference overlap needs a barrier; completed inference is not the
same boundary as accepted publication, and execution completion is not factual success.

Which assumptions were wrong? Dropping a blocking auth-lock waiter did not cancel
its acquisition. A provider-return notification did not prove publication order.
Using one fixture private sentinel in both source and dependent could not distinguish
source leakage from legitimate same-worker continuation. A complete batch trace did
not imply complete child recording in the live sample.

Which abstractions proved useful? Fresh owned runs/sessions, explicit grant
intersection, checked lifecycle reducers, typed bounded result envelopes, protected
original context, internal child drains and the common quota-controlled recorder.

Which abstractions are premature? Leases/retries, durable scheduler recovery,
mutable shared worker conversations, dynamic spawning, worktrees, an approval broker,
remote identities, daemon/desktop lifecycle and a new framework/crate hierarchy.

What debt would distort the next phase? Do not hide fallible child trace coverage or
claim unknown usage as zero. Invocation diagnostics help future recorder investigation;
this live sample does not establish its missing traces' cause. Cooperative cleanup,
shared live filesystem reads, approximate context and backend concurrency limits
must remain explicit before any mutating-isolation work is separately authorized.

What should not be carried forward? Parent/sibling private sessions, shared stdin
approvals, unverified synthesis replacing evidence, automatic merges, inherited
shell authority, aggregate context percentages, or traces presented as recovery.

## Disposition

Recommendation: **close with explicit limitations**; final pinned validation passed.
Maintainer acceptance remains separate; no Phase 9 transition is inferred. The
accepted process-local/read-only scope is implemented, including failure paths and
one bounded factual acceptance. Remaining limits are deliberate or explicitly
observed: no filesystem snapshot/OS confinement, no durable jobs/recovery, no hard
preemption or provider billing/throughput guarantee, no live CLI worker meters/full
transcripts, and optional recording can be incomplete/unavailable. The single live
sample's whole-batch usage/cost is unknown because two child traces were unavailable.
No second live batch is used to erase that evidence. Recovery artifacts, private v2
sessions, original continuation, unrelated files and the requested branch are retained.


The routed-child fixture initially asserted that the generic provider seam's owned
history contained no private bytes. That was the wrong boundary: original history
is intentionally retained; replay mode controls the adapter's visible wire view.
The corrected fixture checks visible accounting/no opaque wire items and per-call
route/effort, while the fresh dependent request separately proves no source private
continuation transfer. Existing provider-wire routing tests remain unchanged.


## Final validation — 2026-10-16

Exact final source: `cargo test --locked` **255 passed, 0 failed, 1 ignored**;
**2 Python passed**; fmt, strict all-target clippy and whitespace passed.
[Source hashes/commands](../evaluations/phase-8-validation-2026-10-16.json),
[deterministic acceptance](../evaluations/phase-8-mocked-2026-10-16.json).
The routed fixture's pure OpenAI accounting also correctly rejected unsupported
effort on its initial synthetic model aliases; it now uses the existing exact
Sol/Terra identifiers without network generation. Final checks supersede intermediate
fixture failures and earlier source checkpoints. The single live record remains
original evidence; later diagnostic/parser/test refinements did not trigger another
live run. No broad staging, commit or PR. Scope and recovery artifacts preserved.
