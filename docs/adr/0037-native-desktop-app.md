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

9. **Diagrams page, haptics and the coloured mark (amended 2026-11-14).** The
   maintainer asked for "a tab for diagrams ... all the diagrams in a grid
   format ... open them in a modal", haptics as the sidebar opens, closes and
   expands, the coloured Astrid logo, and for the toggle and new-chat buttons
   to keep the place they have when the sidebar is closed.
   - **Buttons:** the toggle and new chat sit at the start of the chat card's
     header in every state. The docked sidebar's top row holds only the window
     buttons, and a peek draws the same pair in the same spot over the header.
   - **Diagrams:** a Diagrams row under New Chat shows every saved drawing,
     newest first (`ChatGallery`), as a grid of replayed thumbnails. Their web
     views take no clicks, so the whole tile opens the drawing.
   - **Modal:** clicking a thumbnail opens the drawing in an interactive modal
     with Open Chat. Clicking outside, Esc or × closes it. Nothing new is
     stored; it reads the same chat files.
   - **Haptics:** these use `NSHapticFeedbackManager` and only work on Force
     Touch trackpads.
     - **Opening and closing** gives taps along the 0.3 s slide, then a firmer
       bump when the sidebar reaches full width. Closing ends on a light tap.
     - **Docking a peek** gives a firm bump.
     - **Dragging the edge** gives a detent every 20 pt and a firm bump at the
       minimum and maximum width.
   - **Coloured mark:** drawn natively from the web logo's paths and gradient,
     on the new-chat page and in the sidebar.

10. **Peek material, sidebar edge, quieter peek and blob groups (amended
    2026-11-15).**
    - **Peek material:** a peek now uses the window's own behind-window material
      under the same tint as the docked sidebar.
    - **Sidebar edge:** pushing the edge 40 pt past the minimum width closes the
      sidebar.
    - **Peek haptics:** one tap when it peeks and one when it leaves. The docked
      slide keeps its sequence.
    - **Blob groups (cosmetic):** a Giovanni button, drawn as a little macOS
      icon beside the waveform, opens a picker. Picked blobs preview together
      in a dark circle, and "Make group" saves a `BlobGroup` (name and member
      blob files) in the app's preferences.
      - **Sidebar:** groups show under Diagrams as their blobs in a dark circle
        plus the name; deleting is in the context menu.
      - **Chat:** a group opens a mock chat with Astrid's animated mark up top,
        the blob count, and seeded charts (a per-blob progress ring and bars,
        an activity line, status chips).
      - **Rendering:** all of it comes from `web/group.html` and the blob
        renderer.
      - **Scope:** nothing is sent to a model and nothing is written to the chat
        store.

11. **Goblins replace blobs (amended 2026-11-16).** The maintainer: "port
    https://github.com/404khai/goblins. add support for all its animations ...
    these will replace blobs ... use bramble ... to create the outline icon".
    - **Library:** the Goblins library (MIT, the maintainer's own) is vendored
      in `web/src/goblin` with `three`.
    - **Clan files:** its clan is written by `web/scripts/build-goblin-clan.ts`
      as goblins/v1 recipe files in the repo's `goblins/`. Users can drop studio
      exports into `~/Library/Application Support/Astrid/Goblins`.
    - **Desktop app:**
      - The sidebar row (`web/goblins.html`) shows live 3D goblins.
      - The picker, group avatars and small rows use flat SVG goblins.
      - The group chat uses live goblins with a mood per message, the "writing"
        spell for typing and the "progress" spell on the pulse card.
      - Clicking any live goblin plays its next animation: all 15 moods, then
        all 15 spells.
    - **Composer icon:** a native white outline glyph of Bramble, drawn from the
      maintainer's beardless recipe (beady eyes, long ears, gnome hat).
    - **Phone:** the phone app keeps its blobs for now, so `blobs/` and
      `blobs.html` stay.

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

## Amendment 2026-11-14: voice mode, providers, About

At the maintainer's request, the composer's empty-state button is a waveform:
- **Voice mode.** Pressing the waveform turns the input into Astrid with her
  voice ring, and you talk straight away. What you say is sent when you stop.
  Her reply is spoken, drawings included (she names what she drew), and then
  she listens again.
- **Leaving it.** A keyboard button (or Esc) goes back to typing, and typing
  turns the waveform into Send.
- **Same rules as presence.** It reuses presence's contracts: microphone only
  while listening and on-device recognition only (ADR 0034), and the local
  Kokoro voice or macOS's (ADR 0035). Nothing is saved or sent besides the
  transcript asked.

Settings › General groups the known models by provider (Claude Code, Claude
API, Codex, OpenCode Go), with each provider's mark from `providers/` and an
install or key status. The runtime still takes any exact model ID under "Other
model". Settings gains About.

## Amendment 2026-11-14: blobs

At the maintainer's request:
- **The row.** The sidebar shows a row of up to five "blobs" (little bots)
  above its foot. Click one to pick it; the picked one reacts while Astrid
  works.
- **Settings › Blobs** (Giovanni icon) holds the show/hide toggle.
- **The renderer** is the MIT-licensed `@bible-strong/avatar-web` package.
- **The characters.** Asked about the licence, the maintainer chose to keep
  Avatar Lab's ten built-in characters in the repo, in `blobs/` under their
  own AGPL-3.0 licence (with credit and the changes listed in its README),
  and to license the rest of Astrid under MIT (root `LICENSE`). The
  characters were converted to the MIT runtime's character format.
  Settings › Blobs picks which five show (default: Sunee, Cloudee, Citrus,
  Freddy, Kirby). The user can add their own exports in
  `~/Library/Application Support/Astrid/Blobs` (at most 512 KiB per file).
- **If Astrid ever needs to be free of AGPL** (for example, a closed-source
  build), deleting `blobs/` removes it; the app runs without bots.
- **Voice mode's ring** is presence's `VoiceRing` (`web/voice.html`), fed by
  the host's analyser frames.

### Amendment 12 — goblins as a dependency, expressions only, paste to add (2026-11-16)

Maintainer: "bun add github:404khai/goblins#v0.7.0 for now no spells only expressions. allow me to add bots by
pasting json. the goblin face of the icon in the composer, is not so good, make it better". The vendored
`web/src/goblin` is replaced by the `@404khai/goblins` v0.7.0 git dependency (`/flat` and `/react` entry points;
pnpm `allowBuilds` lets its `prepare` step build `dist`). Goblins show expressions only: clicking one cycles the
fifteen expressions, and no spell or morph plays in the row, picker or group chat. Settings → Goblins gains an
"Add goblins" field: pasted JSON (a goblins/v1 file, one goblin or a list, at most 512 KB) is checked with the same
`recipes(in:)` as files and saved as one goblins/v1 file (0600) in the Goblins folder, then shown everywhere. The
composer icon is now `goblins/bramble-glyph.svg`, drawn by the library's `goblinGlyphSvg` from the beardless Bramble
recipe (`web/scripts/build-bramble-glyph.ts`) and shown as a template image.

### Amendment 13 — group chat panel and goblin profiles (2026-11-16)

Maintainer: "increase the goblins size in a group chat ... the astrid logo and the pill should allow content
scroll under ... their background should be transparent ... if I click the pill, it should [open] a side panel
from the right ... the logo, made up of the animated goblins, 3 max ... every goblin in that group, the name of the
group, the description, with a save button ... click on a goblin ... their profile in a modal", then "put a settings
icon in the top right ... the right panel should be full height". A group chat's page now fills the whole content
card under the app's floating header row; the mark and pill float with no background over a chat that scrolls
under them and fades out at the top (a mask, no painted band). Faces are 64 px. The pill and a top-right gear open
a full-height side panel: up to three live goblins overlapping, editable name and description, the roster, and
Save, which sends `{type:"save", name, about}`; the app stores `about` as an optional `GoblinGroup` field (at most
280 characters; older stored groups decode unchanged). A roster goblin opens a profile modal: the goblin live (click
for its expressions), its skin and traits. Still cosmetic: no model is involved.

### Amendment 14 — live goblin groups (2026-11-16, agent's calls pending maintainer review)

Maintainer: "make goblins like grok bots, chatgpt dots ... they should be able to tag each other in a group ...
sending context between each other. the name and description of a group is sort of the ai prompt ... each person can
have a role and store it to memory. the floating astrid in a group is the sort of overseer, its eyes can react ...
it can also talk in chat ... react to messages ... change expression ... I should also be able to chat to them".
Groups stop being cosmetic:

- **Threads.** Each group's messages are kept in `Application Support/Astrid/Groups/<group id>.json` (0600, at most
  500 messages, removed with the group). A message has an author (`you`, `astrid` or a goblin id), text, mood
  (a Goblins expression), mentions and reactions (at most 6 emoji, one each per author).
- **Turns.** A user message goes to whoever it @tags, or to Astrid (the overseer) if nobody is tagged; each reply's
  tags queue those members next. A round is bounded: 6 turns, 2 per speaker, Stop ends it. Every turn is one ask to
  a separate `astrid presence-ask` child for groups **without a mail source**; any approval it raises is
  **declined**, so goblins only talk. The "context" sent between goblins is the bounded recent thread (6,000
  characters) plus who handed over the turn and their words.
- **Prompt.** The group's name and description are its purpose; then the speaker's role, its memory, the roster and
  the thread. The reply ends with a hidden ```group JSON block: `mood`, optional `react` (one emoji on the message
  it answers) and optional `remember`.
- **Roles and memory.** Roles are set per goblin in the side panel (`GoblinGroup.roles`, optional, 120 characters;
  older groups decode unchanged). A goblin's memory is its own list (`Astrid/Goblins/Memory/<id>.json`, 0600, 40
  facts), separate from the user's ADR 0040 memory; a role saved is noted there, `remember` adds to it, and the
  panel lists it with Forget all.
- **Astrid's eyes** in the group header: tagged → listening (with a pop), a question → solving, a round running →
  searching, Astrid streaming → composing, else idle with pointer tracking.
- **UI.** Live faces are 84 px and show thinking/talking while a goblin answers and its mood after. Reactions use
  the Reaction Bubble design (black emoji bubble, a small bubble with who reacted); hovering a bubble offers six
  quick picks. The composer is native Liquid Glass with @-suggestions; the settings button is a native glass
  circle in the card header, like New Chat and the sidebar toggle. The page's own gear and fake composer are gone.

### Amendment 15 — @all and inline Bklit charts (2026-11-16, agent's calls pending maintainer review)

Maintainer: "add an @all tag ... each individual tag should have the goblin beside it. me tagging @all I expect to
get a response from each goblin. the goblins should use https://bklit.com/ for inline rendering and choose the best
one for each situation". `@all` (or `@everyone`) tags every goblin, in roster order; the round's budget grows to
everyone tagged plus two, so each goblin answers once. Every rendered tag shows its goblin beside it (all of them,
overlapped, for @all; Astrid's mark for @astrid), as do the composer's @ suggestions. A reply's ```group block may
carry one `chart`: a JSON spec of kind line, area, bar, sparkline, pie, ring, funnel, gauge or radar, with a guide
in the prompt for picking the kind. The app keeps it on the message only after checking the kind, finite numbers,
at most 64 items per list and 8 KB; the page checks it again and draws it with Bklit components (MIT, vendored from
its shadcn registry into `web/src/bklit`; adds motion, d3-array/scale/shape, @number-flow/react, clsx,
tailwind-merge, and Tailwind v4 with no preflight for the presence pages build). The model never sends code:
only data drawn by fixed components, as with the canvas (ADR 0036).

### Amendment 16 — group turns never look (2026-11-16)

A group turn's prompt says "this group" and "here", which `presence-ask`'s screen-task routing (ADR 0038) took as
on-screen requests, so turns tried a look and failed with "observation deadline". `presence-ask --no-operator`
(new, hidden like the command) leaves the operator out entirely: no routing, no hint, no look. The desktop app's
group child always passes it. A short tag such as `@moss` now tags the one goblin whose name starts that way.
