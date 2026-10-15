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
