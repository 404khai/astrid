# M02 desktop implementation and acceptance — 2026-10-14

Status: Implementation in progress; product acceptance pending final native/window
checks and revised reference-layout inspection. Historical Phase 4 is unchanged.
Baseline `cfb60e8c0adb93a5fc663cf6e0d314d1e6c086c2` plus preserved dirty/untracked
M01 and user files, branch `feat/m02-tauri-desktop`, original checkout, no worktree.
ADR 0014 option 1/clauses 1–10 accepted after the explicit recommendation and
maintainer instruction to start implementation. No M03/service/recovery authority.

## Implemented boundary

`src/host.rs` owns one selected workspace, existing one-writer SessionStore, one
active polled run, cancellation and pending approvals. Tauri is a narrow adapter;
the renderer never chooses execution principals, raw folder paths or arbitrary
policies. Native picker selects a path, Rust validates it, and the existing runtime
owns model/tool execution. CLI remains independently runnable.

Request IDs retain a task digest and receipt, including after terminal completion.
Same-ID/different-content submission rejects; IDs are never silently evicted and
replayed. This foundation limits retained IDs to 32 per app lifetime. One run at a
time, 20 model-call ceiling, Notes defaults read Allow/write Deny/execute Deny.
Coding presets retain existing authority. Shell Ask replies bind to the actual
run/tool/permission ID, are single use and cannot grant truncated commands.

Allowlisted versioned projections exclude credentials/provider continuation and
wholesale Session/RunResult serialization. Text: 2 KiB; history/activity batch:
16 KiB; history: 16 messages/page; snapshot: 64 KiB; activity ring: 256 KiB;
one subscription, one outstanding batch, 10s acknowledgement deadline. Stalled
views detach without stopping event drain. Gaps/truncation remain visible. These
caps do not bound underlying session/provider assembly memory. DOM content is
plain text under bundled-only navigation, restrictive CSP and explicit command
capabilities. One added core capability permits dragging the application window.

Close hides the window; reopen uses the same host. Explicit quit cancels, awaits
terminal cleanup and attempts idle save. Cleanup/save failure prevents a clean
quit claim and remains visible. Host death implies neither rollback nor recovery.
Folder preference stores only a version/path; workflow grants reset to Notes on
restart. `ASTRID_DESKTOP_FOLDER` is a trusted synthetic launch override which does
not overwrite the user's saved selection. Existing user folder preference and
`logo2.png` were preserved.

## Validation observed

Root `cargo test --locked`: **211 passed, 0 failed, 1 ignored**. Ten new deterministic
host tests cover real permission/tool boundaries with mocked inference: duplicate
submission, unauthorized views, forged/stale approval, clipped approval denial,
view detach, provider cancellation, quit during approval, session validity,
one-writer lock, corruption/save failure, private continuation sentinels,
stalled/ordered/bounded projections and UTF-8 boundaries. Existing runtime and CLI
regressions remain green. Root format and strict all-target clippy passed.

Frontend TypeScript typecheck/build and six interaction tests passed before the
reference-layout revision; subsequent results are recorded below. These are
mocked IPC/DOM tests, not live model quality. Adapter local-origin/selection-store
tests and final locked build/strict clippy are being verified. Repeatable commands
are in [desktop README](../../apps/desktop/README.md).

Toolchain inspected: macOS 27.0 build 26A428; Xcode 27.0 build 27A266a; Swift 6.4;
Rust 1.96; Node 24.16/npm 11.13; TypeScript 5.9.3; Tauri Rust 2.11.5, CLI 2.12.2,
Wry 0.55.1; objc2 0.6.4/AppKit 0.3.2. Debug `.app` bundle built successfully,
48.06 MiB; this is not signed/notarized release acceptance or memory measurement.

## Live synthetic notes and independent CLI

Existing OpenAI/Codex subscription auth, `gpt-5.6-sol`, synthetic copied fixture
`tests/fixtures/personal-tasks/outstanding`. No actual user notes were supplied.
Desktop UI showed selected folder/provider and read Allow/write Deny/execute Deny,
submitted the task and produced the actual three-item checklist:

- Buy two warm-white hallway bulbs (`house.txt`).
- Water balcony basil Saturday (`garden.md`).
- Book annual boiler inspection, date not chosen (`garden.md`).

Excluded completed tap-washer replacement; explicitly limited coverage to observed
`.txt`/`.md` sources. Two glob results and both file reads disclosed complete source
coverage in their results. Task-quality rubric passed this one scenario; run
completion alone was not the criterion. No throughput/cost claim is made;
metadata tracing is disabled in this desktop slice.

An independent actual CLI command after idle desktop quit also passed the same
synthetic known-answer rubric: `cargo run --locked -- run --notes --folder <fixture>
--model gpt-5.6-sol --observability off <task>`. It completed with 3 model calls,
3 tool outcomes (listing and both reads), complete in-scope coverage, zero observed
mutations, correct exclusions and uncertainty. Original fixture hashes remained:
`house.txt` e216c8057a03a65427af7d5b2648120359b032f0b6d50a6775fa5c908d45ddcf;
`garden.md` ff73b4244d1ab7b22fd3b61fb1aced9496114f8e5ffb4822575c1cde52ef3523.
One provider/model/scenario is limited live evidence, not general reliability.

## Native material and reference-layout corrections

[Material research](../research/m02-liquid-glass.md) documents public
NSGlassEffectView 26+ and the pinned Tauri/Wry private transparency dependency.
The app actually instantiates NSGlassEffectView and embeds WK as its contentView;
CSS blur and ordinary vibrancy are not claimed as native Liquid Glass.

The first functional UI diverged from the supplied Figma exports: large opaque
header/welcome/composer cards concealed native material. The maintainer rejected
that appearance and requested [RuneIcons](https://www.runeicons.com/icons).
The three exports were viewed again directly. Revision restores the translucent
sidebar, open central logo/welcome composition, title pill, compact pill composer,
conversation bubbles and restrained settings. All labels/content remain real
single-assistant runtime projections; no group/voice/attachment/progress fiction.
Outline RuneIcons SVGs are bundled locally, pinned to upstream commit
2f2a86a3a96cbf5026e4276d349c50a139bd8a88; provenance/license included. No website/editor
code is copied. Native titlebar traffic lights remain actual system controls.

User explicitly allowed a temporary Reduce Transparency toggle with restoration.
Original Off was inspected; On was confirmed; material mode changed to opaque and
Off was restored. Testing exposed blank visible WebKit composition after removing
WK from the native glass container despite retained AX content. The implementation
now keeps containment stable and covers material with an opaque surface in fallback
mode; it does not label that opaque presentation as glass. Visual retest is required.
Native Clear style is selected for the revised transparent composition (public 26+
API), with contrast/increased-contrast and reduced-motion treatments. No fabricated
wallpaper/background screenshot is used.

Earlier material captures included inactive Stage Manager previews and blank
windows; those do not establish visible acceptance. Keyboard AX editing, actual
live notes submission, session switching and saved conversation restoration after
explicit idle quit were observed. A close/reopen attempt during the live run
returned the still-running host; renderer-reload input was interrupted by a user
surface change, so reload continuity remains unverified manually. Host fixtures
cover view-loss/reattachment without granting permissions.

## Failures and remaining limits

Initial offline adapter build lacked registry package `want`; official registry
fetch resolved it. PNG-only bundle configuration failed and was replaced with an
ICNS derived from the supplied logo. Bundled `tauri://localhost` has an empty path;
initial origin code allowed only `/` and blanked the view, fixed with exact origin
matrix tests. Frequent replacement of unchanged DOM controls destabilized focus/AX;
now unchanged text/session controls are preserved and asynchronous snapshot/history
responses are ordered. Temporary material diagnostic compilation failed on ambiguous
AsRef; diagnostic code was removed rather than shipped. Native containment defect
and reference appearance were corrected but require final visual evidence below.

No Windows/Linux build, low-memory measurement, release distribution, macOS 26
hardware run, fresh login-browser roundtrip, VoiceOver session, complete contrast
audit, overlay/topmost capability or active-run recovery is claimed. Application
lifetime background execution ends at explicit quit/process exit. The private
Tauri/Wry transparency API remains a material compatibility/distribution limitation.

## Maintainer stop and draft handoff — 2026-10-14

Maintainer instructed work to stop and requested a draft PR. Implementation stopped;
M02 is not accepted. The updated UI uses the smooth supplied `logo2.png` asset
(copied into the desktop assets without modifying the original), RuneIcons, revised
welcome/composer proportions and safe bounded Markdown presentation. Nine frontend
tests passed; final macOS debug bundle built, 48.11 MiB. Root **211/0/1**, adapter
**2/0/0**, root/adapter fmt and strict all-target clippy passed in the combined dirty
checkout. These are not clean-PR standalone acceptance results.

Live coding Ask exposed the actual command/workspace/authority. Close/reopen retained
run `e4898420-f6d3-4495-934e-f78de5cc33a4` and tool
`c97ee301-d827-401b-bb98-4b1f14e3c91d` while blocked. A later PermissionGranted event
and actual printf output/exit 0 were observed amid user/UI interaction; do not claim
automatic approval or attribute that grant to the agent. Renderer refresh while
active and explicit quit during an actual live active run remain manually unverified.

The Reduce Transparency check passed with stable native containment and a visible,
editable opaque fallback; original Off was restored and verified. Final HUD backdrop
tuning/layout changed after those full-size screenshots. Captured native material
remained flatter than the exports; native API presence is not final visual acceptance.
No further implementation or successor milestone work follows this handoff.

Draft isolation: include M02 source/assets/tests, native probes/evidence and the
small shared read-only policy helper required to compile the host. Preserve other
M01 CLI/prompt/tests/evaluations and pre-existing administrative/user changes in the
checkout. Live notes quality above used the combined M01/M02 dirty source, including
M01's generalized prompt; the draft must wait for that prerequisite before claiming
standalone personal-task acceptance.
