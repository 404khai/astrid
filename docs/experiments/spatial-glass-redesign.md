# DS-00 follow-up — reference-led spatial spaces and native glass

Authorized by the maintainer's 2026-10-22 redesign request, on the existing
`experiment/gpui-spatial-workspace` branch. Mock-only scope and runtime exclusions
remain unchanged. Preserve unrelated checkout changes.

Adopt `references/desktop/spatial_workspace_tabs.png`: edge-to-edge illustrated
night scene, scattered tall black spaces, compact identity/avatar headers,
one richer artifact-focused space, quiet corner controls, small floating activity
pill, and a bottom composer floating over the canvas. Keep session names/status
legible at overview; show conversation detail on focus. Retain Hugeicons.

Inspected the requested Dia X tab at
https://x.com/imjudekim/status/2113107691430371350, including the native toolbar video
and AppKit hosting discussion. This is an implementation example, not performance
evidence for Astrid. Official Apple SDK headers confirm NSGlassEffectView (macOS
26+) with contentView, cornerRadius, tintColor and regular/clear styles; interactive
glass is macOS 27+. This machine reports macOS 27.0. Guard runtime availability.

Keep GPUI 0.2.2 and the custom spatial canvas. Add an app-local NSView host whose
bounds are driven by GPUI prepaint/layout and whose lifetime is held by the root
view. Host actual NSGlassEffectView controls and a native editable composer.
Route native actions through bounded asynchronous messages, restore GPUI first
responder for palettes, and hide native islands while GPUI overlays are open.
Use NSVisualEffectView fallback on older systems, labeled as fallback in evidence.
No global monitors or new OS permissions. No claim of arbitrary interleaving
between every GPUI-painted element and every native NSView.

NativeDesktop (https://native.formalsnake.dev/) describes a broader retained
cross-platform toolkit with an AppKit backend. Study it, but do not migrate this
experiment or mix its dependency graph into official pinned GPUI.

Use offline Blobatar 2.7.0 (MIT) deterministic fixtures for the five identities,
generated from public mock names. No avatar service requests at app runtime.
Generate a repository-owned night-scene SVG; do not extract artwork from the
reference screenshot or ship external copyrighted wallpaper.

Plan: native glass host → quieter layout/cards and local avatars/background →
compile/invariant checks → native visual/action checks → updated checkpoint.

## Checkpoint — 2026-10-22

Implementation complete for this follow-up; full DS-00 acceptance remains bounded
by the verification limitations below. No Host integration or roadmap transition.

Changed `main.rs` presentation and default `model.rs` placements. Added `art.rs`,
`native_chrome.rs`, local Blobatar SVGs/license/provenance, original night-street
SVG, and Hugeicons Add/Computer/Sidebar/Settings assets. Pinned direct
`cocoa-foundation = 0.2.0` and `resvg = 0.45.1` (default features off), both already
in GPUI's dependency graph. Root crate and existing unrelated dirty files untouched.

The hosted surfaces are real NSGlassEffectView instances on supported macOS,
with native NSButtons, NSSegmentedControl and NSTextFields. GPUI prepaint supplies
logical-point frames, including flipped-coordinate conversion; Rc ownership keeps
the host on the main thread and Drop detaches/releases views and callback targets.
Native actions use a bounded channel. Query notifications read the latest field
value when consumed, avoiding a stale copied query during typing bursts. Native
palette delegates handle arrow navigation, Return and Escape; ordinary text editing
and IME stay with AppKit. When a GPUI modal opens, the other native islands detach
from the parent; only the palette's own editable field remains hosted. Closing
restores the islands and GPUI first responder. Reduced motion also disables the
macOS 27 interactive-glass opt-in. Background blur is provided by AppKit, with no
screen capture permission or image acquisition.

NativeDesktop was inspected, not adopted: its full backend architecture would be
a larger framework change. This experiment retains official pinned GPUI and only
the small macOS hosting adapter. Native material style/appearance and availability
are OS-controlled; no Windows/Linux/wasm portability claim for this adapter.

Visual checks: launched the real application and compared against the supplied
reference. Confirmed staggered black rounded spaces, local distinct blob avatars,
a mock reader preview, quieter floating controls and composer. Fixed an observed
GPUI 0.2.2 `Arc<Image>` SVG color-channel issue: unlike its PNG decoder, the SVG
branch omits conversion from RGBA to BGRA. Artwork now rasterizes once to PNG via
resvg and is cached; the fresh preview shows the intended navy and identity colors.
No dependency patch, scraped wallpaper or externally loaded avatar is used.

Native interaction evidence: the fresh preview's AX tree exposes the native
segmented control, composer, send/search/settings buttons, and palette field.
Clicking Search removed the other native controls from the tree and focused the
palette field; typing “Scholarship” visibly changed the native field value; Return
restored the canvas and native controls with the focus segment selected. A composer
submission was attempted, but subsequent ScreenCaptureKit -3812 failures prevented
final visual confirmation, so it is not recorded as accepted. Repeated Stage
Manager thumbnail captures also limited pixel-level visual review. A separate
`Astrid Spatial Glass` preview was used to preserve the earlier open mock window.

Validation: 18 deterministic tests pass, including native-coordinate/resize math,
keyboard delegate messages, and SVG-to-PNG asset validation in addition to the
existing camera, gesture, layout, search and bounded magnification checks. Clippy
with `-D warnings`, rustfmt and a native build pass. Upstream `block 0.1.6` still
emits its existing future-incompatibility notice. No live model/runtime/network
operation was tested or introduced.

Motion review (Emil principles applied in GPUI/AppKit):

| Before | After | Why |
| --- | --- | --- |
| App-drawn composer with limited editing | Native field hosted in glass | Platform editing and focus behavior; no decorative input animation |
| Native islands could appear above a GPUI palette | Detach other islands; host only the palette field | Explicit compositing and hit-testing ownership |
| Repeated rectangular face treatment | Cached static Blobatar silhouettes | Recognizable identities without perpetual animation/redraw |
| Dotted plane and dense toolbar | Cached scene plus small native control islands | Match the reference's spatial hierarchy |

No new decorative animation was added. Keyboard and palette transitions remain
immediate; existing pointer camera travel stays interruptible and reduced-motion
aware. New native glass appearance/performance is not covered by the old 19.442 ms
frame-interval measurement: no updated frame-pacing or GPU result is claimed.

Next incomplete verification: composer submission/IME across native-to-GPUI focus,
physical trackpad pinch, native islands at 760×560 and on display-scale changes,
reduced transparency/motion, and a frame-pacing run with the glass redesign. Custom
GPUI session cards/results still lack a production screen-reader contract. The
scene is 2D, with rounded/stacked spaces; there is no true perspective projection.
