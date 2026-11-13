# Astrid presence (macOS)

V2-01's presence host: Astrid's own cursor (the "Arrow click" from
`web/src/components/cursors.tsx`) with the Astrid mark trailing it, hand-drawn
ink and subtitles, drawn over every app on every display from a scripted event
stream. No model, no screen capture, no input injection, no network listener.
The mark's eyes follow your real pointer through `NSEvent.mouseLocation`, which
needs no permissions. Contract: [ADR 0026](../../docs/adr/0026-presence-host-and-protocol.md).

Requires macOS 14+ and Swift 5.9+ (Xcode or Command Line Tools).

The desktop chat app is a separate package, `apps/astrid-desktop`
(ADR 0037). It shares this package's `AstridWebUI` library and runs as its own
process, so quitting it never affects presence.

```sh
cd apps/presence-mac
swift build                                   # build
swift test                                    # 29 deterministic tests (protocol, geometry, motion, ink)
.build/debug/astrid-presence                  # play the demo tour (same as --demo)
.build/debug/astrid-presence --print-demo > /tmp/tour.jsonl   # demo as editable JSON lines
.build/debug/astrid-presence --script /tmp/tour.jsonl         # play a script file
.build/debug/astrid-presence --stdin < scripts/example.jsonl  # stream events on stdin
.build/debug/astrid-presence --reduce-motion                  # force Reduce Motion behaviour
```

Quit with **Ctrl-C** in the terminal, the **Astrid icon in the menu bar → Quit
Astrid Presence**, or a `{"v":1,"type":"quit"}` event. `--exit-at-end` quits
once the input ends and the queue drains.

The first time Astrid comes on screen, a colour wave (Libraries.dev
[Distortion Waves](https://libraries.dev/waves), MIT, vendored in
`Sources/AstridPresence/Waves`) rises from the bottom middle of the main display
to the top, lingers on the screen's edges and goes. `--no-intro` skips it, as
does Reduce Motion; `ASTRID_WAVE_FRAMES=/tmp/wave .build/debug/astrid-presence`
renders a few of its frames offscreen to PNGs.

`--snapshot DIR` writes PNGs of Astrid's own layers (never the screen beneath)
once a second; it is how the host is checked without Screen Recording access.

## Voice (ADR 0034)

With `--ask`, hold **⌥Space** anywhere (or hold the mic in the input) to talk;
let go to send. A quick tap on the mic keeps it open until the next tap; Esc
cancels. Speech is recognised on-device only. Microphone access needs the app
bundle's usage strings, so test it from the bundle:

```sh
(cd web && npx vite build -c vite.presence.config.ts) && cargo build \
  && apps/presence-mac/scripts/bundle.sh \
  && open "apps/presence-mac/.build/Astrid Presence.app" --args --ask
```

## Protocol v1 in one screen

One JSON object per line, `#` lines are comments in scripts. Coordinates are
global points from the primary display's top-left corner (y down), or relative
to a display with `"display": n` (0 = primary). The `ready` line on stdout lists
the displays.

```jsonl
{"v":1,"type":"show","x":700,"y":400}
{"v":1,"type":"point_at","x":300,"y":250}
{"v":1,"type":"sketch","shapes":[{"kind":"circle","x":300,"y":250,"r":60}]}
{"v":1,"type":"say","text":"This is the button you want."}
{"v":1,"type":"wait","ms":2500}
{"v":1,"type":"sketch","shapes":[{"kind":"arrow","from":[500,500],"to":[330,280]}]}
{"v":1,"type":"say","text":"Streaming partial","final":false}
{"v":1,"type":"clear_ink"}
{"v":1,"type":"hide"}
{"v":1,"type":"quit"}
```

## Layout

- `Sources/PresenceCore`: protocol parsing/encoding, script player, display
  geometry, motion (glide, trailing, gaze, bob, blink), ink geometry, the mark
  and cursor art, demo generator. No AppKit; fully unit tested.
- `Sources/AstridPresence`: the AppKit host (overlay panels, Core Animation
  layers, frame loop, stdin reader, menu-bar item).

## Timers and alarms (ADR 0029)

```sh
.build/debug/astrid-presence --stdin --resident   # stays in the menu bar
{"v":1,"type":"timer_start","id":"tea","seconds":180,"label":"Tea"}
{"v":1,"type":"alarm_set","id":"wake","at":"2026-11-12T07:30:00Z","label":"Wake up"}
{"v":1,"type":"timers_list"}
{"v":1,"type":"timer_cancel","id":"tea"}
```

Timers persist in `~/Library/Application Support/Astrid/timers.json` (owner-only)
and fire once on the next launch if they came due while the host was not running.
The menu bar lists them; click one to cancel. Notifications appear only when
running as `Astrid Presence.app` (`scripts/bundle.sh`).
