# Astrid v2 gameplan: the over-the-shoulder agent

Status: **Proposed** (maintainer direction, 2026-11-11, branch `v2`). D1 is
**accepted for the presence host** (option a, [ADR 0026](../adr/0026-presence-host-and-protocol.md),
2026-11-11, when the maintainer started V2-01). D2–D5 still need explicit answers
before dependent implementation.

## 1. What Astrid is now

> Astrid is a very personal agent. It **explains** (sees your screen when you
> ask, then points, sketches, builds working diagrams and walks you through
> things) and it **handles the everyday** (timers, alarms, reminders, calendar
> events, finding that email). Out loud, right where you are. Laptop first,
> phone next.

The reference experience is heyclicky (Farza): an assistant that lives on top of
the desktop with its own cursor and ink rather than in a chat tab. Astrid adds
two things of its own:

1. **Anatomy figures.** Explanations as crafted, interactive isometric objects
   ([404khai/anatomy](https://github.com/404khai/anatomy)), drawn on screen and
   driven by the same model that the readout reports.
2. **An answerable runtime.** Every look and every action is an explicit,
   visible, single-use grant with finite limits and metadata-only traces. That
   is the existing Rust harness, and it is what makes "personal" trustworthy.

Polish is a requirement, not a later phase: motion, latency and voice quality
are product features.

### Interaction model

```text
hold ⌃⌥ / type ─► ask ─► LOOK (one frame, granted) ─► THINK (vision model)
                                     │
             ┌───────────────────────┼────────────────────────┐
             ▼                       ▼                        ▼
         POINT (cursor)        SKETCH (ink)           FIGURE (anatomy card)
             └──────────────► SPEAK + SUBTITLES ◄─────────────┘
                                     │
                     WALKTHROUGH: wait ─► re-look ─► verify ─► next step
                                     │
                    DO IT FOR ME (later): ADR 0023/0024 bounded actions
```

### Astrid's presence

Astrid's cursor **is the Astrid mark**, not a generic arrow: a small dark chip
with the logo whose eyes follow the user's own pointer, plus a name tag and a
speaking indicator. When teaching, it can hold a **pointer stick**, like a
teacher at a whiteboard, with the stick's tip as the hotspot. This makes
"Astrid is pointing" unmistakable next to the system cursor and keeps the
character personal. The eyes follow the *pointer*; following the pointer needs
no screen grant, and the UI says so.

Presence is three layers over the user's untouched apps: **screen** (observed,
never modified), **ink** (sketches, highlights, figures), **presence** (Astrid's
cursor, voice and subtitles, always visibly Astrid's, never the system pointer).

## 2. Harness audit: keep, freeze, add, tweak

~20k lines of Rust in one crate. The audit is by fit to the v2 loop above.

### Keep: core of v2

| Area | Files | Why it matters for v2 |
|---|---|---|
| Runtime, events, cancellation | `runtime.rs`, `events.rs`, `cancellation.rs` | Every ask is a run; barge-in by voice equals cancellation; overlay renders events. |
| Screen observation | `computer.rs`, `computer_png.rs`, `computer_wire.rs`, `visual_upload.rs` (ADRs 0022, 0024, 0025) | The single-frame, memory-only, grant-per-look path **is** v2's LOOK. Deadlines (10 s upload, 60 s response) are already measured. |
| Bounded desktop actions | `desktop.rs` (ADR 0023) | Native coordinates, five-second freshness and per-action authority are the future "do it for me". Also the coordinate model POINT reuses. |
| Context, sessions, permissions | `context.rs`, `sessions.rs`, `session_store.rs`, `permissions.rs` | Conversations across asks; private v2 snapshots; permission presets. |
| Provider and auth | `model.rs`, `openai.rs`, `auth.rs`, `routing.rs` | Vision inference already wired; routing picks a vision- and latency-appropriate model. |
| Observability | `observability.rs`, traces | Needed to hit latency targets honestly (ask → first point). |
| CLI console | `console/*`, `main.rs` | Stays the development and validation client. |

### Freeze: keep the code, take it off the critical path

These are well built but do not serve the v2 loop now. Nothing is deleted; each
stays tested and documented, and gets no new feature work until a v2 milestone
needs it.

| Area | Files | Note |
|---|---|---|
| Scheduler and batch workers (Phase 8) | `scheduler.rs`, `batch_*.rs` | Useful later for "research this while I work". |
| Remote VM execution | `instabox*.rs`, `remote*.rs` (ADR 0021) | Relevant to the thin-client principle for the phone, later. |
| Coding-specific inspection | `changes.rs`, `native.rs`, coding prompts | Coding stays a capability, not the identity. |
| GPUI spatial experiment | `apps/desktop-experiments/spatial` | Visual reference for spatial UI; not the overlay host (see D1). |

Recommendation: a later cleanup ADR may move frozen areas behind Cargo features
to shrink the default binary. That is not urgent and is not done here.

### Add

1. **Presence host (macOS).** A transparent, click-through, all-Spaces overlay
   per display: Astrid cursor, ink canvas, subtitles and figure cards. A thin
   client over a local, private IPC event stream from the Rust runtime. *(D1)*
2. **Presence protocol.** Versioned runtime → host events (`PointAt`,
   `Sketch{shapes}`, `ClearInk`, `Say{text, final}`, `ShowFigure{spec}`,
   `StepChanged`) and host → runtime inputs (`Ask`, `PushToTalk`, `Cancel`,
   `ApproveLook`). Coordinates are display points tied to the frame they came
   from, reusing ADR 0023's native-coordinate and freshness rules.
3. **Explanation tools for the model.** `point_at`, `sketch`, `say` and
   `show_figure`: presentation-only tools with **no input injection**, so they
   are lower authority than ADR 0023 actions, but still bound to a specific
   observed frame.
4. **Push-to-talk voice.** A global hotkey (⌃⌥), mic capture, streaming STT,
   streaming TTS with word-timed subtitles. Speaking over Astrid cancels the run.
5. **Anatomy figure runtime.** Render figures inside a sandboxed web view in
   the overlay with the vendored kit; the model supplies a figure spec. *(D3)*
6. **Walkthrough engine.** A step state machine (instruct → wait → re-look →
   verify → advance) whose re-looks follow the chosen grant semantics. *(D2)*
7. **Everyday personal tasks** (new first-party tools, each a typed
   capability with its own permission class):
   - `timer.start/cancel/list` and `alarm.set`: owned by the runtime, so they
     survive the chat. They need a resident process (menu-bar host) and fire
     through overlay and system notifications. **No daemon exists today**:
     this milestone introduces the smallest resident host and must be
     scoped (see D5).
   - `reminder.create` and `calendar.event.create/list`: through EventKit on
     macOS. Creation always shows a confirm card first (external side
     effect); listing is read-only.
   - `mail.search/read`: **read-only**, per-ask grant. First the local Mail.app
     store through Spotlight metadata (no credentials leave the device); IMAP or
     Gmail OAuth later under a separate credential ADR. Results show sender,
     subject, date and snippet. Sending or replying is out of scope until a
     drafting milestone with confirm-before-send.
8. **Grounding evaluation.** A Phase 6–style suite: about 30 synthetic and real
   screenshots with labeled targets, measuring pointing hit rate,
   ask-to-first-point latency, cost and figure correctness. This is the main
   quality bar.
9. **Later:** phone companion (existing Expo app; Android overlay through a
   Kotlin module; iOS through share/broadcast with no cross-app overlay), and
   memory under user control.

### Tweak

- **Product identity docs.** AGENTS.md now carries a dated v2 direction entry.
  The architecture north star stands; v2 narrows what ships first.
- **Defaults for a personal agent.** Look grants stay explicit, but the UX must
  make approval feel like one keypress, not a dialog (for example "hold ⌃⌥"
  means "you may look now"). This depends on D2.
- **Latency is a first-class metric.** Add trace spans for STT, capture, upload,
  first token, first `point_at` and TTS start. Targets: under 1.5 s to first
  spoken word and under 2.5 s to first point (to be measured, not assumed).
- **Copy and naming.** Meister/Astrix stay: Astrix become *skills with a face*
  (Figures, Sketchpad, Guide, Reader), as shown on the website's Home Base.

## 3. Milestones

Each milestone ships something you can feel on your own screen, validated with
deterministic mocks first. Live screen and model trials need exact scope
approval, as with ADRs 0021–0025.

| ID | Milestone | Exit criteria |
|---|---|---|
| V2-00 | **Direction and website** (this branch) | New positioning, site redesign, this gameplan. |
| V2-01 | **Presence on the Mac** (`apps/presence-mac`, in review) | Overlay host draws Astrid's cursor, ink and subtitles from a scripted event stream; multi-display, Spaces, full-screen apps, reduced motion, click-through verified. No model. |
| V2-02 | **Ask, look, point** | Typed ask → one granted frame → vision model → `point_at` / `sketch` on the right target; grounding suite baseline recorded. |
| V2-03a | **Everyday tools** (mail + timers in review; ADRs 0028/0029; EventKit next) | Timers and alarms that survive the chat; reminders and calendar events through EventKit with confirm cards; read-only local mail search. Each tool is mocked, then trialled live on exact scope. |
| V2-03 | **Voice** | Push-to-talk, streaming STT/TTS, subtitles, barge-in cancellation; latency spans in traces. |
| V2-04 | **Anatomy figures, live** | `show_figure` renders a sandboxed, interactive figure card next to the subject; figure checks pass. |
| V2-05 | **Guided walkthroughs** | Multi-step tasks with wait/re-look/verify; never advances on an unverified step. |
| V2-06 | **Phone companion** | Phone asks routed to the Mac runtime; Android overlay prototype. |
| V2-07 | **Memory you control** | Inspectable, deletable personal context. |
| later | **Do it for me** | ADR 0023/0024 actions behind the same presence UI. |

## 4. Decisions needed

**D1: Presence host technology**

- Context: v2's core surface is a system-wide overlay. M02 (Tauri) is paused,
  ADR 0014 is superseded, and GPUI is the pending Phase 11 direction.
- Options: (a) a native Swift/AppKit overlay app (NSPanel, ScreenCaptureKit,
  global hotkey, AVAudio) as a thin client, with the Rust runtime as a sidecar
  over local IPC; (b) Tauri transparent windows; (c) GPUI.
- Recommendation: **(a)**. Overlays, capture, hotkeys and audio are AppKit-native.
  Heyclicky-grade polish needs exact window levels and animation, and the Rust
  runtime stays authoritative. AppKit stays narrowly in the host.
- Cost of changing later: **High.**
- Decision: *Adopt a native macOS presence host with the Rust runtime as
  sidecar, superseding GPUI as the first desktop surface?*
- **Answer (2026-11-11):** accepted for the presence host. The maintainer's
  "let's start with V2-01 ... the overlay of astrid's logo, plus cursor" is the
  go-ahead; [ADR 0026](../adr/0026-presence-host-and-protocol.md) records the
  host and presence protocol v1. Rust sidecar IPC is a later slice (stdin pipe seam).

**D2: Look-grant semantics for conversations and walkthroughs**

- Context: ADR 0022 grants exactly one frame per approval. Walkthroughs need
  repeated re-looks.
- Options: (a) keep one approval per look and make approval a single keypress;
  (b) a visible, time-boxed "watching" grant (for example 2 minutes, one app,
  with an always-on indicator and instant revoke).
- Recommendation: **(a) first**, then evaluate (b) with evidence from V2-05.
- Cost of changing later: **Medium** (permission semantics).
- Decision: *Ship V2-02 to V2-05 with per-look approval only?*

**D3: How figures execute**

- Options: (a) the model writes figure code run in a sandboxed, network-less web
  view; (b) the model fills a constrained figure spec rendered by trusted
  templates built on the kit.
- Recommendation: **(b)** to start (safe, fast, checkable), then add (a) behind a
  sandbox ADR.
- Cost of changing later: **Low–Medium.**

**D4: Roadmap precedence**

- Recommendation: the v2 milestones take precedence over Phases 8–12 ordering.
  Phase 8 scheduler work is frozen, not reverted. Phase 6 evaluation methodology
  carries into the grounding suite.
- Decision: *Make the v2 milestones the active roadmap?*

**D5: A resident host for timers and alarms**

- Context: timers must fire after the ask ends. AGENTS.md has excluded
  daemons so far.
- Options: (a) the presence host (menu-bar app) owns timers and alarms locally
  and persists them, with the runtime as a client; (b) a background Rust
  daemon; (c) delegate to the system Clock and Reminders apps only.
- Recommendation: **(a)**, with (c) for reminders through EventKit.
- Cost of changing later: **Medium.**
- Decision: *Let the presence host stay resident in the menu bar and own
  timers and alarms?*
- **Answer (2026-11-11):** yes; see [ADR 0029](../adr/0029-resident-presence-timers.md).

## 5. Website (done on this branch)

`web/` was redesigned around the new positioning, keeping the existing design
language (black, #4459F9/#1A32EC, Space Grotesk and IBM Plex Mono, square
hairline grids):

- **Hero:** Nexvyn laptop mockup with a live macOS scene. Astrid reads a PDF
  with the user, builds an Anatomy-kit isometric engine whose valves, spark,
  flywheel and stroke readout all come from one crank model, then glides its
  own cursor from part to part with hand-drawn ink and subtitles.
- **How it helps:** a bento grid of points, sketches, walkthroughs, figures,
  privacy, phone (Nexvyn phone mockup) and push-to-talk.
- **Everyday:** Clicky-style light cards on the blue bands: mail search, timers
  and alarms, calendar, teach any app, and a large Astrid mark whose eyes follow
  your mouse. Labeled as planned, not shipped.
- **Explainer poster:** after `references/color_font.jpeg`. Blue blocks with
  halftones and coordinate tags around a light panel, where Astrid holds a
  pointer stick over an animated DNA double helix.
- **Hero callouts:** plain text beside the laptop, no boxes. Astrid's cursor
  is now the logo chip with tracking eyes. Page background is Hoplite's
  `#0a0a0a`.
- **Spatial:** an exploded view of the three presence layers.
- **Home base:** a Hoplite-style app shell (Hugeicons) over an animated
  ordered-dither landscape, with Astrid hanging on a thread and writing its
  greeting in VerseText.
- **Underneath** and **Gameplan** sections reframed for v2.

The engine figure is generated at build time with `node web/scripts/build-figure.mjs`
from the vendored kit in `web/vendor/anatomy`.
