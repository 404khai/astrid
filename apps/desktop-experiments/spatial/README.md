# Astrid Spatial · DS-00

Standalone, macOS-only GPUI experiment. All sessions and activity are mock fixtures.
No dependency on Astrid's runtime; no Host, model, network data source, background
jobs, or OS window capture. Layout and notes live only in memory.

```sh
./apps/desktop-experiments/spatial/run-macos.sh
```

Requires Rust (tested 1.96), full Xcode + macOS SDK and Metal toolchain. The exact
GPUI release and transitive dependencies are pinned in Cargo.toml/Cargo.lock.
Builds independently of the root crate. No Windows/Linux support is claimed.

- Opens on bot home. Swipe horizontally or use Left/Right to switch the five mock bots.
- Native sidebar button or `⌘B`: collapse/restore navigation. Bottom sidebar avatars select bots.
- Native view control: folders / bot home. Browse spaces also opens folders; Spatial canvas opens movable cards.
- Folder view: hover a tab to lift its folder; click to open its session. Left/Right selects, Return opens.
- Drag blank canvas / two-finger scroll: pan. Drag a card header: move that card.
- Pinch inward: see more; spread outward: zoom in. Native trackpad path still
  needs physical-device acceptance. ⌘ or Ctrl + scroll: pointer-centered zoom. `+` / `−`: center zoom.
- Click a space or Focus: bring forward and frame it. Swipe horizontally or use
  Left/Right in focus to browse sessions. Return to Overview to move card headers.
- `Tab` / `Shift-Tab`: cycle sessions; `Enter`: expand/collapse; `P`: pin/unpin.
- `0`: fit all; `1`: bot home; arrow keys in the canvas: pan.
- `⌘K`: session search; `⌘⇧K` or `⌘P`: commands; arrows / Enter to choose.
- `Escape`: close overlay, stop motion/drag, or leave composer.
- `⌘R`: reset layout; `⌘Z`: recover previous layout. Pinning does not block an
  explicit layout reset, and Undo restores pins and positions together.
- `M` or the commands palette: reduced motion. Initial setting reads macOS reduceMotion.
- Composer: type a local mock note and Enter. No message is sent.
- Click artifact row: inspect static fixture. All artifacts are illustrative.
- Palette includes empty and overlapping/long-title fixtures. Undo recovers layout.

Run deterministic invariant checks:

```sh
cargo test --locked --manifest-path apps/desktop-experiments/spatial/Cargo.toml
cargo clippy --locked --manifest-path apps/desktop-experiments/spatial/Cargo.toml -- -D warnings
```

`--benchmark` requests exactly 240 render frames with a gently moving five-card
canvas, reports frame callback intervals and render-tree construction time, then
returns to event-driven idle. These are not Metal GPU timings or presentation
latency. Pass it after `cargo run … -- --benchmark`.

See [design, upstream investigation, and checkpoint](../../../docs/experiments/spatial-desktop.md).

Native glass uses macOS 26+ NSGlassEffectView, with NSVisualEffectView fallback
on older systems. AppKit owns composer/palette editing; GPUI owns the canvas.
Local Blobatar 2.7.0 identities and the original night scene are bundled offline.

Current limitations: 2D rounded surfaces (no perspective curvature), unverified
full IME/focus acceptance, custom-card screen-reader support and pending manual
interaction/resize QA. The glass redesign needs a fresh performance measurement.
See the [current checkpoint](../../../docs/experiments/spatial-glass-redesign.md).
Close the previous prototype window before relaunching a rebuilt bundle.

See the [home and folder checkpoint](../../../docs/experiments/spatial-home-folders.md) for this follow-up and remaining device QA.
