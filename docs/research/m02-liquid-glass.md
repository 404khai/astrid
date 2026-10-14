# M02-00: native macOS Liquid Glass feasibility

Date: 2026-10-13
Status: In progress; ADR 0014 acceptance pending; no desktop runtime host implemented
Baseline: `cfb60e8c0adb93a5fc663cf6e0d314d1e6c086c2` plus pre-existing dirty/untracked M01/user files
Branch: `feat/m02-tauri-desktop`; original checkout, no worktree

## Source and contract review

Reviewed CURRENT, canonical roadmap, M02 plan, development workflow, ADRs 0001–0012
and proposed 0014, `src/lib.rs`, runtime Session/RunConfig/run_in_session, client
Sessions coordination, SessionStore, permissions and generalized agent prompt.
Reviewed M01 headless/CLI fixtures and `evaluations/m01-2026-10-12.json`:
three synthetic live notes scenarios, one provider/model, solo rubric review and
separate scripted evidence. These establish an existing workflow to expose, not
any desktop acceptance. No new model authentication, task mode, persistence schema
or permission preset is needed for the first desktop slice.

The runtime already owns completion barriers, sequential tool execution, IDs,
events, cancellation and conversational history. Its lossless event channel can
backpressure execution: the host must drain it independently of a stalled UI.
Raw Session, RunResult and content-bearing events are not safe wholesale IPC DTOs.
Opaque continuation stays private; metadata trace privacy differs from executable
idle snapshot retention. CLI Sessions contains terminal-dependent label cleaning;
extract only needed headless coordination after ADR acceptance. Keep one store
lock for one selected folder and prohibit switching during active work.

## Reference layouts (images directly viewed)

| Export | M02 mapping | Deferred or changed |
| --- | --- | --- |
| `astrid_first_screen.png` | Sidebar: selected folder, new session, real saved session list; Meister welcome and task composer; settings; visible provider and effective policy | No fabricated populated sessions, recent durations or background “Working” cards. Astrix identity/pod selector, voice, attachments and commands without handlers omitted. No wallpaper asset is extracted or assumed licensed. |
| `astrid_pod_interface.png` | Single Meister conversation: committed user/assistant messages, provisional stream labeled until committed; ordered tool activity, real approval/cancel; coverage and failure notices | No group members, participants, typing avatars, image attachment or delegated progress. Reference bubbles/spacing and split composition remain useful. Normal completion says “Run completed”, not verified task success. |
| `astrid_pod_settings.png` | Settings panel composition for actual folder/provider/auth status, effective next-run policy and supported observability controls | Pod membership/name/about/delete is outside M02. Provider sign-in never grants tool authority. Permission changes occur between runs and do not persist grants. |

The translucent reference is a visual direction, not proof of a native material.
Keep conversation/tool text on readable surfaces; use authentic native glass for
navigation/control hierarchy rather than making all conversation content glass.

## Official API and integration evidence

- [Apple NSGlassEffectView](https://developer.apple.com/documentation/appkit/nsglasseffectview)
  and [Build an AppKit app with the new design](https://developer.apple.com/videos/play/wwdc2025/310/):
  custom glass uses `contentView`, not a sibling material behind content. Apple
  recommends controls/navigation at the upper functional layer; grouping nearby
  glass through NSGlassEffectContainerView shares sampling and rendering.
- Installed `AppKit.framework/Headers/NSGlassEffectView.h`: NSGlassEffectView,
  regular/clear styles and container available macOS **26.0**; `effectIsInteractive`
  additionally requires **27.0**. The probe intentionally does not require 27's API.
- [Tauri window customization](https://v2.tauri.app/learn/window-customization/)
  documents narrow Rust/AppKit access via `ns_window`.
  [with_webview](https://docs.rs/tauri/2.11.5/tauri/webview/struct.Webview.html#method.with_webview)
  supplies platform handles on the main thread. Pin dependencies for this adapter;
  do not let platform types enter Astrid core.
- [Tauri configuration](https://v2.tauri.app/reference/config/#macosprivateapi)
  describes the transparent background API/private API setting; current docs also
  describe changed behavior in 2.12.1+. That is not the pinned experiment version.
  Installed Wry source explicitly marks `drawsBackground` KVC as **private API**.
  `NSGlassEffectView` itself is public; transparent WK composition has a separate
  compatibility/distribution risk. A passing probe is not an App Store promise.

Environment measured: macOS **27.0 (26A428)**; Xcode **27.0 (27A266a)**;
Swift **6.4**, arm64 macOS target; Rust **1.96.0**; Node **24.16.0**;
npm **11.13.0**. SDK selected by `xcrun --show-sdk-path`:
`/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk`.
Older OS execution and Windows/Linux are untested.

## Independent bounded probes

Source: [GlassProbe.swift](../../experiments/m02-glass/GlassProbe.swift).
No runtime, provider, IPC, network navigation, screen reading or device capability.
AppKit/WebKit are platform/material fixtures only, not a Swift primary client.
The probe exits after 180 seconds or its window closes.

Three surfaces share an application-drawn color backdrop: real NSGlassEffectView
with native text/button content, real NSGlassEffectView with WKWebView content,
and NSVisualEffectView ordinary vibrancy. Transparent HTML and the public
`underPageBackgroundColor = .clear` do **not** remove WK's opaque backing here.
Native glass visibly renders; the web rectangle obscures it. CSS blur is absent.

Screenshots are of the probe's own app window through the UI connector:
[public-WK composition](../../experiments/m02-glass/native-public-webview.png),
[forced opaque fallback](../../experiments/m02-glass/opaque-fallback.png).
The initial fallback capture coincided with window animation/exit and was replaced
by a stable capture. UI connector first timed out; packaging the executable as a
local app enabled window/AX inspection. Native labels/button and named HTML input
are exposed in the accessibility tree. The normal-mode web input accepted text;
full keyboard traversal, VoiceOver and input behavior across material changes are
not yet accepted. Do not treat AX presence alone as accessible interaction proof.

Forced `--opaque-fallback` exercises app-owned opaque backing without changing
system preferences. It is **simulated** preference selection, not evidence of
real Reduce Transparency switching. Probe checks the actual preference at startup;
Observed startup settings on this Mac: Reduce Transparency=false, Increase
Contrast=false, Reduce Motion=false (logged by the native probe). Production
adapter must observe preference changes and update material/contrast.
Reduce Motion is not animation-tested: these fixtures contain no app animation.
The ordinary-vibrancy comparison deliberately stays visible in forced mode;
it is not the recommended production accessibility fallback.

Point samples: [contrast data](../../experiments/m02-glass/contrast-samples.json),
reproducible with [ContrastSamples.swift](../../experiments/m02-glass/ContrastSamples.swift).
These compare hypothetical white foreground with sampled screenshot background;
not complete WCAG or glyph-antialiasing verification. Bright backdrop samples
reinforce the need for opaque text/content surfaces rather than unconditional
white web text over arbitrary glass. Product contrast acceptance remains pending.

A separate [Tauri material fixture](../../experiments/m02-glass/tauri/Cargo.toml)
pins Tauri **2.11.5**, tauri-build **2.6.3**, objc2 **0.6.4**, and
objc2-app-kit/foundation **0.3.2**. Lockfile also records Wry **0.55.1**,
Tao **0.35.3**, tauri-runtime-wry **2.11.4**, tauri-utils **2.9.3**. No Astrid dependency, registered app commands,
plugins or capability grants; bundled static HTML/CSS and restrictive CSP. Its
whole-webview glass wrapper tests composition only, not the recommended final
control-only layout or lifecycle host contract. Build/visual evidence below is
recorded when complete. Version-aware opaque fallback does not claim Liquid Glass.

## Recommended material path and limits

Use public NSGlassEffectView (26+) through a narrow main-thread macOS adapter,
embedding control content through contentView; opaque readable transcript/tool
surfaces and an explicitly labeled opaque fallback on unavailable OS/reduced
transparency. Treat the current whole-webview wrapper as disposable feasibility
code. Do not ship its geometry/lifetime management as the desktop host.

Before M02 visual acceptance: prove the pinned Tauri transparency composition,
resize/reload and focus stability; inspect real accessibility preference changes,
light/dark and increased contrast; measure representative text contrast. If the
supported integration cannot satisfy genuine glass, report the constraint. Do not
replace it with CSS blur or ordinary vibrancy and call the requirement complete.

## Host decision presented

Recommend ADR 0014 option 1: embedded Rust coordinator, one active run and selected
locked store, Rust-owned authority and private continuation, versioned bounded
visible IPC, authorized bundled views only. Window close/hide and renderer loss
retain the host run; explicit quit cancels, polls cleanup and attempts an idle save.
Forced death makes no rollback/recovery promise. Sidecar and independent service
are alternatives with transport/supervision/trust costs outside this first slice.

Explicit acceptance requested in this thread on 2026-10-13; **pending**.
M02-01 through M02-04 remain Draft. Material experiments do not establish host,
background survival, remote trust, new tool grants or mascot approval authority.

## Tauri visual findings and validation follow-up

The locked/offline Tauri fixture built and launched on macOS. Actual log:
`wk_first_responder=true`; `material=NSGlassEffectView style=regular
content=WKWebView tauri=2.11.5 transparency=wry-private-api`.
[Stable composition/focus screenshot](../../experiments/m02-glass/tauri-composition.png)
shows application-drawn colors through native glass and opaque text surfaces.
It is real NSGlassEffectView, not CSS blur or the NSVisualEffectView comparison.

Initial capture was a reduced/transformed window preview and AX exposed only
window chrome. Explicit probe-window activation produced a stable capture.
Restoring WK as the NSWindow first responder after reparenting enabled text entry
and exposed HTML/input/button AX nodes; Tab moved the visible focus ring from
input to button. This is bounded keyboard/AX evidence, not VoiceOver acceptance.
The initial build required an objc2 MainThreadOnly import and an explicit icon
(the existing user-supplied logo); both failures corrected. No distribution bundle,
signing/notarization or installed production app is claimed.

Root validation uncovered a pre-existing M01 fixture deadline problem: nested
`cargo test --offline` compiles a fresh fixture under a two-second shell limit.
Full parallel and serial runs failed the null-exit-code assertion, while a focused
run passed. Raised **only that compilation fixture** to 30 seconds and added its
structured result to the assertion diagnostic in `tests/personal_tasks.rs`.
Other fixture deadlines, production runtime limits and cancellation semantics
remain unchanged. The entire existing untracked file and M01 implementation are
preserved with this narrow additive test robustness change; no staging/commit.
The failures preceding that change remain part of this record.

Repeatable commands (from repository root):

```sh
xcrun swiftc -warnings-as-errors -parse-as-library experiments/m02-glass/GlassProbe.swift -o /tmp/astrid-glass-probe -framework AppKit -framework WebKit
xcrun swift experiments/m02-glass/ContrastSamples.swift experiments/m02-glass/native-public-webview.png
cargo build --manifest-path experiments/m02-glass/tauri/Cargo.toml --locked --offline -j 2
cargo fmt --manifest-path experiments/m02-glass/tauri/Cargo.toml --check
cargo clippy --manifest-path experiments/m02-glass/tauri/Cargo.toml --locked --offline --all-targets -j 2 -- -D warnings
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

The fixture bundles static HTML/CSS directly with Tauri; Node/npm versions are
environment inventory only. No npm dependencies, frontend TypeScript/typecheck,
interaction test suite or product UI build exists yet. No mocked host tests or
live desktop model/notes run took place. These wait for accepted ADR 0014 and
dependent host implementation.

Final root results after fixture correction: `cargo test --locked` **201 passed,
0 failed, 1 ignored**; `cargo fmt --check`,
`cargo clippy --locked --all-targets -- -D warnings`, and `git diff --check` passed.
Swift probe warnings-as-errors build passed. Locked/offline Tauri build passed
(final rebuild 26.88 seconds; not a performance baseline). Root test totals are
foundation/M01 regression evidence, not acceptance of an unimplemented desktop
host. No live provider or paid infrastructure used by this investigation.

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

## Application composition correction — 2026-10-14

The first real application UI placed opaque welcome/header/composer cards over
native glass. The maintainer correctly rejected the resulting appearance. Those
cards were removed; reference proportions, translucent sidebar, open welcome,
compact composer and genuine locally bundled RuneIcons now form the presentation.
No reference screenshot is used as a fake backdrop or simulated app content.

Native NSGlassEffectView uses its public Clear style (26+ SDK API). To supply a
window backdrop, the adapter adds public NSVisualEffectView with behindWindow
blending and HUDWindow material below the glass container. This backdrop is
ordinary vibrancy; the NSGlassEffectView which actually embeds WK remains the
native Liquid Glass implementation. The base alone is not reported as glass.
[Apple's behindWindow documentation](https://developer.apple.com/documentation/appkit/nsvisualeffectview/blendingmode-swift.enum/behindwindow)
and installed SDK confirm that blending API. No 27-only interactive effect or new
screen-reading/control privilege is used. Window titlebar overlay/traffic lights
are actual Tauri/macOS controls, not painted replicas.

With explicitly authorized temporary Reduce Transparency, original Off was
confirmed, On switched the application to an opaque surface, and Off was restored.
Removing WK from glass had produced blank composition with intact AX content.
Keeping containment stable and occluding material with a solid fallback fixed that
failure: full-size opaque screenshot retained welcome, controls and edited unsent
text; restoration retained the UI and returned native mode. These screenshots are
[opaque](../../experiments/m02-glass/desktop-opaque.png) and
[native](../../experiments/m02-glass/desktop-native.png).
The captures were taken before final HUD backdrop tuning. Native capture still
looked flatter than the Figma export; it does not prove wallpaper fidelity or a
complete contrast audit. Inactive Stage Manager previews are excluded from visual
acceptance. Unsupported-OS behavior is class availability plus opaque fallback,
not an actual macOS 25/26 hardware acceptance run.
