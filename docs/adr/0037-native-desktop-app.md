# ADR 0037: Native desktop app (SwiftUI, Liquid Glass) beside presence

Date: 2026-11-13
Status: **Accepted 2026-11-13** as a test run. The maintainer: "let's work on a
native appkit / swift app, liquid glass. the main app should be able to be
closed, and removed from dock without disturbing the presence, this is a test
run. a session should have the chat i SENT, and astrid's reply along with the
diagram for that session. create session names based on the content of the
first message of that session. it should just feature a sidebar for now with
chats, and the main content to the right."

## Context

The desktop direction has moved several times. ADR 0013 (native SwiftUI/AppKit
with a helper) was superseded by ADR 0014 (Tauri 2, M02). M02 was then paused,
and the 2026-10-15 revision named GPUI as the future desktop direction pending
Phase 11. Since then the presence host (ADR 0026) has shown that a Swift
package with AppKit hosts, the private `astrid-ui://` web views and the
`astrid presence-ask` JSON-lines child (ADRs 0033/0036) work well on macOS. The
maintainer asked for a full desktop app on that same native stack, with macOS
26 Liquid Glass.

## Decision

1. **Direction.** A native SwiftUI/AppKit app becomes the desktop direction for
   this test run, instead of GPUI. GPUI is not removed or ruled out; this is a
   test of a native app beside presence. Tauri (M02) stays paused.
2. **A separate app and process.** `apps/astrid-desktop` is its own Swift
   package, so its look comes from the macOS 26 SDK. Its executable is
   `astrid-desktop`, bundled as `Astrid.app` with the id `dev.astrid.desktop`.
   - It is a normal Dock app (`Foreground`, not `LSUIElement`).
   - Astrid Presence stays `dev.astrid.presence`, a menu-bar accessory at
     macOS 14+.
   - The desktop app never starts, signals, waits on or talks to presence. Each
     is launched by LaunchServices on its own.
   - Closing the window quits the desktop app, and so does Quit. Either way
     presence keeps running. Removing Astrid from the Dock changes only the
     Dock's own preferences.
   - Shared code is a library rather than a shared process. `AstridWebUI` (the
     read-only `astrid-ui://` file server) moved out of the presence executable
     into a library product of `apps/presence-mac`, and both apps use it.
3. **Layout.** One window (`Window`, not `WindowGroup`). It is a
   `NavigationSplitView`:
   - **Sidebar:** chats, most recent activity first, with a New Chat toolbar
     button and ⌘N.
   - **Main content:** the selected chat's thread and a composer.
   - **Liquid Glass:** the real macOS 26 SwiftUI APIs. The sidebar and toolbar
     come from the system. The composer uses `glassEffect`, inside a
     `GlassEffectContainer` and a `safeAreaBar` with a soft scroll edge.
     Buttons use `.glass` and `.glassProminent`. No blur stands in for glass.
   - **SDK stamp:** SwiftPM writes the binary's SDK version from the deployment
     target, and AppKit picks the system look from that stamp. That is why this
     package targets macOS 26 while presence stays at 14.
4. **Chats on disk.** Each chat is a JSON file,
   `~/Library/Application Support/Astrid/Chats/<id>.json`. The folder is 0700,
   the files are 0600 and writes are atomic. Version 1 holds:
   - the chat's id, title, created and updated times
   - its messages. A user message is the text the user **sent**. An Astrid
     message is the reply text, the brief if there was one (ADR 0033), the
     **validated canvas shapes** of its drawing in order (ADR 0036) and how the
     run ended.

   **Loading:** files that are over 8 MiB, unreadable, from a newer version or
   named for a different id are skipped and never rewritten.

   **Limits:** 64 KiB of text per message, 200 shapes per drawing and 500 chats
   listed.

   This store belongs to the desktop client and is separate from the runtime's
   private idle sessions (ADR 0012) and from traces (ADR 0009). It holds what
   the user saw. When a reply quotes mail, that quote is on disk in the user's
   own Library folder. That is the point of a chat history, but it is new
   persisted content, so it is recorded here.
5. **Chat names.** The name is set once, from the first message, by a fixed
   rule rather than a model:
   - take the first sentence or line
   - drop greetings and polite openers ("hey astrid", "can you", "please", …),
     matching whole words only
   - cut to at most 6 words or 40 characters, adding "…" if anything was cut
   - capitalise the first letter

   "New chat" is the fallback.
6. **Diagrams.** A drawing is shown inline by the same canvas page
   (`web/src/canvas`) in a `WKWebView` per reply, in a new `?embed` mode:
   - **Embed mode:** no close button and no window drag. Plain scrolling stays
     with the thread.
   - **Replies made in this window** stream shape by shape, with Astrid's
     cursor, like the presence canvas.
   - **Saved replies** are sent as one `canvas`/`replay` event and drawn at
     once, framed, with the cursor hidden.
   - **Interaction:** drag turns a 3D figure, pinch zooms and double-click
     resets.
7. **Runtime.** The app starts its own `astrid presence-ask` child on the first
   ask: the same protocol, read-only runs, mail approvals and model default as
   ADR 0033, with approvals shown as Allow once / Don't cards in the thread.
   - **One addition:** `ask` takes an optional `history`, a list of
     `{role: user|astrid, text}` with earlier turns (a drawing is described by
     its title).
   - **Bounds:** the runtime keeps the most recent 12 turns, up to 2,000
     characters each and 12,000 in total. Bad entries are dropped.
   - **Placement:** the history goes in front of the task as context only. With
     no history the task is unchanged, so presence behaves exactly as before.
   - **No runtime state:** the runtime persists nothing new.
   - **Child lifetime:** the child ends when the app quits, or when its stdin
     closes because the app was killed.

8. **Look, themes and settings (amended 2026-11-13).** The maintainer: "design
   the sidebar to be like my radian browser ... allow me to theme the app, like
   sidebar and main content ... a settings modal and themes" (references in
   `references/radian`, `references/rex`).
   - **Sidebar, after Radian.** The window has a hidden title bar and the
     system's window material (`containerBackground(.thinMaterial, for:
     .window)`) under a theme tint. On it:
     - round `.glass` buttons beside the traffic lights (hide sidebar, new chat)
     - a glass search pill that filters chats, with a sliders button that opens
       Settings
     - a "+ New Chat" row, then one-line chat rows; the selected one is a
       raised, outlined capsule
     - the Astrid mark and a Settings button at the bottom

     The main content is an inset rounded card. Collapsing the sidebar (⌃⌘S)
     lets the card fill the window. As in Arc, resting the pointer on the left
     edge then **peeks** it. Peeking is the same sidebar with the same
     slide-in motion as opening it; the only difference is that opening
     pushes the content card over, while a peek slides in inside the card's
     inset box (its 8 pt margins and rounded left corners) over the card's
     content, and the card stays put. Docking from a peek leaves the sidebar where it is and moves
     only the card. Its toggle docks the sidebar, and
     leaving the panel hides it. The docked sidebar is resizable from its
     right edge (200–380 pt). The top row follows Radian: the toggle on the
     left and new chat on the right, on the window buttons' centre line
     (measured from AppKit). The **window buttons** show only on a docked
     sidebar at least 236 pt wide, never while peeking; ⌘W, ⌘M and the Window menu keep working
     when they are hidden. This custom sidebar replaces the
     `NavigationSplitView` above; Liquid Glass stays on every control.
   - **Themes.** Each theme sets the sidebar tint and its strength, the
     sidebar text, the content card, text, secondary text, accent and the
     user's bubble. There are four light and four dark built-ins, as plain hex
     data in `DesktopCore`. The light and dark themes are chosen separately,
     and the app follows the system appearance or a forced Light or Dark.
     Drawings keep their dark canvas.
   - **Settings, after Rex.** A sheet (⌘,) with a searchable page list:
     - **General:** the ask model and mail source. They apply from the next ask
       by restarting the app's own child, and launch flags win over them.
     - **Themes:** Appearance, then Light Mode and Dark Mode rows of theme
       cards, each with a Default and Built-in group.
   - **Storage.** Preferences live in the app's UserDefaults domain. They carry
     no new authority: mail stays read-only with an approval for every call.

## Alternatives considered

- **A new executable target in `apps/presence-mac`.** It would raise presence
  to macOS 26 or leave the app on the macOS 14 SDK stamp, which has the old
  look. Rejected for the sibling package.
- **One app for both presence and the window.** Quitting the window app would
  take presence down with it, which the maintainer ruled out.
- **The runtime owning chat history (ADR 0012 sessions).** The sessions are
  workspace-bound coding snapshots with tool data and provider continuation.
  Chat display history is client state, and passing bounded history keeps the
  runtime stateless for asks. This can be revisited when chats need tool
  continuity.
- **Re-rendering diagrams natively in SwiftUI.** That would duplicate the
  canvas templates and the 3D Anatomy kit. Reusing the page keeps one renderer.

## Consequences

- **Desktop direction:** the 2026-10-15 GPUI direction is paused, not
  superseded, until the maintainer judges this test run. Presence and desktop
  are separate apps that share libraries and the web UI build.
- **Follow-ups:** a follow-up gets the earlier text and drawing titles. It does
  not get the earlier shapes, so "make it bigger" redraws from the description.
- **Out of this slice:** voice, look, multi-window, deleting or renaming
  chats and imported themes.
- **App icon:** the maintainer's Icon Composer `AstridIcon.icon` is compiled by
  `actool` into the bundle's `Assets.car` and an `.icns` at bundle time. It is
  read from the repo root (or `ASTRID_ICON`) and is not committed, because one
  of its layers is a third-party stock image and the repository is public.
- **Cost of changing later:**
  - **Low** for the layout.
  - **Medium** for the chat file format, which is versioned.
  - **Low** for the additive `history` field.
