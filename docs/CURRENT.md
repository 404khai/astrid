# Screenshot observation contract and fixtures — 2026-10-21

CU-01 uses parent-created `feat/screenshot-observation` from main `7d7f45d5`,
same preserved checkout, no worktree. #22/#23 were already merged upstream;
this task merges no PR. The maintainer explicitly accepted
[ADR 0022](adr/0022-screenshot-observation.md) with its gates: separate exact-call
observe authority, one memory-only frame/10 s retention and zero model image
context/disclosure. No local preset, exec grant, selection or resume grants observe.

Public OpenAPI and SDK 0.5.3 audit found POST desktop status but no direct screenshot
endpoint; SDK screenshot uses commands/files and guest cleanup. No guessed wire
payload or live adapter was added. The executable specification is Python-only,
synthetic and disconnected from Rust/runtime/provider/session/trace integration.
Its restricted PNG decoder and explicit fake-clock cleanup are fixture proofs,
not a production decoder, idle timer or hard cancellation guarantee.

**308 Rust passed, 0 failed, 1 ignored; 18 Python passed (16 CU-01 + 2 existing);
fmt, strict all-target Clippy and whitespace passed.**
[Review/API audit](reviews/screenshot-observation-2026-10-21.md),
[evidence](evaluations/screenshot-observation-2026-10-21.json),
[milestone](milestones/screenshot-observation.md), [active plan](plans/remote-mobile-cli.md).
Private v2, original results/history, continuation/provenance, complete exchanges,
context admission, cancellation and ADRs 0008/0009/0021 remain unchanged.

Design/fixture slice validated. Full CU-01 runtime observation remains gated on
transport, production decoding, runtime integration and separately authorized live
scope. Image persistence/provider/schema changes remain gated. RE-02 live trial
is independently pending exact target/scope; CLI/fixture checks are not live evidence.
No credentials/private sessions or reference images read, cloud/VM/model action,
CU-02 actions, lifecycle, Host/mobile, daemon, PTY or desktop client implementation.
Next plan item CU-02 remains future; stop here and leave its contracts unresolved.
Unrelated `.gitignore`, historical CURRENT edits and untracked user/recovery data
remain preserved and outside the task commit.

Publication: contract `1579a889` and fixtures `11971bf8` are published in
[PR #24](https://github.com/404khai/astrid/pull/24) against main, **open and unmerged**.
No PR was merged by this task. Only the design/fixture slice is delivered;
runtime/live gates remain pending and CU-02 has not begun.

---

# Foreground InstaBox CLI approvals — 2026-10-21

RE-02 connects invocation-only `--instabox-machine <exact-ID>` selection to
separate exact-call approvals for one-shot and interactive CLI runs, on existing
`feat/instabox-cli-approvals` from RE-01 `2f8f9358`, same checkout, no worktree.
Latest maintainer direction supersedes only RE-01's stop restriction for CLI
implementation, deterministic mocks and a reviewable live plan. ADR 0021 remains
binding: no local preset, resume, read-only worker or selection alone grants
remote authority. No target discovery/default/persistence or automatic retry.

Commands/cwd display as complete JSON strings; remote authority, private exact
attribution, actual output and cancellation/unknown outcome are visible in the
foreground terminal. Fresh answer channels per run discard cancelled replies.
Private v2 history/continuation/provenance, context/exchange admission and ADRs
0008/0009 off/metadata-trace semantics are unchanged. Keys stay host-side.

**308 Rust passed, 0 failed, 1 ignored; 2 Python passed; formatting, strict all-target Clippy and whitespace passed.**

Validation is recorded in [review and concrete trial plan](reviews/instabox-cli-approvals-2026-10-21.md),
[evidence](evaluations/instabox-cli-approvals-2026-10-21.json),
[active plan](plans/remote-mobile-cli.md) and [milestone](milestones/runtime-recovery.md).
No exact live machine/provider/action scope was supplied or approved: live trial
and full RE-02 acceptance remain pending. No authenticated metadata, provider
inference, cloud/VM operation, credential/private-session read or successor work.

Read-only fetch found #22 already merged upstream at `56880270`; the dependency
source matches our baseline. Implementation `5cf8eb0b` is published in [PR #23](https://github.com/404khai/astrid/pull/23),
targeting main and left unmerged. No PR merge performed here. Preserve unrelated `.gitignore`, historical CURRENT edits,
recovery files and untracked user assets outside the task commit.

---

# Foreground exact-target InstaBox tool — 2026-10-21

RE-01 is implemented on `feat/instabox-tool-adapter`, from fetched main `cc0fa964`,
in the same preserved checkout. The maintainer explicitly accepted
[ADR 0021](adr/0021-remote-target-authority.md) with atomic single-use consumption
at POST admission, the local OS scope clarification and future audit privacy/schema
work deferred. The accepted contract was committed before dependent code.

The host installs one exact target, opaque client and separate per-call broker.
Grants bind principal/run/runtime-call/provider-call/target and exact canonical
command/cwd/timeout. No authenticated metadata before approval; final checks and
atomic consumption admit at most one POST. Local presets, resumed sessions and
read-only workers grant no typed remote authority. No retry/replay; dispatched
cancellation/deadline/transport/malformed responses preserve unknown outcome.
Actual outputs and exit status are retained only when observed, and incomplete
cancelled batches still reject continuation. Exact targets stay in private
approval/result data; metadata traces omit commands/outputs/paths/errors/credentials.
No private v2 schema/continuation or budget changes; ADR 0008 off semantics remain.

**302 Rust passed, 0 failed, 1 ignored; 2 Python passed; fmt, strict all-target
Clippy and whitespace passed.** Initial parallel Git-observation assertion failure
passed isolated and full serialized reruns without observer changes; see
[review](reviews/instabox-tool-adapter-2026-10-21.md),
[evidence](evaluations/instabox-tool-adapter-2026-10-21.json),
[active plan](plans/remote-mobile-cli.md), [milestone](milestones/runtime-recovery.md).

This is a mocked foreground host-installed runtime adapter. Normal CLI sessions
still do not implicitly select a target, load an InstaBox key or install a broker;
standalone InstaBox CLI remains supported. No live provider/cloud/VM action or
credential/private-session content read. No remote workers, computer use, mobile
networking, Phase 9, daemon, PTY or desktop work. Preserve Meister primary/Astrix
specialized and Rust harness authority. Implementation `5ec7fdd6` is published as
[PR #22](https://github.com/404khai/astrid/pull/22), targeting main and left unmerged.
Next proposed slice is RE-02's separately authorized exact isolated live trial and
foreground CLI broker connection; do not start it here.

Unrelated `.gitignore`, historical CURRENT content, user assets and recovery files
remain preserved and unstaged.

---

# Aggregate tool-exchange admission — 2026-10-21

Maintainer authorized RC-03 on a new branch, then its PR, merges of the open
implementation PRs and closure of Tauri draft #13. Current branch:
`feat/aggregate-tool-admission`, parent `fd419fb6`, same preserved checkout.
No new conversation thread could be created through the available tools.

The guard checks original/current requests, instructions and the new assistant/
actual-result prefix before the first tool and between results. Over-budget
prefixes stop pending tools and further inference, retaining all actual results
and mutations. No synthetic outcomes, per-tool truncation, budget increase or
private v2 change. Skipped incomplete batches still require `/new`. A prefix
that fits cannot guarantee future results fit; this is not a total-memory bound.

**288 Rust passed, 0 failed, 1 ignored; 2 Python passed; fmt and strict all-target
Clippy passed.** The original runtime fails the later-mutation regression.
[PR #21](https://github.com/404khai/astrid/pull/21) publishes implementation
`b87218d4` and targets main after its dependencies merged; [RC-03 review](reviews/aggregate-tool-admission-2026-10-21.md),
[active plan](plans/remote-mobile-cli.md), [milestone](milestones/runtime-recovery.md).
#19 merged as `a0b42bed54c4d19acb42a310eba70d22334c6fd3`; #20 was retargeted
and merged as `413c82203dbd2f68efbc76f310ba183edac3c033`. Fetched main's
source tree matched each reviewed dependency exactly. #13 is closed, not merged;
its branch remains at `8baf6af826ce4e4fed200d31e35b93d4e00f06b2` and paused
source is preserved. RC-03 has passed local checks and is published; consult
PR #21 for final GitHub merge disposition. Next work is RE-01's concrete remote
target-authority ADR, then its smallest mocked InstaBox adapter.
No live inference/InstaBox actions, provider/private-session reads or successor
implementation. Original unrelated `.gitignore`, local checkpoint history,
user assets and recovery data remain preserved.

---

# Recovery PR published; live token usage — 2026-10-18

Committed recovery as `dc7cbfbc` and opened
[PR #19](https://github.com/404khai/astrid/pull/19), targeting main. The nineteen
scoped files exclude initial unrelated edits, local incident artifacts and user
assets; all six validated source hashes matched the index and 125 index Markdown
links resolved. The original local checkpoint history remains preserved in the
working file, outside that commit. No merge or installed CLI refresh.

The maintainer authorized starting the next item after publication. RC-02 is
committed as `36ccc11d` on `feat/live-token-usage` and published as
[PR #20](https://github.com/404khai/astrid/pull/20), stacked on PR #19:
one optional typed provider-usage report per call, checked replay, per-call CLI
counts and known run totals with per-metric coverage. Observability off still
means no added telemetry collection or trace writes. Missing/invalid counts stay
unavailable, zero stays zero, cached is a subset of input. No quota/cost or model
window estimate is invented. No session v2/private-continuation format changes.

**283 Rust passed, 0 failed, 1 ignored; 2 Python passed; fmt and strict all-target
Clippy passed.** [Compatibility and solo review](reviews/live-token-usage-2026-10-18.md),
[active plan](plans/remote-mobile-cli.md), [milestone](milestones/runtime-recovery.md).
No live inference/InstaBox action, provider/InstaBox credential or private-session
content read, provisioning, port exposure, secret transfer, daemon, desktop or PTY
implementation. Existing GitHub CLI authentication is used for publication. Original
unrelated dirty files and recovery data remain intact. RE-01 awaits its concrete
target-authority contract after RC-02; no next phase is inferred.

---

# Recovery PR preparation — 2026-10-18

Maintainer authorized committing the completed recovery slice, opening a PR, then
starting RC-02 from the [active plan](plans/remote-mobile-cli.md). Recovery branch:
`feat/followup-context-recovery`, based on fetched `origin/main` `ea183c08` in the
same checkout; its baseline tree equals the previously validated `8dea6765` tree.
Scoped publication excludes unrelated `.gitignore`, pre-existing local checkpoint
history, incident artifacts, user assets and recovery files. Source hashes still
match the recorded **279/0/1 Rust**, **2 Python**, fmt/strict Clippy validation.
No phase transition, merge, live inference or VM operation.

---

# Runtime recovery and remote/mobile/CLI direction — 2026-10-18

Maintainer requested root-cause context/cancellation recovery before remote
computer use, reuse of the existing Expo mobile app for iOS/Android, and CLI
infrastructure research. Separately accepted [ADR 0020](adr/0020-followup-context-recovery.md)
allows the latest historical submission's earlier complete exchanges to compact
while retaining its question and actual latest exchange. Original private v2
history, continuation, provenance, permissions and incomplete-batch denial remain.

Implemented that selector/event-validation repair and CLI display of model-call
ceiling, response planning reserve and request byte ceiling. Budget defaults are
unchanged. New deterministic fixtures exercise oversized follow-ups, cancellation
after complete exchanges, no repeated edit, original history retention, summary
lineage, private-reasoning exclusion and event replay. The same new unit regression
fails with `ProtectedOverflow` under the original selector. Final validation:
**279 Rust passed, 0 failed, 1 ignored; 2 Python passed; fmt/strict all-target
Clippy and whitespace passed**. See [milestone](milestones/runtime-recovery.md) and
[solo review](reviews/runtime-recovery-2026-10-18.md).

[Active dependency-ordered plan](plans/remote-mobile-cli.md): RC-01 recovery,
RC-02 honest live usage projection, then RE-01 typed InstaBox execution. Mocks can
support mobile platform work independently; real mobile networking waits for a
concrete authenticated Host/trust contract. Computer use needs bounded image
admission and distinct observation/action grants. CLI starts with existing status
protocols and a tmux/Herdr reuse experiment, not a new PTY multiplexer. Live usage
counts in the normal CLI projection and the runtime remote adapter are not yet
implemented; merged standalone InstaBox CLI execution remains independent.

Same `feat/tui-redesign` checkout, baseline `8dea6765`; initial dirty changes and
unrelated assets/recovery files preserved. No credential/private-session read,
live inference/cloud/VM action, provisioning, port exposure, secret transfer,
branch/worktree switch, staging, commit or publication. M02 remains paused; no
Phase 9, concurrent writes/shell, daemon or desktop implementation.

---

# Terminal redesign PR published — 2026-10-18

[PR #18](https://github.com/404khai/astrid/pull/18) targets main after merged
[InstaBox PR #17](https://github.com/404khai/astrid/pull/17) (`5d423a7d`).
Implementation `8ddc037c` contains exactly twelve scoped console/evidence files;
no InstaBox/runtime changes or unrelated user files. Exact source validation:
276 Rust passed, 0 failed, 1 ignored; 2 Python passed; fmt and strict Clippy passed.
Source hashes and historical evidence are retained in the review/evaluation below.
Maintainer authorized both merges; GitHub checks and PR #18 merge remain at this
checkpoint. No live VM commands, credential reads, desktop or phase advancement.

---

# Terminal redesign publication review — 2026-10-18

Maintainer requested a separate PR and merge for the named Redesign Astrid Terminal
UI session, after completing/merging InstaBox. PR #17 is now merged at `5d423a7d`
with isolated 271/0/1 Rust and 2 Python evidence. This new TUI PR targets that main
baseline and contains no InstaBox implementation changes.

Reviewed the session's latest exchanges and presentation code, including the user
correction restoring the original mascot beside the ARC-gradient ASTRID wordmark.
Centered/adaptive welcome, shaded composer, exact mode colors and narrow/monochrome
fallbacks retain existing runtime, permissions, scrollback, cancellation, sessions
and trace behavior. No desktop work or phase advancement.
[Scope and visual review](reviews/tui-redesign.md),
[exact publication evidence](evaluations/tui-redesign-validation-2026-10-18.json).

Exact staged source export: 276 Rust passed, 0 failed, 1 ignored; 2 Python passed.
Formatting and strict all-target Clippy passed. Tests used deterministic mocks
and PTY fixtures, with no live VM commands or credential reads.

Private credentials/sessions, local incident reports, ignore changes and unrelated
user/reference/build files are excluded and preserved. Same checkout and branch;
no worktree or main switch. Earlier checkpoint statements retain their original
implementation/publication-time scope.

---

# InstaBox CLI completion for merge — 2026-10-18

Maintainer requested completion and merge of PR #17. Optional metadata discovery,
local `.env` key fallback and separately granted bounded CLI exec on an explicit
existing ID are now reviewed together. Corrected the interrupted executor's wire
fields, cwd/timeout defaults, bounded metadata/response waits, phase-aware Ctrl-C,
unknown-outcome/error/output reporting and nonzero CLI exit behavior. No lifecycle,
transfer, runtime/model tool, scheduler grants or phase advancement. No credentials
read, account claims or live VM commands.
[Completion review](reviews/instabox-completion-2026-10-18.md),
[exact isolated-source evidence](evaluations/instabox-completion-2026-10-18.json),
[setup/research](research/instabox-discovery.md).

Isolated exported source checks: **271 Rust passed, 0 failed, 1 ignored; 2 Python
passed**; format, strict all-target Clippy and whitespace passed. Fifteen focused
synthetic InstaBox tests passed; no live cloud acceptance is claimed.

Console redesign work is excluded from this InstaBox commit/PR and will be reviewed
and published separately after its merge. User files, private sessions and recovery
data remain preserved in the same checkout. Prior publication/checkpoint counts
below retain their original historical source scope.

---

# InstaBox PR published — 2026-10-18

Opened [PR #17](https://github.com/404khai/astrid/pull/17),
**feat(instabox): add opt-in metadata discovery**, from `feat/instabox-discovery`
against `main` at the maintainer's explicit request. Implementation commit:
`0fb9f603`. Exactly nine scoped files; no unrelated user artifacts or scheduler
implementation changes. No merge, rebase, main switch, worktree or phase advancement.
Scheduler PR #16 was verified merged at `f3d2f3cd`; its tree equals the original
InstaBox baseline `c9efb084`, so `main` is the correct base.

Pre-publication review verified all four code/test hashes against the staged tree,
154 local Markdown targets and clean scoped whitespace. Prior exact-source checks
remain **263 Rust passed, 0 failed, 1 ignored; 2 Python passed; fmt/strict Clippy
passed**. Publication/setup notes change documentation only; no new runtime or
live-cloud acceptance is claimed. [Report and key setup](research/instabox-discovery.md),
[validation evidence](evaluations/instabox-validation-2026-10-18.json).
No API key was read, persisted or sent to chat/Git. No authenticated InstaBox call,
remote execution, lifecycle change or file transfer was performed. Remote target
execution/security contracts remain proposed; unrelated files/recovery data retained.
The following checkpoints preserve their original pre-publication state.

---

# InstaBox publication preparation — 2026-10-18

Maintainer explicitly authorized scoped staging, conventional commit, push and PR
for the completed discovery experiment. Branch `feat/instabox-discovery`, same
checkout. Fetched main `f3d2f3cd` contains merged scheduler PR #16 and has the
identical tree to the original `c9efb084` baseline, so the PR targets `main` with
only the nine InstaBox files. No rebase/merge, main switch or phase advancement.

Full pre-publication review and all four recorded code/test hashes match the
previous **263/0/1 Rust**, **2 Python**, fmt and strict Clippy evidence; only setup
and publication documentation changed. Hidden zsh key entry guidance is in the
[report](research/instabox-discovery.md); no `.env` loading, key persistence,
credential inspection, authenticated cloud requests or remote machine actions.
Unrelated assets and recovery data remain outside the scoped index.

---

# Optional InstaBox discovery experiment — 2026-10-18

Completed the user-authorized official-source research and local metadata discovery
spike on `feat/instabox-discovery`, created from existing HEAD `c9efb084` after the
user explicitly superseded the initial no-branch-switch instruction. Same checkout;
no main switch/worktree, staging, commit or publication. Unrelated user files,
private sessions and recovery data preserved.

`astrid instabox discover --allow-discovery [--machine <exact-id>]` performs bounded
metadata GETs only, with separate explicit discovery consent and environment API-key
auth. No VM connection, exec, auto-resume, lifecycle, transfer, snapshot or desktop
implementation. No authenticated/live cloud tests or real credential reads.
[Research, source links, commands, proposed future contracts and solo review](research/instabox-discovery.md).
[Exact-source validation](evaluations/instabox-validation-2026-10-18.json): **263 Rust
passed, 0 failed, 1 ignored**, **2 Python passed**; fmt, strict all-target Clippy and
whitespace passed. The report retains an initial sandbox bind restriction and an
existing nested-compilation timeout; isolated fixture and final full reruns passed.

Phase 8 scheduler/PR #16 disposition below remains authoritative: implementation
review complete, closure with limitations recommended, maintainer acceptance
pending. No phase advancement, remote worker grants or durable jobs. Next scoped
implementation action is a proposed target-authority/cancellation/trace ADR and
mocked exec contracts; no live remote action is authorized by this checkpoint.

---

# Scheduler PR publication — 2026-10-16

Opened [PR #16](https://github.com/404khai/astrid/pull/16), **Add bounded read-only task
scheduling and workers**, from `feat/task-scheduler-workers` against `main` at the
maintainer's request. Implementation commit: `be701c13`, 33 explicitly scoped files.
Same checkout; no branch/worktree change or unrelated assets included. Recovery
snapshots/stash and private sessions remain preserved. No merge or successor work.

Publication verified all 86 pinned source hashes against the exact Git index and
clean scoped whitespace. Prior exact-source validation remains 255 Rust passed,
0 failed, 1 ignored; 2 Python passed; fmt/strict all-target Clippy passed. This
publication adds no runtime code, tests or inference. Original live evidence and
its 1/3 complete child-trace coverage remain unchanged. [Guide](scheduler.md),
[review](reviews/phase-8.md), [plan](plans/phase-8.md). Historical checkpoints below.

---

# Scheduler and workers — implementation and review complete, 2026-10-16

P8-01–05 Done on `feat/task-scheduler-workers`, main baseline `16d755c4`, same
checkout. Bounded static DAG, exclusive owned fresh workers, hard read-only grant
intersection, independent cancellation/deadlines, attributed committed-result edges,
protected mandatory inputs, deterministic source bundle, checked batch lifecycle,
separate metadata traces/shared quotas and `astrid batch run/trace` are implemented.
The auth orphan wait was reproduced first and fixed with cancellable nonblocking
exclusive acquisition plus explicit guard unlock; account/credential validation
and private idle-session v2/original continuation remain intact.

Final validation: **255 Rust passed, 0 failed, 1 ignored**; **2 Python passed**;
fmt, strict all-target clippy and whitespace passed. [Pinned source/commands](evaluations/phase-8-validation-2026-10-16.json),
[deterministic acceptance](evaluations/phase-8-mocked-2026-10-16.json),
[guide](scheduler.md), [plan](plans/phase-8.md), [solo exit review](reviews/phase-8.md).

One frozen bounded live notes batch passed separate solo factual grading: 29.000902 s,
6 model calls/4 tools, original results retained, fixture bytes unchanged.
[Original live evidence](evaluations/phase-8-live-2026-10-16.json). Batch metadata
complete, child coverage 1 complete/2 unavailable; missing recording causes were
not retained by that earlier CLI. No cause is invented or second live batch spent.
Whole-batch usage/cost remain unknown. Final explicit reports retain writer diagnostics
without persisting arbitrary errors/content into metadata. No speed/quality superiority.

Review recommends **close with explicit limitations**; maintainer acceptance pending.
The maintainer explicitly authorized this Phase 8 transition; no separate Phase 7
administrative acceptance is invented. Stop here: no automatic Phase 9. Work is
uncommitted, no broad staging/PR, branch/worktree switch or unrelated artifacts.
Recovery `/Users/admin/Developer/astrid-scheduler-handoff-5c3hdhrj`, earlier stash
“preserve tracked changes before main-based phase6 closure and phase7 handoff” and
`/Users/admin/Developer/astrid-phase7-handoff-ejp1viwq` remain preserved. M02 stays
Paused/unaccepted, ADR 0014 Superseded, Phases 5/13 Deferred. Historical evidence follows.

---

# Scheduler metadata checkpoint — 2026-10-16

P8-01–04 complete on `feat/task-scheduler-workers`, same preserved checkout.
Batch lifecycle has checked replay, child outcome/recording coverage and separate
optional metadata traces. Flat generated `batch-<UUID>` names share existing total
store quotas and writer exclusion with unchanged schema-1 run traces. Existing
run lists/statistics exclude batch aggregates. `astrid batch trace [id]` inspects
metadata without authentication or recovery claims. Result text, labels, paths,
content digests, arbitrary errors and private continuation stay out of traces.

Full validation: **246 Rust passed, 0 failed, 1 ignored**; fmt and strict all-target
clippy passed. [Pinned evidence](evaluations/phase-8-slice-3.json), [plan](plans/phase-8.md),
[solo review](reviews/phase-8.md). P8-05 In progress: bounded manifest execution,
CLI acceptance, one frozen bounded live scenario and final review. No live
inference yet. No staging/commit, branch/worktree change or Phase 9. Recovery
snapshots/stash and unrelated files retained. Historical checkpoints follow.

---

# Scheduler dependency and cleanup checkpoint — 2026-10-16

P8-01–03 complete on `feat/task-scheduler-workers`, same preserved checkout.
Static DAG admission, all-required dependencies, ordered attributed bounded final
text and deterministic original-result bundle are implemented. Unverified synthesis
cannot replace sources. Failures skip descendants; unrelated work continues.
Task/root cancellation, deadlines, panic, actual compaction and real auth-lock
contention/refresh are covered; cleanup failure keeps failure precedence.

New full validation: **237 Rust passed, 0 failed, 1 ignored**; fmt and strict
all-target clippy passed. [Pinned evidence](evaluations/phase-8-slice-2.json),
[solo review](reviews/phase-8.md), [plan](plans/phase-8.md). No live inference.
P8-04 In progress: checked metadata inspection and separate optional batch trace
family under shared store quotas. Run schema/idle v2 remain unchanged. No branch,
worktree, staging or commit; recovery snapshots/stash and unrelated files preserved.
No Phase 9. Historical checkpoints follow.

---

# Scheduler implementation checkpoint — 2026-10-16

P8-01 complete on `feat/task-scheduler-workers`, same checkout; no branch/worktree
change, staging or commit. Auth blocking-lock orphan reproduced with synthetic
fixtures and fixed without removing exclusive account locks. Headless owned
read-only tasks have barrier-proven overlap, separate sessions/tokens, bounded
admission/results/lifecycle, independent event drains and explicit projection
detach. Existing run events, trace schema and idle v2 remain unchanged.

New validation: `cargo test --locked` **224 passed, 0 failed, 1 ignored**;
fmt, strict all-target clippy and whitespace passed. [Pinned evidence](evaluations/phase-8-slice-1.json),
[solo review](reviews/phase-8.md), [active plan](plans/phase-8.md). No live inference.
P8-02 In progress: add validated dependencies and bounded attributed final-text
transfer, preserving original results and mandatory context. Accepted ADRs and
implementation authorization remain in force; no Phase 9 advancement. Recovery
snapshots/stash and unrelated files remain preserved. Historical handoff below.

---

# Implementation thread handoff — 2026-10-16

Maintainer approved ADRs 0017–0019 and instructed “I approve begin implementation,
new thread, new branch.” Phase 8 implementation is authorized. [Plan](plans/phase-8.md),
accepted ADRs [0017](adr/0017-bounded-task-dag.md),
[0018](adr/0018-worker-authority-and-results.md),
[0019](adr/0019-batch-events-and-traces.md). P8-01 is Ready for the requested new
implementation thread; subsequent tickets retain their dependencies. Phase 7
closure remains recommended with limitations; no separate administrative acceptance
is fabricated or required to repeat the maintainer's explicit transition.

Branch `feat/task-scheduler-workers`, created directly from freshly fetched
`origin/main` at `16d755c4f04c70d07eed23e9302f725176f3960f` in the same checkout;
no worktree. All ten pre-existing planning files preserved byte-for-byte across
switch before these authorization edits. Unrelated untracked artifacts retained.
New recovery snapshot: `/Users/admin/Developer/astrid-scheduler-handoff-5c3hdhrj` (manifest.json, tracked.patch, files/).
Keep earlier recovery stash and `/Users/admin/Developer/astrid-phase7-handoff-ejp1viwq`.

Naming preference added to AGENTS: use descriptive branch/PR titles, normally omit
phase numbers; phase/ticket references belong in descriptions/plans/evidence.
No Rust implementation, new tests or live inference during this handoff. Prior
baseline remains 218 Rust passed/0 failed/1 ignored, 2 Python passed, fmt/strict
clippy; do not call those Phase 8 validation. Handoff checks passed: 390 local Markdown targets resolve, accepted ADR/Ready ticket
statuses verified, new-document whitespace clean and source hashes unchanged.
`git diff --check` passed; no new runtime tests claimed.

New thread: inspect CURRENT/AGENTS/plan and accepted ADRs, reproduce the concrete
auth-lock cancellation concern with synthetic fixtures, then implement two owned
read-only workers with barrier-proven overlap. Continue authorized slices with
required tests/fmt/strict clippy and checkpoint updates. Preserve private idle v2,
existing run events/traces, permissions and original continuation. No concurrent
writes/shell, worktrees, retries/durable leases, daemon/desktop, harness adapter,
Phase 9 or later scope. M02 Paused/unaccepted; 5/13 Deferred.

Earlier planning-only checkpoints below preserve history and are superseded by
this explicit authorization. Work remains uncommitted; do not broadly stage assets.

---

# Phase 8 planning checkpoint — 2026-10-16

Maintainer authorized planning for scheduler/workers: dependencies, ownership,
cancellation, permissions and result reconciliation. [Plan](plans/phase-8.md),
[readiness review](reviews/phase-8-readiness.md). Documentation only; no scheduler
source, new tests or live inference. Existing checkout/branch retained at `395af2c3`;
no worktree, staging or commit. Unrelated user files and recovery data preserved.

[PR #15](https://github.com/404khai/astrid/pull/15) is merged at `16d755c4` (verified
through GitHub). Phase 7 exit evidence supports closure with limitations; no separate
administrative closure/Phase 8 implementation approval is inferred from that merge
or this planning instruction. Prior exact-source validation remains 218 Rust passed,
0 failed, 1 ignored; 2 Python passed; fmt/strict clippy passed. Not a new rerun.

Completed planning ticket P8-00: source audit, prior-phase exit/debt review, proposed
state/authority/cancellation/reconciliation contracts, five implementation tickets
P8-01–05 plus the planning ticket, failure matrix and
validation criteria. All implementation tickets remain Draft.

Three consequential proposals await maintainer resolution:
[ADR 0017](adr/0017-bounded-task-dag.md) bounded static process-local DAG, exclusive
one-attempt ownership, scoped cancellation and continued unrelated work;
[ADR 0018](adr/0018-worker-authority-and-results.md) read-only grant intersection,
declared visible-result transfers and source-preserving reconciliation;
[ADR 0019](adr/0019-batch-events-and-traces.md) explicit projection detach on overflow,
bounded returned lifecycle log and separate metadata-only correlation traces.
No recommendation is silently accepted. Auth blocking-lock lifetime and independent
child event draining are concrete implementation prerequisites to test.

Proposed first slice: two independently owned read-only tasks overlap headlessly,
with deterministic barrier evidence, then a dependent reconciliation task. No shared
private history or stdin approvals, writes/shell, worktrees, retry/lease/durable
scheduler, harness adapter or daemon. Session v2 and existing run traces stay intact.
M02 Paused/unaccepted; 5/13 Deferred. Next: review contracts, record Phase 7 disposition
and separate implementation authorization before P8-01 can become Ready.

Planning validation passed: 385 local Markdown file targets resolve against tracked
plus scoped planning files; all three ADRs remain Proposed and all five implementation
tickets Draft. Source hashes match prior Phase 7 evidence; tracked source/tests/scripts
unchanged. New-document whitespace and `git diff --check` passed. No cargo/live
rerun for documentation-only work. Earlier publication and implementation checkpoints
below are historical.

---

# Publication checkpoint — 2026-10-16

Opened [PR #15](https://github.com/404khai/astrid/pull/15) against `main` from
`feat/context-meter-routing-comparisons`. Implementation commit: `cdc6e95c`,
33 explicitly scoped files. No worktree/branch change, force push or unrelated
artifacts included. Recovery stash/snapshot remain preserved.

Publication verified pinned source hashes against the exact index and all 339 local
Markdown file targets within committed files, independent of excluded user artifacts.
Previously completed exact-source checks remain: 218 Rust passed, 0 failed, 1 ignored;
2 Python passed; fmt, strict all-target clippy and whitespace passed. Publication
adds no runtime change or new live inference; final checkpoint changes are documentation only.
See the [review](reviews/phase-7-followups.md) and [active plan](plans/phase-7-followups.md).

Next agenda: maintainer review/merge and explicit Phase 7 closure disposition, then
Phase 8 planning for task ownership, dependencies, worker state, cancellation,
permission boundaries and explicit result reconciliation. Phase 8 implementation
still requires separate authorization. Worktree isolation remains Phase 9; M02
Paused/unaccepted and Phases 5/13 Deferred. Earlier uncommitted checkpoints below
are historical.

---

# Active follow-up — 2026-10-16

PR #14 merged at `6608e26b`. Maintainer authorized context usage, controlled routing
quality comparisons and resumable routing, in that order, on a new branch.
`feat/context-meter-routing-comparisons` is based on fetched `origin/main`, same
checkout, no worktree. [Active plan](plans/phase-7-followups.md).

P7-05–07 implemented and solo-reviewed: default context budget meter, controlled
coding/notes comparisons, resumable routing under accepted
[ADR 0016](adr/0016-persistent-routing-provenance.md). The older one-shot restriction
and v1-only format are superseded. V2 records response dispatch origins and durable
replay mode. V1 history loads without writing, remains unknown, and follows visible
history; atomic idle save publishes v2. Original private bytes are retained; tools
are not replayed. Older binaries reject v2. Same-source native continuation remains.

Final validation: **218 Rust passed, 0 failed, 1 ignored**; **2 Python passed**;
fmt, strict all-target clippy and diff whitespace passed.
[Follow-up review](reviews/phase-7-followups.md),
[pinned checks](evaluations/phase-7-followup-validation-2026-10-16.json).
Meter labels the non-opaque bytes/4 estimate and configured input admission budget;
partial opaque occupancy and the actual model window remain unknown. Idle values
are the last prepared selection, not unsent text or total stored history.

Eight new live synthetic comparison episodes passed their task-specific rubrics:
fixed Sol/low vs Sol→Terra/low, two repetitions each on identical coding and notes
fixtures. [Live comparison](evaluations/phase-7-comparison-2026-10-16.json).
This small sample shows no quality superiority. Routed coding input was higher;
cost, true TTFT/decode and model-window occupancy remain unavailable. These live
runs preceded provenance implementation; restart/migration evidence is deterministic.

Installed CLI refreshed with locked/offline `cargo install --path .`; `astrid --help`
smoke passed without inference or loading existing sessions. 339 local Markdown
targets resolve and pinned source/comparison hashes match.

Work remains on the new branch, uncommitted; no broad staging or unrelated assets
included. No Phase 8. M02 stays Paused/unaccepted; Phases 5/13 Deferred. Recovery
stash/snapshot and unrelated untracked assets are retained. Earlier checkpoints
below are historical; 211/0/1 is the preceding merged baseline.

---

# Publication checkpoint — 2026-10-15

Opened [PR #14](https://github.com/404khai/astrid/pull/14) against `main` from the
existing `feat/phase-6-closure-phase-7-routing` branch. Implementation commit:
`ef1dbb8` (72 explicitly scoped files). No branch/worktree change or force push.
The PR includes carried headless notes/terminal prerequisites and supporting
history/evaluation records; desktop source, experiments, personal household notes,
reference images, build output and the local product brief remain outside it.
Recovery stash and handoff snapshot remain preserved.

Validated an exported copy of the exact staged code/test tree independently of
excluded local artifacts: `cargo test --locked` **211 passed, 0 failed, 1 ignored**;
fmt and strict all-target clippy passed. All 208 checked local Markdown targets
resolved in that snapshot; staged source hashes match the Phase 7 validation record.
This publication-only checkpoint adds no runtime change or new live inference.

Maintainer reported missing compact context usage. Current detailed inspection is
`astrid run ... --show-context`; no percentage/footer meter is implemented.
Recommended next scoped follow-up: an always-visible context budget meter with
explicit estimate/unknown labels, provider-reported last-call input tokens when
available, and a model-window percentage only with verified capacity/accounting.
Existing non-opaque JSON bytes/4 estimates and configured budgets must not be
presented as exact full model-window occupancy. Follow with controlled routed vs
single-model quality comparisons and separately decide provenance before resumable
routing. This note does not authorize Phase 8 or a persistence-schema migration.

Earlier uncommitted/publication-pending statements below are historical.

---

# Current checkpoint — Phase 7 implemented and reviewed, 2026-10-15

Phase 6 is **Closed with explicit limitations** under the maintainer's delegated
review authorization; [closure](reviews/phase-6.md) and actual configuration
[comparison](evaluations/phase-6-comparison-2026-10-15.json). Its fresh main-based
pre-routing regression was 203/0/1; the older 213 result included ten M02 host tests.

Phase 7 P7-01–04 are implemented. [Solo review](reviews/phase-7.md) recommends
closure with explicit limitations; [plan](plans/phase-7.md), accepted
[ADR 0015](adr/0015-visible-history-routing.md). Deterministic direct-inference
stages, validated exact-model effort, per-call route events/trace metadata and CLI
flags are complete. Visible-history handoff excludes private continuation while
retaining original messages. Per maintainer's explicit refinement, multi-stage
routing is fresh one-shot only: returned history rejects serialization, saving
and continuation. Native/effort-only snapshot schema and continuation stay intact.

Final source: `cargo test --locked` **211 passed, 0 failed, 1 ignored**;
fmt, strict all-target clippy and whitespace passed. [Pinned validation](evaluations/phase-7-validation-2026-10-15.json).
Six routed invariant tests, ten evaluation execution cases, native persistence,
permissions, cancellation/context and new duplicate-descriptor lock-release test
pass. Intermediate test failures, transient lock observation and fixes remain in
the review; final clean run is distinct evidence. Runner hashes match reports.

One live synthetic notes sample exercised Sol/low then Terra/low and passed solo
frozen-rubric grading: 3 model calls, 4 tools, zero approvals/changes, 3313 input,
278 output, 0 cached tokens. [Live record](evaluations/phase-7-live-2026-10-15.json).
This is limited quality/protocol evidence, not a routing benefit or general model
ranking. Cost/true TTFT/decode remain unavailable. Effort support is deliberately
limited to verified exact Sol/Terra identifiers; stages are caller configuration.

Implementation and requested review are complete; stop at Phase 7. Maintainer
acceptance/any Phase 8 transition remain separate decisions, not inferred here.
M02 Paused/unaccepted; ADR 0014 Superseded; 5/13 Deferred; GPUI Phase 11 only.
Same main-based branch/checkout, no staging/commit, branch/worktree change, desktop
import or discarded user files. Recovery stash/snapshot below remain preserved.
The preceding checkpoints below are historical and superseded by this entry.

---

# Current checkpoint — Phase 7 implementation, 2026-10-15

Phase 6 **Closed with explicit limitations** under delegated evidence review;
[closure](reviews/phase-6.md), [comparison](evaluations/phase-6-comparison-2026-10-15.json).
Fresh main-based checks: `cargo test --locked` **203 passed, 0 failed, 1 ignored**;
fmt, strict all-target clippy and whitespace passed. Ten M02 host tests are absent.
Two context configurations passed five scripted execution tasks each; one additional
recency live notes sample passed solo grading against the frozen rubric. This does
not establish policy superiority or general model quality.

Phase 7 implementation now active and authorized; [plan](plans/phase-7.md).
[ADR 0015](adr/0015-visible-history-routing.md) accepted explicitly by maintainer:
visible-history handoff, private original history retained, no opaque cross-model
replay. Typed routes/capabilities/effort, exact accounting and per-call route events now
implemented; five focused routing tests pass. Full post-routing checks pending.
Maintainer approved the one-shot inspection-only contract: routed Session save,
serialization and continuation now fail explicitly; native idle schema unchanged.
Live fresh routed notes sample passed the frozen rubric with Sol then Terra/low.
Final post-refinement cargo checks pending; no resumable-routing claim.
Same main-based branch/checkout, no staging/commit/worktree or discarded files.
Recovery data listed below remains preserved. M02 Paused; 5/13 Deferred; no Phase 8.

---

# Phase 6 closure / Phase 7 thread handoff — 2026-10-15

Current authorized action: review Phase 6 closure recommendation against the new
main baseline; address missing evidence; record closure disposition when justified;
then plan and implement Phase 7 under the maintainer's explicit new-thread request.
The older pending-authorization entries below are historical. Consequential new
routing/provider contracts still require the established decision workflow.

Branch: `feat/phase-6-closure-phase-7-routing`, based directly on fetched
`origin/main` `2deea5f` in `/Users/admin/Developer/astrid`; no worktree. M02 remains
on `feat/m02-tauri-desktop` at `8baf6af` and draft PR #13; no desktop commit merged.
Carried existing tracked edits byte-for-byte and retained untracked files. Recovery:
`git stash` entry “preserve tracked changes before main-based phase6 closure and
phase7 handoff” plus `/Users/admin/Developer/astrid-phase7-handoff-ejp1viwq`
(manifest and tracked patch). Do not drop/remove these during unrelated cleanup.

The prior **213/0/1** result included ten M02 host tests. It is not fresh evidence
for this main-based branch. Main omits the M02-introduced `PermissionPolicy::read_only`
helper used by uncommitted M01/Phase 6 work; inspect and reconcile the minimum
headless prerequisite, rather than importing the desktop host. Existing paused
M02 README/review edits remain as preserved files; some historical links target
files present on the M02 branch only. Preserve provenance and repair navigation
without pretending the whole draft is integrated.

Start with [Phase 6 review](reviews/phase-6.md), [plan](plans/phase-6.md),
[roadmap](roadmap.md), AGENTS and the development workflow. Re-run relevant checks
on this branch; existing live results are historical evidence and need not incur
another model call unless the review identifies a concrete need. Update checkpoint,
phase marker and active plan with actual closure/transition evidence.

---

# Astrid development checkpoint

Updated: 2026-10-15
Baseline: `8baf6af` plus preserved dirty/untracked M01/planning/user files.
Branch: `feat/m02-tauri-desktop`, same checkout, no worktree or branch change.

## Active scope and authority

**Phase 6 — Evaluation** is active. The maintainer's 2026-10-15 direction
explicitly authorizes reconciliation, prerequisite review, plan and implementation.
Phases 0–4 foundations and M01 acceptance remain with documented limitations.
Phase 4 is now historical; Phase 5 is Deferred, incomplete. Phase 13 is Deferred.
M02 is **Paused**, partial and unaccepted. ADR 0014 is Superseded, retaining its
2026-10-14 acceptance history. GPUI is the future desktop direction pending
Phase 11 investigation and scope; no replacement ADR or GPUI code now.
CLI development/validation and authoritative Rust runtime are the priority.

[Canonical roadmap](roadmap.md), [direction/pause audit](reviews/direction-revision-2026-10-15.md),
[Phase 6 plan](plans/phase-6.md). The historical milestone-first order below no
longer authorizes implementation. Later phases require reviewed evidence and
separate transition authorization; no routing/workers/interface work in this task.

## Current implementation and validation

Prerequisite audit complete; no consequential blocker identified. Five scripted
inference/real-tool baseline cases pass, as does the new real-PTY plain terminal
composer regression. Controlled live outstanding-notes sample passed the frozen
rubric in solo review: 27.04s wall, 3 model calls, 3 tool calls, 0 compactions,
0 approvals, 0 changed fixture paths. Usage available, cost/true TTFT unavailable.
See [live evidence](evaluations/phase-6-live-2026-10-15.json).

Current validation: `cargo test --locked` **213 passed, 0 failed, 1 ignored**;
format, strict all-target clippy and whitespace checks passed. 152 local Markdown
file targets resolved and recorded runner/rubric hashes matched. Baseline replay
again passed all five cases; [mock report](evaluations/phase-6-mocked-2026-10-15.json).
[Phase 6 review](reviews/phase-6.md) recommends closure with limitations. Implementation
is complete; maintainer closure and Phase 7 transition authorization remain pending.
Stop here; next authorized work requires that disposition. The installed CLI was
refreshed offline; actual `TERM=dumb astrid` reached input and `/quit` exited 0.
Plain canonical input now handles terminals unable to support the rich composer.
No model inference was used for that startup verification. M02's final visual/reload/live-active-quit gaps
remain paused, not silently closed. All existing dirty/untracked files preserved.

---

# Historical checkpoints through 2026-10-14

Updated: 2026-10-14
Baseline: `cfb60e8c0adb93a5fc663cf6e0d314d1e6c086c2` plus pre-existing worktree changes

## Active scope

- Historical phase marker: **Phase 4 — Trace and Inference Observability**.
- Latest completed milestone: [M01 — Personal task foundation](milestones/01-personal-tasks.md), accepted with limitations.
- Active milestone: M02 implementation in progress; host/private projection slices implemented,
  macOS product acceptance ongoing. See [M02 review](reviews/m02-desktop.md).
- Authorization: scoped M02 work authorized 2026-10-13; ADR 0014 option 1 and clauses
  1–10 accepted 2026-10-14 after the concrete recommendation and instruction to start.
  Liquid Glass remains required on macOS.
- Branch/checkout: `feat/m02-tauri-desktop`, `/Users/admin/Developer/astrid`;
  existing checkout, no worktree. Preserve all pre-existing dirty/untracked files.

## Complete and remaining

Phases 0–4 foundations exist; Phases 2–4 were administratively closed with
limitations on 2026-10-10. Accepted ADRs 0001–0012 cover the runtime, permissions,
context, traces and private idle snapshots. Non-Git folders already work.
Tauri 2 is selected; ADR 0013 is Superseded and ADR 0014 is Accepted (2026-10-14).

Canonical [roadmap](roadmap.md), nearest milestone plans and M00 validation/evidence
are complete. M01 implementation and acceptance passed with limitations on
2026-10-12. M02 now has a real Tauri client and application-lifetime Rust coordinator.
One live synthetic notes task passed its known-answer rubric through the desktop.
Independent job service, Astrix/group execution and connected Android remain later scope.
The web site/mobile demo do not supply those capabilities.

## Gates and next action

Finish M02 macOS material/accessibility, renderer/window and explicit quit acceptance,
record actual tests/builds and remaining gaps. ADR 0014 is resolved; no host gate remains.
Native glass uses public NSGlassEffectView (26+); pinned Tauri/Wry webview transparency
uses private drawsBackground and has a distribution compatibility limitation.
Reduce Transparency testing was explicitly authorized with restoration to the original
Off setting. Follow the evidence in [M02 review](reviews/m02-desktop.md) and
[material research](https://github.com/404khai/astrid/blob/8baf6af/docs/research/m02-liquid-glass.md). No later milestone is authorized.

Directory-free sessions, remote trust, independent service survival and recovery remain
later decisions. Historical checkpoints below describe the state at their dated entry;
they are not current gates.

## Validation and last verified checkpoint

2026-10-12 at the baseline above: `cargo test --locked` passed **189**, failed **0**,
ignored **1** (optional recording-overhead experiment). Format, strict all-target
clippy and diff whitespace checks passed; 70 local Markdown targets across 10
planning/navigation files passed validation. Known test failures: none observed.
No live model or desktop acceptance was performed. Details: [M00 evidence](milestones/00-foundations.md#completion-evidence).

M01 handoff, 2026-10-12: created `feat/m01-personal-tasks` from `feat/web` in the
same checkout and recorded scoped authorization. Documentation whitespace checks
passed; the Rust results above remain the preceding baseline, not a rerun or M01
acceptance. Start the new thread with M01-01; no code was changed by this handoff.

M01 verified checkpoint, 2026-10-12: uncommitted changes on the same baseline
and branch. `cargo test --locked`: **199 passed, 0 failed, 1 ignored**; format,
strict all-target clippy, diff whitespace and build checks passed. Seven headless
M01 tests, two CLI driver/parsing tests and one CLI subprocess test passed; existing
coding, session, context and permission regressions passed. No new schema or mode
meaning was introduced. Previous handoff/baseline entries above remain history.

The real `run --notes --folder` CLI completed three synthetic non-Git scenarios
using existing OpenAI/Codex subscription auth and `gpt-5.6-sol`. Solo frozen-rubric
review passed all three, including conflict and incomplete coverage. Zero observed
fixture mutations, model shell dispatches or approvals; no summary compactions.
Usage, wall times, checklists and limitations:
[live record](evaluations/m01-2026-10-12.json) and
[M01 evidence](milestones/01-personal-tasks.md#completion-evidence).
Scripted output is execution evidence only. One provider/model and one run per
scenario do not establish general personal-task reliability; unavailable cost,
true TTFT and decode throughput remain unavailable.

Repeat validation:

```sh
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

All pre-existing dirty/untracked files remain; no staging, commit, reset, cleanup,
branch change or worktree was performed. This is an uncommitted checkpoint.
Historical Phase 4/foundation closure remain unchanged. M02 was subsequently
authorized on 2026-10-13; ADR 0014 acceptance remains required. See
[development workflow](development.md).

## Authorized M01 follow-up: compact workspace evidence

2026-10-12: Maintainer reported post-answer build-cache spam and requested a fix
and PR. The CLI now collapses unavailable content comparisons into a count and
prints at most 20 other workspace change details. Full runtime evidence and
native mutation patches are preserved; unknown content is not called unchanged.
See [focused review](reviews/cli-workspace-evidence-noise.md).

Validation in the current checkout: `cargo test --locked` **201 passed, 0 failed,
1 ignored**; `cargo fmt --check`, strict all-target clippy and `git diff --check`
passed. The focused PR stages only console state/tests and its review; the M01
implementation and pre-existing user files remain separately uncommitted. This
follow-up authorizes no later milestone.

Published follow-up: commit `cfb60e8`, [PR #12](https://github.com/404khai/astrid/pull/12)
against `main`; verified exactly the three scoped files. Refreshed the installed
CLI with `cargo install --path . --locked --debug --offline`. Restart Astrid to
use the updated executable. No other files were staged or committed.

## M02-00 investigation checkpoint — 2026-10-13

Reviewed runtime/session/permission source, accepted ADRs and actual M01 evidence;
directly viewed all three primary Figma exports. Mapping, official Apple/Tauri
sources, installed SDK evidence and visual findings are recorded in
[Liquid Glass research](https://github.com/404khai/astrid/blob/8baf6af/docs/research/m02-liquid-glass.md). Independent AppKit/WK and
Tauri material fixtures live under `experiments/m02-glass/`; no Astrid host,
model execution, app IPC command or capability grant is implemented.

Native NSGlassEffectView is available on this macOS 27.0/Xcode 27.0 environment,
introduced in macOS 26. Native content visibly renders glass; WK content with
public background API alone remains opaque. The pinned Tauri transparency path
uses Wry private API; its feasibility and compatibility are assessed separately.
App-window screenshots and contrast point samples are retained. Forced opaque
fallback is simulated; actual Reduce Transparency switching and full accessibility
acceptance remain pending. Do not equate this experiment with desktop acceptance.

ADR 0014's exact recommended host/IPC/privacy/window/quit contract was presented
for acceptance in this thread. Response pending; M02-01–04 remain Draft. No M03,
daemon, recovery, remote trust, screen observation/control or mascot scope added.
The historical Phase 4 marker and prior foundation/M01 acceptance stay unchanged.

Validation during investigation: initial `cargo test --locked` stopped on the
M01 coding fixture's shell verification assertion; focused rerun passed. A full
rerun concurrent with the initial Tauri compilation failed two workspace-evidence
assertions. These failures are recorded rather than replacing them with historical
201-pass evidence. Root `cargo fmt --check`, strict all-target clippy and
`git diff --check` passed. Final build/test results are recorded below when complete.
No files staged/committed/discarded, no branch switch or worktree.

Final M02-00 foundation validation after the narrow fixture deadline correction:
`cargo test --locked` **201 passed, 0 failed, 1 ignored**; root format, strict
all-target clippy and whitespace checks passed. Production runtime/M01 source
unchanged by this thread. `tests/personal_tasks.rs` preserves existing tests with
a 30-second deadline only for the fresh Rust compilation fixture and a structured
assertion diagnostic. Earlier failed attempts remain documented above/research.

Locked/offline Tauri 2.11.5 material fixture built and launched. Native glass
through WK is visible in the saved screenshot; explicit WK first-responder
restoration enabled text input, Tab traversal and HTML/input/button AX exposure.
This uses Wry private transparency API. Actual Reduce Transparency switching,
VoiceOver, light appearance, resize/reload, distribution packaging and product
acceptance remain unverified. Swift warnings-as-errors compilation passed.
No desktop model calls, mocked host acceptance or live desktop notes task ran.

Repeat fixture validation from repository root:

```sh
cargo build --manifest-path experiments/m02-glass/tauri/Cargo.toml --locked --offline -j 2
cargo fmt --manifest-path experiments/m02-glass/tauri/Cargo.toml --check
cargo clippy --manifest-path experiments/m02-glass/tauri/Cargo.toml --locked --offline --all-targets -j 2 -- -D warnings
```

Next action: resolve the pending ADR 0014 decision; finish outstanding M02-00
material/accessibility checks and make M02-01 Ready only after explicit acceptance.
Tauri/M02 selection is already authorized and requires no repeated permission.

Final fixture checks: locked/offline Tauri build, fixture format and strict
all-target clippy passed (`cargo clippy ... -- -D warnings`, 6m 05s for the first
check dependency graph; no performance claim). Local documentation targets and
experiment text whitespace passed; final `git diff --check` passed.

Solo scope review: fixture has no Astrid runtime dependency, registered app
commands, plugins or capability grants. Private WK transparency, disposable
whole-webview ownership/geometry, real preference switching, older OS execution
and product lifecycle acceptance remain explicit gaps. Native glass, readable
opaque content and bounded keyboard/AX evidence are demonstrated; no runtime
privacy or background guarantee is inferred. ADR 0014 remains Proposed pending
the maintainer's response; stop at this consequential gate. Existing dirty/untracked
user files remain, with only the documented M01 fixture robustness adjustment;
no staging/commit/cleanup/branch switch/worktree.

## M02 implementation started — 2026-10-14

The maintainer instructed implementation to start after the specific ADR 0014
recommendation. Recorded option 1/clauses 1–10 as Accepted in the ADR and AGENTS.
M02-01 is now In progress; material acceptance remains partial. Continuing with
headless Rust host coordination and deterministic authority/lifecycle tests, then
the narrow Tauri adapter and real notes/conversation UI. All prior pending entries
are historical checkpoints. No worktree/branch switch/staging or discarded files.

### Maintainer stop / draft handoff — 2026-10-14

Maintainer requested implementation to stop and a draft PR. Work stopped; M02-03/04
and final visual acceptance remain incomplete. Latest combined-checkout results:
211 Rust passed, 0 failed, 1 ignored; 2 adapter tests; 9 frontend tests; fmt, strict
root/adapter clippy and debug macOS bundle passed. Reduce Transparency restored to
original Off. Native API/opaque transition verified; final reference appearance,
active renderer refresh and live active-run quit still need acceptance. See
[the M02 review](reviews/m02-desktop.md) for exact evidence and the isolated draft's M01 prerequisite.

Draft PR created: [#13](https://github.com/404khai/astrid/pull/13), commit `8baf6af`,
base `feat/m01-personal-tasks`. Implementation remains stopped; no final M02 acceptance.
