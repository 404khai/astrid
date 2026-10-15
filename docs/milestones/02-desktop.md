# Paused desktop record — 2026-10-15

Status: **Paused**, implementation partial, acceptance Pending. Maintainer direction
supersedes the prior Tauri implementation authorization; no further desktop work.
ADR 0014 is Superseded. Existing Done slice evidence remains valid at its recorded
revision; incomplete material/layout/reload/active-quit acceptance is not closed.
See [pause audit](../reviews/direction-revision-2026-10-15.md) and
[canonical roadmap](../roadmap.md). GPUI investigation belongs to Phase 11.
The dated implementation record below is historical.


# M02 — Tauri desktop foundation

Updated: 2026-10-14
Authorization: Scoped M02 work authorized by maintainer, 2026-10-13; ADR 0014 accepted 2026-10-14
Implementation: In progress (host/projections implemented; macOS UI acceptance ongoing)
Acceptance: Pending
Dependencies: M00 complete; acceptance of ADR 0014 for dependent host implementation
Product acceptance dependency: M01 notes workflow, passed with limitations 2026-10-12
Branch: `feat/m02-tauri-desktop`, existing checkout, no worktree

## Objective and boundary

Deliver a macOS Tauri client that submits real tasks to Astrid's Rust runtime,
shows ordered activity and visible history, handles approvals/cancellation, and
restores idle sessions. Preserve the independent CLI. Meister is the assistant
in this workspace; a floating surface and advanced delegation are later work.

The [existing library](../../src/lib.rs) provides headless execution;
[session storage](../../src/session_store.rs) provides private, locked idle
snapshots. CLI session coordination in [sessions.rs](../../src/sessions.rs) is
client code; extract only the needed reusable headless coordination. No Tauri
types belong in core runtime APIs. The current web marketing site and mobile
demo are not desktop runtime implementations.

[ADR 0014](../adr/0014-tauri-desktop-runtime-boundary.md) proposes embedded
application-lifetime hosting, restricted web IPC and quit semantics. Its contract
must be accepted before these dependent slices become Ready. One selected
directory store and one active run suffice. No sidecar, daemon, remote listener,
multi-agent group system, avatar engine or active-run recovery is included.

## Vertical slices

### M02-00 — Review host decision and macOS Liquid Glass feasibility

Status: In progress (assessment and independent feasibility only)
Dependencies: Scoped M02 authorization, 2026-10-13

Inspect the actual reference images and runtime/M01 implementation. Review ADR
0014 against current requirements and present its precise host/IPC/lifecycle
decision for acceptance without asking again whether Tauri or M02 work is desired.
Dependent host tickets remain Draft until the contract is accepted. Independent
design and native-material experiments may continue without host authority.

Acceptance:

- [x] Map `astrid_first_screen.png`, `astrid_pod_interface.png` and
  `astrid_pod_settings.png` to actual M02 functionality and explicitly deferred UI.
- [x] Verify current Apple/Tauri documentation and SDK/OS availability for native
  Liquid Glass; compare a narrow desktop-adapter integration with other materials
  without calling CSS blur or ordinary vibrancy native Liquid Glass.
- [ ] Render a bounded macOS feasibility view; verify material visibility, text
  contrast, Reduce Transparency/accessibility behavior and webview composition.
  Record OS/SDK/API, screenshots, actual behavior and limitations. No runtime
  execution, screen observation or new agent permissions are needed for this view.
- [ ] Record the recommended native material path and version-aware fallback. If
  authentic Liquid Glass cannot be delivered under the chosen supported SDK/API,
  expose the constraint rather than silently substituting another effect.
- [x] Resolve ADR 0014 explicitly before making M02-01 Ready. No service/background
  recovery or multi-client writer guarantee follows from the experiment.

### M02-01 — Host one task independently of views

Status: Done — deterministic acceptance verified 2026-10-14
Dependencies: M00; ADR 0014 acceptance; implementation authorization

Create a small Rust coordinator and narrow Tauri adapter with host-owned run
futures, cancellation handles, sessions and pending permissions. Do not hold a
global mutex across provider/tool/permission waits. Choose and lock actual Tauri
and frontend dependencies during implementation, following approved host scope.

Acceptance:

- [x] A mocked task submitted from an authorized local view creates one host-owned
  request/run association; duplicate request IDs cannot start another run, including
  when a duplicate carries different task content.
- [x] Unauthorized view commands and renderer-supplied principal/target/policy
  claims cannot confer authority. Rust resolves effective workspace and policy.
- [x] Cancel and permission decisions are routed through runtime interfaces, with
  stale, duplicate and wrong-run approval replies rejected.
- [x] No terminal input is needed. Dropping a view does not drop the polled run.
- [x] A simultaneous CLI writer fails clearly under the existing workspace lock;
  desktop does not create another authoritative session store.

Tests: coordinator fixtures with mocked providers and real permission/event
boundaries; authorized/unauthorized view and duplicate submission cases.

### M02-02 — Project bounded activity and private history

Status: Done — deterministic acceptance verified 2026-10-14
Dependencies: M02-01

Use versioned visible DTOs and authorized stream subscriptions. Set explicit
batch/page byte caps, outstanding-data allowance and reattachment behavior;
document chosen numerical limits before tests. Never serialize Session/RunResult
wholesale, provider credentials or opaque continuation to the webview.

Acceptance:

- [x] Stream preserves run/parent IDs and sequence correlation. Bounded batches
  and acknowledgement prevent a stalled renderer from accumulating unbounded data.
- [x] Host continues draining authoritative events and processing cancellation
  when a subscriber stalls. Projection loss is labeled with coverage gaps.
- [x] Reopened authorized view obtains bounded current state, including pending
  approval; reattachment cannot fabricate a complete historical stream.
- [x] History pagination/tool truncation disclose omissions. Credentials and
  continuation sentinels never appear in IPC, visible exports or web storage.
- [x] Bundled UI renders untrusted file/model/tool content safely; CSP/navigation
  and command capabilities prevent injected content from granting approvals or
  invoking generic shell/filesystem/network access.

Tests: stalled subscriber, ordered projection/gaps, pagination bounds, secret
sentinels, malicious HTML/links and command authorization. Do not promise bounded
whole-runtime memory: original session/provider assembly limits are separate debt.

### M02-03 — Deliver the real conversation workspace

Status: In progress — live notes passed; reference-layout revision/macOS visual checks ongoing
Dependencies: M02-02; M01 for notes-workflow acceptance

Use the supplied Figma exports: `references/astrid_first_screen.png`,
`references/astrid_pod_interface.png` and
`references/astrid_pod_settings.png` for layout and composition. These maintainer reference assets remain local and
are not included in the headless Phase 6/7 PR.
Apply the verified macOS Liquid Glass material through a narrow desktop adapter;
keep presentation/platform types outside the runtime. Expose only real
capabilities: directory/session selection, task submission, streamed text/tool
activity, effective policy, approval and cancellation. Defer group tabs, voice,
attachments and delegated progress until their underlying features exist.

Acceptance:

- [ ] Synthetic notes workflow works end to end with read Allow/write Deny/execute
  Deny, visible folder/provider/policy and actual source coverage.
- [ ] New/switch/restore sessions use existing committed history and rejection
  rules. UI does not pretend incomplete tool batches can continue.
- [ ] Shell Ask in a separately selected authorized coding workflow shows actual
  command/cwd/authority and supports grant/deny; notes policy never widens silently.
- [ ] Keyboard navigation, readable status, reduced motion and accessible controls
  work. Loading/failure/empty states report actual runtime status.
- [ ] Existing Rust provider authentication is reused with explicit login/failure
  presentation; provider login does not grant tool authority or export secrets.

Tests: frontend interaction tests plus mocked host-to-view acceptance. Manual macOS
acceptance records what was actually exercised; a static screenshot is not enough.

### M02-04 — Verify window lifecycle, quit and idle persistence

Status: In progress — host fixtures passed; manual renderer/active quit checks outstanding
Dependencies: M02-03

Acceptance:

- [ ] Hide/close/reopen and renderer reload preserve the same active run while
  the host process lives. Unanswered Ask stays blocked until answered/cancelled.
- [ ] Explicit quit requests cancellation, polls cleanup to a terminal outcome
  and attempts an idle save; cleanup/save failure is surfaced truthfully.
- [ ] Provider/tool/permission cancellation and committed mutations preserve current
  runtime semantics. Host death implies neither rollback nor automatic recovery.
- [ ] Fresh launch restores valid idle history. Corrupt/unsupported stores,
  lock contention and failed saves retain existing explicit failure behavior.
- [ ] CLI tests and one CLI acceptance scenario still work independently of Tauri.

Tests: host lifecycle fixtures, real macOS close/reopen/quit/renderer-loss acceptance,
store failure cases and cancellation regression suite. Process-kill tests must not
expect replay or classify uncertain effects as cleanly cancelled.

## Validation and evidence required for completion

Run locked Rust tests/fmt/clippy and `git diff --check`; record the actual frontend
typecheck/test/build and Tauri/macOS build commands once scaffolded. Establish
repeatable commands in CURRENT.md before marking any desktop ticket Done.
Record revision/worktree, versions, OS, mocked results, privacy/authority failures,
manual app acceptance, M01 known-answer task and remaining limitations separately.
No Windows/Linux support or performance claim follows from a successful Mac build.

## Known risks and later decisions

Tauri IPC is a new trust surface. Plugin/view capabilities do not replace Astrid's
tool policy or constitute OS confinement. Renderer queues, host locking, blocked
approval, explicit quit and one-writer persistence are high-priority failure tests.
Independent service survival, crash recovery, shared CLI/desktop active sessions,
remote trust and mascot window approval authority need their own later contracts.
Original/static avatars suffice; third-party asset/package licensing is not a
desktop prerequisite. Native platform integrations require demonstrated need.

## Completion evidence

M02 work authorized 2026-10-13; new branch created in the existing checkout.
No desktop implementation or Liquid Glass verification is claimed at handoff.
ADR 0014 remains Proposed and dependent host implementation is Not started.

### M02-00 assessment evidence — 2026-10-13

See [native material research](https://github.com/404khai/astrid/blob/8baf6af/docs/research/m02-liquid-glass.md) for source/ADR/M01
review, directly viewed reference mapping, official API sources, installed SDK
evidence, bounded probes and app-window screenshots. Native AppKit glass renders;
WK public background API alone obscures it. Pinned Tauri composition is a separate
fixture with no Astrid host or IPC commands. Accessibility/real preference changes
and product visual acceptance remain incomplete; forced opaque fallback is
simulated. The full feasibility ticket is not Done.

ADR 0014 recommendation presented for explicit acceptance; pending in this thread.
M02-01–04 remain Draft. Baseline `cfb60e8` plus preserved dirty/untracked M01/user
files; no staging, commit, discard, worktree or branch switch. Current Rust test
failures and subsequent validation are recorded in CURRENT/research, separately
from historical M01 acceptance.

Final root regression after a narrowly corrected M01 compilation-fixture deadline:
**201 passed, 0 failed, 1 ignored**; format/strict all-target clippy/whitespace checks
passed. Locked/offline Tauri material build and warnings-as-errors Swift build
passed. Basic Tauri web text entry, visible focus and Tab traversal/AX nodes
verified after restoring WK first responder. Private Wry transparency dependency
is explicit. This is not host, lifecycle, real preference-switching or M02 product
acceptance. ADR acceptance remains pending; M02-00 stays In progress.

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

### Implementation started — 2026-10-14

Maintainer instructed implementation to start in response to the concrete ADR
0014 decision. Its option 1/clauses 1–10 are accepted. M02-01 is In progress;
successor tickets remain dependent on their actual test evidence. No M03 or
independent service is authorized.

Local projection limits selected before implementation/tests: each visible text
field at most 2 KiB UTF-8; history/activity page at most 16 KiB serialized JSON;
recent activity ring at most 256 KiB; one outstanding batch per subscription;
32 retained request IDs (reject new submissions when full, never evict IDs and
accidentally permit duplicate side effects); one subscriber lease, 10-second
acknowledgement deadline; history at most 16 messages/page. Long permission
commands are disclosed as truncated and cannot be approved, only denied or
cancelled. These bounds are projection bounds, not whole-runtime memory limits.

### M02 implementation checkpoint — 2026-10-14

ADR 0014 option 1/clauses 1–10 accepted; real headless coordinator and Tauri adapter
implemented. M02-01/02 deterministic acceptance passed (10 host tests, 2 adapter
tests, privacy/authority and bounded projections). Root suite **211 passed, 0
failed, 1 ignored**. Seven frontend tests passed after the maintainer requested
closer Figma fidelity and RuneIcons. Real desktop notes and independent actual CLI
known-answer scenario passed, with unchanged synthetic input hashes. Current
macOS/reference/material/lifecycle evidence and named remaining checks are in
[the M02 review](../reviews/m02-desktop.md). M02-03/04 remain In progress; this is
not final M02 acceptance. Historical entries above remain dated evidence.

### Maintainer stop / draft handoff — 2026-10-14

Maintainer requested implementation to stop and a draft PR. Work stopped; M02-03/04
and final visual acceptance remain incomplete. Latest combined-checkout results:
211 Rust passed, 0 failed, 1 ignored; 2 adapter tests; 9 frontend tests; fmt, strict
root/adapter clippy and debug macOS bundle passed. Reduce Transparency restored to
original Off. Native API/opaque transition verified; final reference appearance,
active renderer refresh and live active-run quit still need acceptance. See
[the M02 review](../reviews/m02-desktop.md) for exact evidence and the isolated draft's M01 prerequisite.

Draft PR created: [#13](https://github.com/404khai/astrid/pull/13), commit `8baf6af`,
base `feat/m01-personal-tasks`. Implementation remains stopped; no final M02 acceptance.
