# ADR 0036: Live canvas from voice asks

Date: 2026-11-13
Status: **Accepted 2026-11-13** by the maintainer's V2-04 request: "I should be
able to be on the voice mode, talk to it, and it generates based on that on a
canvas on screen ... astrid can move around with its cursor to point at stuff it
created ... at least create anything ... move from weaving, composing and
sketching". It settles v2 gameplan D3 as option (b), with a general shape
vocabulary instead of a few figure templates.

## Context

V2-04 wants Astrid to draw what you ask for, on screen, while you talk. D3 asked
whether the model writes figure code that runs in a sandbox (a) or fills a spec
that trusted templates render (b). "Anything" rules out a small set of figure
templates, but it doesn't need model-written code.

## Decision

- **Format.** The model draws by putting one fenced `canvas` block in its reply:
  one JSON shape per line on a 1000 × 640 canvas. The shapes are `title`, `rect`,
  `ellipse`, `line`, `polygon`, `path`, `arrow`, `text`, `box` (a flat
  isometric block) and `point`.
- **3D solids (amended 2026-11-13).** Physical subjects are built as real 3D
  Anatomy figures. `slab` is a rounded block and `cylinder` is a cylinder,
  cone or taper along x, y or z. Both use world coordinates (x and y on the
  ground, z up, origin at the canvas centre) and render through the vendored
  Anatomy kit (`web/vendor/anatomy`). They use one fixed isometric camera and
  the kit's shading, and the page keeps them back to front. `text`, `point`
  and `arrow` take world coordinates (a `z`, or `[x,y,z]` ends) to label 3D
  parts. Unfilled solids keep the grey Anatomy look; a `fill` tints one to
  highlight it. `point` moves Astrid's cursor
  to something it drew and shows a short label there. Shapes can also carry a
  colour, a stroke width, `dashed`, `sketchy` (hand-drawn) and a `label`. The
  instructions go in every presence ask, and the model decides when to draw.
- **The runtime validates.** `astrid presence-ask` reads the block as text
  streams in (`src/presence_canvas.rs`) and emits
  `{"type":"canvas","event":"begin"|"shape"|"end"|"cancel"}`:
  - **Shapes.** Each one is sent once its line is complete, rebuilt from known
    fields only.
  - **Limits.** Numbers are clamped and colours must be `#hex` or a palette
    name. Path data may use only SVG commands and numbers. Text is capped at
    160 characters and rendered as text, never markup. A reply carries at most
    160 shapes and one canvas.
  - **No streaming.** A provider that doesn't stream text is parsed from its
    final reply.
  - **Speech.** The block is removed from the answer that is shown and spoken.
- **The host draws.** The presence app opens a second floating panel
  (`CanvasPanel.swift`) when a drawing begins. It hosts `web/canvas.html` from
  the same read-only `astrid-ui://` folder as the ask surface. The panel takes
  clicks (close, drag) but never keyboard focus.
  - **One shape at a time.** The page draws each shape with fixed templates:
    the stroke draws in, then the fill.
  - **The cursor.** Astrid's Arrow-click cursor with the trailing mark travels
    to each shape.
  - **Phases.** The status pill moves between **Composing** (the block has
    started), **Sketching** (shapes are drawing), **Weaving** (waiting for the
    next shapes) and **Done**.
  - **Reduced Motion.** Shapes appear without animation.
- **Moving the figure (amended 2026-11-13).** The canvas fills the card. The
  title floats in a translucent pill; drag the pill to move the panel. On the
  drawing itself:
  - **Drag** turns a 3D figure, re-projecting every solid and world-anchored
    label (shift-drag pans).
  - **Two-finger scroll** pans, and **pinch** zooms.
  - **Double-click** resets the view.
  - **Framing.** Each drawing is fitted to the view as it grows, so nothing is
    cut off.
- **Follow-ups (amended 2026-11-13).** While the canvas is up, the ask
  surface's listening, runtime states and finish are also sent to the canvas.
  A state orb sits beside Astrid's mark, as in the docs' agent states, and
  goes away when she is idle. Answers that drew are flagged `canvas`, so the
  ask panel speaks them without showing a text box.
- **Authority.** No new tool, permission or capability. Drawing is
  presentation only: it reads nothing and changes nothing. Asks stay read-only
  runs with mail approvals (ADR 0033). Voice and spoken replies are unchanged
  (ADRs 0034/0035), so a dictated ask hears the short reply while the canvas
  draws.

- **Astrid Weaver (amended 2026-11-26).** The maintainer asked to "improve the
  quality of astrid's drawing ... call it our astrid weaver skill", to "change
  the color of the canvas ... if asked" and to "add motion for stuff it draws if
  necessary, like how a dam works, plane turbine", pointing at the plates in
  `references/weaver-engine`. These are the agent's calls, pending the
  maintainer's review:
  - **The skill.** The drawing instructions now live in `src/weaver_skill.md`
    (`WEAVER_SKILL`). They include a house style taken from the references: a
    monochrome figure, one glowing accent part, quiet labels and one spec card.
  - **Canvas colour.** A `canvas` line (`background`, `grid`) recolours the
    sheet. On a light sheet the ink, chrome and Anatomy solids switch to their
    light tones. A recolour is a redraw that starts with that line.
  - **Motion.** Any shape may carry `motion` (`spin`, `flow`, `pulse`, `bob` or
    `sway`) and a `speed` from -4 to 4. The page animates these with fixed CSS
    templates; no model code runs. Reduced Motion keeps everything still.
  - **Rotors.** `rotor` is a fan, propeller or turbine wheel (`blades`, `hub`,
    `pitch`, `axis`). With `spin` it is re-projected every frame, so it turns
    in 3D instead of rotating flat on the screen.
  - **Callouts.** `callout` is a spec card (`title`, an accent `tag`, up to 4
    `rows`) placed at `at`, with a leader line to a canvas or world point. All
    of its text is validated plain text.
  - **Glow.** `glow` adds a halo in the shape's own colour.
  - **Authority.** Authority is unchanged: this is presentation only.

## Alternatives considered

- **Model-written HTML, SVG or JavaScript in a sandboxed web view** (D3 a). This
  is more expressive, but it runs model output as code and needs a sandbox ADR.
  The shape vocabulary already draws most things. This option stays open behind
  that future ADR.
- **A `draw` tool call per shape.** This is more structured, but each call costs
  a model round trip. It would also need the tool-exchange admission and
  approval paths for something that has no side effects. A streamed block draws
  as fast as the model writes.
- **Drawing on the click-through overlay.** That layer belongs to the screen
  (point and sketch on a look frame, ADR 0027). A drawing you asked for needs a
  surface you can close and move.

## Consequences

- **Anything visual.** Diagrams, figures, sketches, layouts and isometric
  scenes can be drawn from any provider, including `claude-code/*`.
- **Model skill.** Drawing quality depends on the model's spatial skill. There
  is no figure-correctness suite yet: the V2-04 exit criterion "figure checks
  pass" is still open.
- **Not tied to the screen.** The canvas isn't placed "next to the subject" on a
  look frame. That needs ADR 0027 look plus canvas placement.
