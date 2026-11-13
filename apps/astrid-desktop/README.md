# Astrid (macOS desktop app)

Astrid's desktop chat app ([ADR 0037](../../docs/adr/0037-native-desktop-app.md)).
It is a normal Dock app, `Astrid.app` (`dev.astrid.desktop`). The sidebar on the
left lists your chats, most recent first; the selected chat is on the right.
Each chat keeps what you sent, Astrid's replies and the drawings it made.
Saved drawings re-render inline: drag to turn a 3D figure, pinch to zoom,
double-click to reset.

The sidebar follows the Radian browser: glass buttons, a search pill whose
sliders open Settings (⌘,), and an inset content card. ⌃⌘S hides the sidebar; then rest
the pointer on the window's left edge to peek it: it slides in exactly as
when opened, but inside the chat card, over its content, instead of pushing it.
Drag the sidebar's edge to resize it. The window buttons show only while the
sidebar is open and wide enough for them.
Settings follows Rex: General (model, mail source) and Themes, with four
light and four dark themes that colour the sidebar and the chat, and System,
Light or Dark appearance.

It is separate from Astrid Presence (`apps/presence-mac`). Closing the window,
quitting the app or removing it from the Dock never affects presence.

Requires **macOS 26+**, because it uses the Liquid Glass SwiftUI APIs.

```sh
# from the repo root: build the web UI and the runtime, then the app
(cd web && npx vite build -c vite.presence.config.ts) && cargo build \
  && apps/astrid-desktop/scripts/bundle.sh \
  && open apps/astrid-desktop/.build/Astrid.app

cd apps/astrid-desktop
swift build && swift test        # 19 deterministic tests (store, titles, child protocol, themes, search)
.build/debug/astrid-desktop --help
```

- **Answers.** They come from the app's own `astrid presence-ask` child, with
  the same read-only runs and mail approvals as the presence ask panel
  (ADR 0033). The model defaults to `claude-code/sonnet`; change it with
  `--model`, and choose the mail source with `--mail gmail|apple|all|none`.
- **Icon.** `scripts/bundle.sh` compiles `AstridIcon.icon` from the repo root
  (or `ASTRID_ICON=/path/to.icon`) with `actool`; without it the app keeps the
  generic icon.
- **Chats.** They are saved in `~/Library/Application Support/Astrid/Chats`
  (`--chats DIR` to use another folder).
- **Debug.** `ASTRID_DESKTOP_SNAPSHOT=/path.png` saves the app's own window
  (not the screen) and prints its window number. `ASTRID_DESKTOP_ASK="text"`
  sends one ask at launch. `ASTRID_DESKTOP_PEEK=1` starts with a closed sidebar peeking. `ASTRID_DESKTOP_SETTINGS=1` opens Settings
  (`ASTRID_DESKTOP_SETTINGS_PAGE=themes` for that page). `ASTRID_DESKTOP_CLOSE_AFTER=seconds` closes the
  window as its close button would.
