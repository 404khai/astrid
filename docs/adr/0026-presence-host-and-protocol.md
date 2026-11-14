# ADR 0026 — Native macOS presence host and presence protocol v1

Date: 2026-11-11
Status: Accepted — 2026-11-11 for the presence host only (v2 gameplan D1 option a)
Related: [v2 gameplan](../plans/v2-gameplan.md) D1 and V2-01; ADRs 0013/0014
(superseded desktop directions), 0022–0025 (observation/action grants, unchanged).

## Context

The v2 gameplan makes a system-wide overlay Astrid's primary surface: Astrid's
own cursor, ink and subtitles drawn above the user's untouched apps. D1 asked
which technology hosts it. The maintainer instructed "let's start with V2-01 ...
the overlay of astrid's logo, plus cursor from the components", which is the
go-ahead to build the recommended option (a) for the presence host.

Overlays need exact AppKit window behaviour: non-activating, click-through
panels at a fixed level, joining every Space and showing beside full-screen
apps. Tauri (paused M02, ADR 0014 superseded) and GPUI (pending Phase 11) would
each reach those through extra layers. The Rust runtime must stay authoritative
for runs, grants and limits, so the host must be a thin client.

## Decision

1. **Host.** A native Swift/AppKit presence host lives in `apps/presence-mac/`
   as a Swift package (macOS 14+, `swift build`, no Xcode project). It runs
   as an accessory app (menu-bar item, no Dock icon) and owns presentation only:
   one transparent, borderless, click-through, non-activating `NSPanel` per
   display at `.screenSaver` level with `canJoinAllSpaces`, `stationary`,
   `fullScreenAuxiliary` and `ignoresCycle`, drawn with Core Animation.
   AppKit stays inside this host.
2. **No authority in V2-01.** The host has no model, no screen capture, no
   input injection and no network listener. Following the user's pointer for
   the logo's eyes uses `NSEvent.mouseLocation`, which needs no Screen
   Recording or Accessibility permission. The overlay never reads what is
   beneath it.
3. **Presence protocol v1.** Runtime → host events, one JSON object per line
   (UTF-8, `\n`-terminated). Every event carries `"v": 1` and `"type"`.
   Unknown fields are ignored so v1 can grow additively. Unknown types, wrong
   versions, wrong field types and out-of-range values are rejected per line.

   | type | fields | meaning |
   |---|---|---|
   | `show` | optional `x`, `y`, `display` | fade in, optionally jumping first |
   | `hide` | | fade out |
   | `point_at` | `x`, `y`, optional `display`, optional `cursor` | glide the cursor hotspot to the point, drawn in `cursor`'s style |
   | `say` | `text`, optional `final` (default true), `duration_ms` | subtitle; empty text clears; non-final partials never auto-clear |
   | `sketch` | `shapes` (1–32), optional `display` | ink: `circle {x,y,r}`, `arrow {from:[x,y],to:[x,y]}`, `line {from,to}` |
   | `clear_ink` | | remove all ink |
   | `wait` | `ms` | script pacing: hold later events |
   | `quit` | | exit the host |
   | `wave` | | play the Distortion Waves signal now (see the 2026-11-13 amendments) |

   **Coordinates** are global display points with the primary display's
   top-left corner as origin and y downward (Quartz convention). With
   `display` (index in `NSScreen.screens` order, 0 = primary), they are
   relative to that display's top-left. A `point_at` while hidden repositions
   without a glide.

   **Limits:** 16 KiB per line, 1 MiB per script file, 1,000 subtitle
   characters, 32 shapes per sketch, 256 ink shapes until `clear_ink`,
   |coordinate| ≤ 100,000, radius ≤ 10,000, `ms`/`duration_ms` ≤ 60,000,
   display index ≤ 15. A display that is not connected rejects the event.

   **Host → runtime** status lines on stdout (`ready` with the display list and
   reduce-motion state, `displays_changed`, `bye`) and errors on stderr
   (`{"v":1,"type":"error","message":…,"line"?:n}`). Input is one local
   source per process: the bundled demo, a script file (all lines validated
   before anything plays) or stdin.
4. **Runtime seam.** The Rust runtime becomes a sidecar later by spawning the
   host and writing the same JSON lines to its stdin, reading status from
   stdout. Pipes keep the channel private to the two processes with no port or
   socket. Host → runtime inputs from the gameplan (`Ask`, `PushToTalk`,
   `Cancel`, `ApproveLook`) are not defined yet and need their own review.
5. **Motion.** Mirrors `web/`: "Arrow click" cursor (hotspot 7,6, #6575FF, filled,
   click lines fanned around the tip), the mark trailing at (+22, +18) with slower easing, 2.4 s bob,
   9 s blink and look-around loop (look-around only after the pointer rests 4 s),
   ink draw-on 0.75 s. Reduce Motion (system setting, followed live, or
   `--reduce-motion`) removes glides, trailing, fades, bob, blink and
   look-around; eyes still follow the pointer directly.

6. **Intro wave (amended 2026-11-13).** The maintainer: "when the presence
   mascot first comes on my screen, use this https://libraries.dev/waves design
   to come around the edges of my mac. it should appear from the bottom middle
   to the top, and go".
   - **When:** the first time Astrid comes on screen, which is at launch with
     `--ask`, or on the first `show` otherwise. A colourful wave rises from a pill
     at the bottom middle of the main display to the top, leaves its colour on
     the screen's edges, and clears, in about 3.6 s.
   - **How:** the wave is Libraries.dev Distortion Waves (vertical type, colourful
     palette, MIT, vendored in `Sources/AstridPresence/Waves`, licence in
     `ThirdParty/distortion-waves`). It is drawn in its own click-through,
     non-activating panel at `.screenSaver` level. The band shader is compiled at
     run time by the Metal framework into a `CAMetalLayer`, so building presence
     needs no Xcode Metal toolchain. The sparks are plain layers.
   - **Limits:** only the colour band and sparks are used. The library's lens and
     blur distort the content beneath, which the overlay never reads, so they are
     left out.
   - **Opt out:** skipped under Reduce Motion and with `--no-intro`.
     `ASTRID_WAVE_FRAMES=DIR` renders frames offscreen for checks.

## Alternatives considered

- **Tauri transparent windows** (D1 b): web rendering matches the site, but
  window level, Spaces and non-activation go through plugin layers, and M02 is
  paused.
- **GPUI** (D1 c): pending Phase 11 investigation; overlay window control is
  not established.
- **Unix socket or localhost listener** for the protocol: unnecessary while the
  runtime can own the host process; stdin keeps the surface smaller.

## Consequences

- V2-02 can drive `point_at`/`sketch`/`say` from model tool calls without host
  changes. Coordinates from an observed frame must still pass ADR 0023's
  native-coordinate and freshness rules before becoming presence events.
- The overlay's windows currently allow capture (`sharingType` default) so the
  maintainer can screenshot them. V2-02's LOOK must decide whether Astrid's own
  overlay is excluded from the frames it sends.
- D2–D5 remain open. This ADR does not authorize a resident timer host (D5),
  look grants (D2) or figure execution (D3), and does not make the host a daemon.
- Cost of changing later: High for the host technology; Low for additive
  protocol fields; Medium for coordinate semantics.

## Amendment 2026-11-13: cursor styles and the operator wave (V2-05, additive)

The maintainer asked for "/operator" to "use the different cursor styles from
the docs to do stuff" and, "if it is working, the distortion wave should be
visible". Two additive v1 fields; nothing existing changes meaning.

- **`point_at.cursor`.** An optional style from a fixed set that PresenceCore
  validates (`CursorStyle`): `arrow_click` (the default, Astrid's own),
  `arrow`, `click`, `pointer`, `press`, `tap`, `text`, `grab`, `select`,
  `crosshair`, `focus`, `zoom_in`, `zoom_out`, `pen`, `not_allowed`. An unknown
  name rejects the line. The art is copied from `web/src/components/cursors.tsx`
  on the same 32×32 grid, each with its own hotspot on the target. A `point_at`
  without `cursor` (and every `hide`) returns to `arrow_click`, so each point
  says what to do on its own. Styles are presentation only: none clicks, types
  or drags. The runtime's `point_at` tool takes the same optional `cursor`
  (`CURSOR_STYLES` in `src/computer.rs`).
- **`wave`.** Plays the intro's Distortion Waves on demand, for example when an
  operator session goes live. Still skipped under Reduce Motion and with
  `--no-intro`; a running wave is not restarted. The once-per-launch intro is
  unchanged. The wave panel belongs to the host process, so ADR 0027 captures
  exclude it like the rest of the overlay.
