# ADR 0033: Presence ask surface

Date: 2026-11-13
Status: **Accepted 2026-11-13.** The maintainer asked to "do this gmail stuff on presence, not only terminal", with the logo only, a liquid-gooey input and agent states, and answers in the polish of Ardia's daily briefs (ardia-browser/ardia#192).

## Decision

- **The surface.** `astrid-presence --ask` shows only the Astrid mark in the
  bottom-right corner (no cursor) in a floating, typeable panel (`AskPanel`).
  The panel is separate from the click-through overlay.
  - Clicking the mark opens a `liquid-gooey` input. The send button springs
    out of the input's right edge through the goo once there is text.
  - While working it shows the docs' agent states: a `ThinkingOrb` and
    `AstridLogo` expressions mapped from runtime events.
- **The UI is the web code.** It is a React page in `web/src/presence`, built
  to `web/dist-presence` (`vite.presence.config.ts`), so it reuses the real
  components. A WKWebView serves it from that folder over a private
  `astrid-ui://` scheme, read-only and confined to the folder, because ES
  modules don't load from `file://`.
- **The runtime is a child process.** The panel starts
  `astrid presence-ask --model M --mail-source gmail`, a JSON-lines client of
  the runtime:
  - **In:** `ask`, `approve`, `cancel`.
  - **Out:** `state`, `delta`, `approval`, `answer` (with an optional `brief`)
    and `done`.
  - **Each ask** is a fresh **read-only** run (no shell, no writes) in an
    empty Astrid-owned folder, `~/Library/Application Support/Astrid/presence-workspace`,
    with read-only mail per ADR 0028.
- **Approvals.** Each mail call shows a one-time "Allow once / Don't" card in
  the panel. With no answer within 5 minutes it is denied. Escape cancels a
  running ask.
- **Briefs.** The model is asked to end with a `brief` JSON block: title,
  summary, to-dos, numbered updates and numbered sources. The UI renders it
  Ardia-style, with circle-check to-dos, source badges with a popup centred
  above, and a sources row. A Gmail source with a message id opens that
  message in the browser; only `https://mail.google.com` links are opened.
  Without a valid brief, plain text is shown.
- **Model.** The default is `claude-code/sonnet` (ADR 0032); `--ask-model`
  overrides it. Any Astrid provider works.

## Consequences

- **Visible app surface.** The presence app now has a clickable, keyboard-focusable panel.
- **Lifetimes.** The runtime stays a separate process; the panel never holds credentials.
- **Building.** The ask UI must be built with Node before use.
- **Debugging.** `ASTRID_ASK_SNAPSHOT=/path.png` saves the panel's own web
  content (never the screen) for headless checks.
