# DS-00 — mock spatial desktop

## Contract and design specification

The maintainer's DS-00 request authorizes this standalone GPUI experiment only,
superseding earlier desktop exclusions within this scope. Existing runtime, CLI,
CU integration and paused M02 sources stay untouched. No Host, credentials,
network data source, model, scheduler or operating-system window capture.
Branch: `experiment/gpui-spatial-workspace`, same checkout, from `62e00f4b`.
Unrelated dirty and untracked data is preserved.

### Reference study

| Reference | Adopt | Interpretation / boundary |
| --- | --- | --- |
| `desktop/astrid_floating_spatial_workspace.png` | Deep navy canvas, near-black rounded sessions, mint/blue mark, bright face avatars, understated activity pills, floating bottom composer | Primary identity; Meister is primary, Astrix are specialists. Always-visible mock label. |
| `desktop/phantomat.png` | Pointer-centered navigation, selected outline, keyboard switcher, fit-all and minimap | Virtual session surfaces, no captured OS windows or borrowed orange identity. |
| `desktop/rounded_canvas.png` | Broad spatial overview, breathing room, quiet dotted field, persistent orientation | Flat 2D world for this prototype. Perspective curvature would require custom textured drawing and distort text. |
| `desktop/clonk.png` | Independent task surfaces with concise activity and artifact context | Dark Astrid surfaces rather than its translucent pastel treatment. |
| `astrid_first_screen.png` | Personal-assistant invitation, floating composer, blue/mint logo, warm scene depth | No permanent sidebar. A quiet procedural night field avoids extracting wallpaper from a screenshot. |
| `astrid_pod_interface.png`, `astrid_pod_settings.png` | Color-coded face identities, message hierarchy, artifact previews and relationship to specialists | Fixture content only; no pod spawning or settings that imply authority. |
| `logo.png`, `color_font.jpeg` | Actual supplied logo and clear sans-serif typography hierarchy | System sans-serif for native readability; no unlicensed font assumption. |

Canvas (revised per maintainer feedback): full-bleed rounded viewport with floating
composer and navigation, dark navy field, generous spacing; 5 cards in world
coordinates. A compact top rail gives search, overview, reset and motion settings.
Bottom composer explicitly submits local mock notes only. Minimap stays in screen
space. Cards use summary, compact and detailed levels; never illegible transcripts
at overview zoom. Header dragging is distinct from blank-canvas panning. Pinned
cards resist dragging, and focusing never changes stored placement. Meister may
suggest relevance through badges or a switcher ordering in a future integration;
never silently rearrange pinned or manually placed work.

Motion: direct pointer tracking, no keyboard or palette animation. Occasional
pointer focus/overview gets a short interruptible camera transition; reduced
motion removes camera travel. No perpetual pulsing or simulated live updates.
Apply Emil's `emil-design-eng`, `animate`, `review-animations` from upstream commit
`2d228630ba1446365f614a108cb8252f1feda202`, translating principles rather than CSS APIs.

## Small sequential milestone plan

1. **DS-00A:** independently buildable shell, branding, viewport, mock composer,
   shortcuts. Verify native dependency and initialization API with a real build.
2. **DS-00B:** tested camera math, exclusive gestures, virtual drag/focus/selection,
   bounded zoom, reset/fit and minimap.
3. **DS-00C:** deterministic source and five purposeful fixtures, semantic zoom,
   compact/expanded content and artifacts.
4. **DS-00D:** keyboard search/palette, focus navigation, pinning, undoable reset.
   Mock layout is memory-only; optional disk persistence deliberately omitted.
5. **DS-00E:** interruptible camera movement, reduced motion, visual and redraw audit.
6. **DS-00F:** invariant tests, native launch and scoped visual/interaction QA,
   performance evidence and honest limitations. No successor Host integration.

## Framework investigation

Official sources inspected: https://gpui.rs/, https://github.com/zed-industries/zed,
https://github.com/zed-industries/awesome-gpui,
https://github.com/longbridge/gpui-kit, https://github.com/emilkowalski/skills.

Use exact published official `gpui = 0.2.2`, its source package and committed
Cargo.lock. Its VCS metadata identifies Zed `69e2130295c2649963eb639fc70b4f2ee8ea1624`
(published metadata records dirty=true; the crates.io checksum is the reproducible
artifact). Current Zed HEAD observed `544780422affd11699fb531c49af5cc6e82a3fd7`
uses a split `gpui_platform::application()` API; do not mix that example with the
published release, whose bundled example uses `Application::new().run`,
`cx.open_window`, `cx.new`, `Render`, `div` and `canvas`.

Target macOS only, default Linux/Windows features disabled and `font-kit` explicitly
enabled. Without `font-kit`, macOS selects GPUI's NoopTextSystem: text lays out but
does not render. This was caught by native visual QA and corrected. Xcode is installed at
`/Applications/Xcode.app/Contents/Developer`; `xcrun --find metal` succeeds.
Rust 1.96.0 observed. Metal compiler and macOS SDK are required, not just Rust.
GPUI Kit main declares exact `gpui-pre = 0.3.9` snapshots, incompatible with this
official published release. It was examined and omitted; bespoke canvas needs no
component dependency. Canvas transforms, dotted field, minimap and interaction
ownership need application code. Platform parity and production accessibility
are not claimed.

## Final checkpoint — 2026-10-22

Stopped at the maintainer's “Round up pls.” request. DS-00A–E prototype code is
present; DS-00F automated checks and native launch/visual inspection are complete
only to the extent recorded below. Full interaction acceptance remains pending.
No production readiness or complete definition-of-done claim.

Files added: standalone `apps/desktop-experiments/spatial/` manifest/lock, launcher,
README, supplied logo copy, MIT Hugeicons subset/license, `src/main.rs` presentation,
`model.rs` geometry/layout, `fixtures.rs` replaceable mock source, `icons.rs` assets,
`magnify.rs` app-local Cocoa responder, and this specification/checkpoint.
No core/runtime files or pre-existing dirty files were changed by DS-00.

Implemented: world-coordinate overview, exclusive header drag/background pan,
bounded pointer zoom, focus/z-order, pinning, fit/home, minimap, five deterministic
sessions, semantic summary/detail, compact/expanded cards, local mock notes,
artifact previews, search/palette, keyboard navigation, memory-only layout undo,
empty/overlap/long-title fixtures, reduced motion and a 220 ms interruptible camera
transition using cubic-bezier(0.23, 1, 0.32, 1). Key/palette/expansion changes are
immediate. Hugeicons Stroke Rounded assets are exported from exact
`@hugeicons/core-free-icons@4.3.5` (MIT); retained license accompanies the SVG subset.
No JS/framework runtime was added.

Maintainer feedback incorporated: full-bleed canvas with larger rounded corners,
composer floating above it, click-to-focus, and a horizontal focused presentation
that leaves manual overview positions intact. Horizontal scrolling or arrow keys
navigates focused sessions; overview remains the place to reposition cards.
GPUI's public scroll events do not expose magnification. An instance-local subclass
of this app's GPUI NSView implements `magnifyWithEvent:`, delivers finite deltas
through a bounded channel, and wakes GPUI without polling/re-entering App state.
Pinch inward reduces zoom; spread outward increases it. No global event tap,
input injection, screen capture, cross-app capability, or new permission semantics.

Validation: `cargo test --locked --manifest-path apps/desktop-experiments/spatial/Cargo.toml`
passes 15 tests (coordinates, pointer zoom, bounds/NaNs, drag/pin/isolation, focus,
search/long names, reset/undo, fit/empty state, keyboard selection, easing,
magnification direction and bounded native callback delivery). `cargo clippy
--locked --manifest-path apps/desktop-experiments/spatial/Cargo.toml -- -D warnings`
and rustfmt are the final checks. Upstream `block 0.1.6` emits Cargo's future
incompatibility notice; no dependency patch was made.

Native evidence: real Metal-backed application launched; visual inspection caught
and resolved missing glyphs, confirmed Astrid branding, Hugeicons, near-black
rounded cards, full canvas, floating composer and minimap. The rounded surfaces
follow the supplied Astrid identity; wallpaper/artwork was not extracted from
screenshots. Later automation was rejected because the user was interacting with
the app; no attempt was made to override that activity. CUA also intermittently
captured the Stage Manager thumbnail rather than the main surface. This prevents
an honest claim that every pointer/keyboard, resize, empty, dense or reduced-motion
scenario was visually accepted end-to-end.

Performance: a bounded 240-frame debug run with native fonts and five moving
virtual cards measured frame callback intervals **19.442 ms median / 31.814 ms p95**
(max 571.661 ms, including startup). Render-tree construction measured
**338.084 µs median / 578.750 µs p95**. This is CPU callback pacing, not Metal GPU
timing or display presentation latency; it does not establish stable 60 fps.
An earlier NoopTextSystem run is not representative and is excluded. Eliminated
duplicate frame notifications, bounded dotted-grid density, and stopped requesting
frames after transitions/benchmark completion. No perpetual mock updates.

Next incomplete task: physical-trackpad acceptance of native pinch, then a clean
manual QA pass for search/keyboard routing, horizontal navigation, dragging/pins,
resize to 760×560, empty/overlap/long titles, reset recovery and reduced motion.
The native callback test does not establish physical trackpad delivery. GPUI 0.2.2
exposes only the native window in the observed AX tree; custom control screen-reader
support is unverified. Mock editing currently supports append/backspace/paste,
not a production IME/text-selection implementation. The canvas is 2D with rounded
surfaces; true curved/perspective projection is **not implemented**. Motion currently
rebuilds GPUI layout rather than using a general compositor transform, so further
frame profiling is needed before judging Dia-level craftsmanship.

Before real Host integration: a separately authorized boundary design is required
for runtime snapshots/events, authority and cancellation UX, persistence/provenance,
secure transport and authentication. Replace `SessionSource` with a scoped adapter
only after that decision; do not treat this visual state as authoritative runtime
state or invent a universal protocol from fixtures. Production accessibility/text
input and consistent measured frame pacing also need work. No Host work started.
