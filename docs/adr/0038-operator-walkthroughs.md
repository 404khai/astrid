# ADR 0038 — Operator: guided walkthroughs with "Show me" and "Do it"

Date: 2026-11-14
Status: **Accepted 2026-11-14.** The maintainer asked for V2-05 "/operator ...
should be able to have screen context of what's on my screen, and use the
different cursor styles from the docs to do stuff. if it is working, the
distortion wave should be visible". Asked about look grants, guide versus act
and the model, the maintainer answered "make the calls, the decisions", with a
reference card: "Step 3 of 4 · Your site needs a bucket. Want me to create it?
[Do it] [Show me]". The calls below are the agent's, made on that instruction.
Related: ADRs 0022–0025 (observation, actions, disclosure, deadlines), 0026
(presence protocol; 2026-11-13 cursor/wave amendment), 0027 (local look),
0033 (ask surface); [v2 gameplan](../plans/v2-gameplan.md) D2, V2-05.

## Context

V2-05 turns one look into a walkthrough: look, propose a step, let the user (or
Astrid) do it, look again, verify, move on. Three things were open:

1. **D2, look grants.** ADR 0027 allows one approved frame per look and two
   looks per run. A walkthrough needs a look after every step.
2. **Guide or act.** The reference card offers both "Do it" and "Show me".
   Acting on this Mac is new authority: ADR 0027 has no input actions for the
   local target, and ADRs 0023/0024 bind actions to an exact InstaBox machine.
3. **Model.** Only the ADR 0024 path carries images (`gpt-5.6-sol/terra/luna`
   through the Codex subscription). The ask surface defaults to
   `claude-code/sonnet`, whose adapters send no images.

## Decision

### Session and state machine

- `/operator <goal>` in the presence ask input starts a session (`astrid
  presence-ask` recognises the command; `--operator-model` and
  `--operator-display` configure it; `astrid-presence --operator-model` passes
  the model through).
- The session is an explicit state machine in `src/operator.rs`
  (`Walkthrough`): look → report → step card → choice → look → verify. Each
  look is its own ADR 0027 run whose reply must be one ```` ```operator ````
  JSON block, validated by the runtime: step index/total (at most 8 steps),
  the point inside that frame, a cursor from the fixed `CURSOR_STYLES`, an
  optional action, and the check that proves the step done.
- **A step is never advanced past unless a newer frame verifies it**
  (`verified: true`). Unverified: the same step is retried with fresh
  coordinates, at most twice, then the session stops. Jumping ahead or
  finishing from an unverified step stops the session rather than skipping.
  Finishing is allowed only before any step or after a verified one.

### Looks (D2 answered as option a)

- **One approval per look, made a single press.** A look happens only after
  the user presses a card button that says Astrid will look: **Look** (start),
  **Done** ("Press Done ... and I'll look again to check") or **Do it** ("then
  look again to check"). The press arms exactly one `local.look` approval for
  the next run; every other permission ask in that run is refused. The start
  card names the display and the model and says nothing is saved.
- ADR 0027 is unchanged per run (capture at 1x excluding Astrid's windows,
  frame memory-only for 10 s, one image per request, never replayed, limits and
  deadlines). New: **at most 12 looks per operator session**. No watching or
  standing grant exists; option (b) stays open for evidence from use.

### "Show me" and "Do it" (guide and act)

- **Show me** is presentation only: Astrid points at the target with the
  step's cursor style (ADR 0026 amendment) and circles it in ink, and the card
  asks the user to press Done.
- **Do it** performs exactly the one action the card names, on this Mac:
  - **Actions:** left click at the point; click then type at most 200
    characters (one line); one key from enter, tab, escape, space and the
    arrows, with no modifiers; or scroll up/down 1–10 notches. There are no
    drags, shortcuts, right clicks or modifier keys.
  - **Model guidance:** the model must give no action for passwords, payments,
    deleting or sending, so those steps can only be shown.
  - **Authority:** the press on the card that names the action. Approval is
    per step and single use; nothing is remembered.
  - **Freshness:** ADR 0023's 5-second rule assumes no human reading in
    between, so it is adapted. When the user presses Do it, the host captures
    the display again and compares a 96-point square around the target with
    the look the step was planned on. It compares a coarse grayscale
    fingerprint (one cell per 4 points) and refuses (`stale`) if the mean
    difference is above 6/255, or if the look is over 120 s old or isn't the
    latest. The fingerprint stays in the host process's memory, is replaced by
    the next look and never leaves the process. The reference look stays the
    planning frame. The freshness capture is never sent anywhere.
  - **Host checks:** the host validates the action again
    (`OperatorAction`), needs Accessibility access for the Astrid Presence
    bundle (otherwise nothing is posted), and never posts input when the
    capture source is `synthetic`.
  - **No retry:** an acknowledgement is not success. The next look verifies
    the step. A refused or failed action falls back to Show me, and no look
    happens until Done.
- This extends ADR 0027 (no local input) and ADRs 0023/0024 (actions only on
  an exact InstaBox machine) **for operator sessions only**. `astrid run
  --look` still has no actions.

### Model

- Operator uses the ADR 0024 image path: `gpt-5.6-sol` by default, or terra or
  luna. No provider restriction changes. Ordinary asks keep their own model
  (default `claude-code/sonnet`).

### Visible signal

- The Distortion Waves wave (`wave` event) plays once per session as soon as
  the first look is granted, with the usual Reduce Motion and `--no-intro`
  opt-outs. In `--ask` mode the overlay (cursor, ink) appears only while
  operator runs and hides when the session ends.

### Step cards

- Step cards are native, non-activating panels beside the target (they never
  cover it), so the user's app keeps keyboard focus.
- Only buttons the card offered count. Anything else, a cancel or 10 minutes
  without a press counts as Stop.

## Amendment 2026-11-14: Claude, plain asks and voice

The maintainer: "why is it using gpt sol, it should use claude i don't have a
codex sub rn", then "I don't think I always need to say operator, i should be
able to talk normally and it will infer what I mean. build both (Claude as
operator's model, and voice operator)".

- **Model.** Operator now defaults to `claude-code/sonnet` (`claude-code/opus`
  also allowed; gpt-5.6-sol/terra/luna stay available). This extends ADR 0024's
  image path to a second destination for **local looks only**
  (`computer::vision_destination`): the approved frame goes to Anthropic
  through the Claude Agent SDK on this Mac's Claude Code login (ADR 0032, with
  its subscription-terms caveat, which the maintainer accepted). The bridge
  sends it as one base64 PNG image block in a single streaming user message.
  The frame never enters the replayed prompt or history, the grant is consumed
  against the live frame immediately before the bridge starts, the local frame
  is released before the turn runs, and the visual turn has the same 60 s
  deadline. The approval card names "Claude (sonnet)"; retention follows
  Anthropic's Claude Code data policy (there is no `store:false` there).
  InstaBox computer use stays OpenAI-only; `astrid run --look` is unchanged.
- **Plain asks.** When operator is configured, every presence ask carries a hint:
  for a screen task the ask model replies only with an ```` ```operator-start ````
  block `{"goal": ...}`, and the session hands off to operator with that goal.
  Nothing is captured until the user presses (or says) Look on the start card.
  `/operator` remains as an explicit shortcut.
  *Follow-up the same day:* "unsubscribe from this channel" on a YouTube tab was
  read as an email question (mail search plus a brief). Clear on-screen requests
  (an on-screen reference such as "this channel/page/tab/video", or "turn
  on/off ...", and no mention of mail) now go straight to operator with no model
  call, after dropping a leading "Astrid"/"hey Astrid". The hint now leads the
  task and says plainly that "this" means the screen and that mail tools are for
  email only.
- **Voice.** While a step card waits, the page keeps a copy, voice mode keeps
  listening, and a final transcript is sent as an `utterance`. The runtime maps
  it to one of the card's offered buttons (`choice_from_speech`): stop words win
  ("don't do it" stops), unclear speech presses nothing and Astrid says what to
  say. Spoken asks get spoken cards. A spoken "do it" counts as the same press
  as clicking it: it approves exactly that card's action and the following look.

## Amendment 2026-11-14: real frames, real reasons

The maintainer's first try stopped with "computer stopped" after granting
Screen Recording. Development frames were synthetic and flat; a real 1x screen
(photo wallpaper, gradients) encodes to more than ADR 0022's 1 MiB PNG cap, and
the host's reason was replaced by a generic stop message.

- **Fit, don't resize.** The host now writes 8-bit RGB PNGs (no alpha) and, only
  if the full-colour frame is over 1 MiB, quantizes colours to 6, 5, then 4 bits
  per channel until it fits. The size and coordinates stay exact (1x display
  points) and every limit is unchanged. A photo-like 1512×982 test frame is
  2.4 MB in full colour and 0.75 MB at 4 bits; the runtime's decode worker
  accepts the result.
- **Say why.** The host's categorical reason (`screen_recording_permission_required`,
  `frame_png_exceeds_1mib`, ...) is kept in the look result and shown to the
  user as an actionable sentence, never as `computer_stopped`.
- **Permission.** The host tries ScreenCaptureKit before trusting the permission
  preflight (which can lag a fresh grant). A first attempt to keep grants across
  rebuilds with an identifier-only designated requirement failed: tccd logged
  "Failed to match existing code requirement" because TCC does not honour such a
  requirement on ad-hoc code. Ad-hoc signing therefore stays matched by exact
  code hash (stable when the code is unchanged; a code change needs a fresh
  grant, e.g. `tccutil reset ScreenCapture dev.astrid.presence`). A real signing
  identity in `ASTRID_SIGN_IDENTITY` keeps grants across rebuilds.

## Alternatives considered

- **(b) A time-boxed "watching" grant.** Fewer presses, but it is a standing
  capture authority over the user's most sensitive surface. Per-press looks
  already fit the walkthrough's rhythm (a look follows each step anyway).
- **Guide only.** Safer, but it ignores the reference card's "Do it".
  Bounded, per-press, freshness-checked actions keep the new authority small.
- **ADR 0023's 5 s freshness unchanged.** No user can read a card that fast;
  the result would be that "Do it" always fails.
- **Image support for the Claude adapters.** It needs new adapter work and a
  second disclosure provider, which can come later as a separate amendment.

## Consequences

- The operator can see this Mac on request and, step by step, act on it with
  the user's press. Every look and every action is user-triggered, single-use
  and visible on screen.
- Accessibility access for the presence bundle becomes a prerequisite for "Do
  it", not for "Show me".
- The look count, actions and fingerprint are new limits that tests pin:
  `src/operator_tests.rs`, `src/operator_bridge.rs` and PresenceCore
  `OperatorTests`.

## Live validation

Development used the synthetic capture source, a fake look host and mock
inference only. Running operator on the maintainer's real screen, sending
frames to OpenAI, or posting real input needs a separate exact trial approval:
the display, the model, the number of looks, the asks, and whether "Do it" may
be pressed.

## Cost of changing later

Medium. Grant semantics and the action set are permission meaning. The card
UI, cursor styles and limits are easy to adjust.
