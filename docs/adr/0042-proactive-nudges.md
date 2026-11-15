# ADR 0042 — Proactive nudges and when Astrid may interrupt

Date: 2026-11-15
Status: **Accepted 2026-11-15 as the agent's call, pending the maintainer's review.**
The maintainer asked for Astrid to "bring things up without being asked: a
morning brief, 'you have a meeting in 10 minutes, want me to open the link?',
an important email arriving", built on the calendar surface (ADR 0041), mail
(ADR 0028) and memory (ADR 0040), with "a clear interruption rule", and asked
for conservative calls without stopping. Every choice below is the
conservative one and should be confirmed or changed.
Related: ADRs 0029 (resident host owns timers), 0033 (ask surface), 0035
(spoken replies), 0037 (desktop Settings).

## Context

Until now Astrid speaks only when spoken to. Unasked interruptions are the
easiest way to make an assistant annoying or embarrassing (a mail subject on a
shared screen), so the rule has to be explicit, testable and default-off.

## Decision

### Who runs it

- **The resident presence host owns it** (ADR 0029): `astrid-presence --resident
  --ask`. Its frame loop calls `NudgeController` at most once a minute. There
  is no daemon of Astrid's own, no network listener and no launch at login.
  If the host isn't running, nothing is brought up and nothing is queued.
- **The rule is pure and tested** (PresenceCore `Nudges.swift`:
  `NudgePlanner` proposes, `Interruption` decides). The host only gathers
  signals and shows the result.

### Opt-in, per kind

- Three kinds, **each off until turned on** in the desktop app's Settings →
  Nudges: morning brief, meeting about to start, important email. Settings live
  in `~/Library/Application Support/Astrid/nudges.json` (`$ASTRID_NUDGES_FILE`),
  0600, written atomically; numbers are clamped on read. A missing file means
  all off; an unreadable file means all off and is never overwritten.
- What was shown (ids and times, 2 days) and which mail ids are already known
  live in `nudge-state.json` (0600), so a restart doesn't repeat a nudge.

### The interruption rule (in order)

A nudge is shown only if all of these hold; otherwise it waits for a later
check while it is still current, or is dropped:

1. Its kind is turned on (else dropped).
2. It hasn't expired and hasn't been shown before (else dropped). Each nudge is
   shown at most once: the brief once a day until noon, a meeting from N
   minutes before (1–30, default 10) until it starts (2 min after, with a
   link), an email for 2 hours.
3. Not in quiet hours (default 21:00–08:00, wrapping midnight).
4. No Focus or Do Not Disturb (`INFocusStatusCenter`, after macOS's "share
   Focus status" prompt). If Astrid isn't allowed to know, nudges may still
   show but are never spoken.
5. The screen isn't locked.
6. The screen isn't being shared (best effort: a known sharing helper such as
   Zoom's `CptHost` is running).
7. Not in a call: no camera in use, and no microphone in use by another app
   (when Astrid's own wake word holds the mic, only the camera counts).
8. The frontmost app isn't full screen or presenting (a window the size of a
   display).
9. The user isn't typing (a key in the last 5 seconds).
10. No ask, operator session or open ask panel is on screen.
11. At most 8 a day, and at least 20 minutes since the last one (2 minutes
    for a meeting about to start, which also goes first).

### How it shows

- A short bubble beside the presence mark with at most two buttons (e.g.
  **Open link** / **Not now**). No sound and no notification. Unanswered after
  2 minutes counts as Not now.
- Spoken (local voice, ADR 0035) only if the user turned on "Say nudges out
  loud" **and** Focus is known to be off.
- The morning brief uses the user's first name if they asked Astrid to remember
  it ("my name is ...", ADR 0040). Memory is read on this Mac and only shown.

### Never acts without a yes

- A nudge does nothing until its yes button. Then exactly one thing happens:
  - **Open link** opens the meeting's URL in the browser, only for the fixed
    meeting hosts in `MeetingLink` (https).
  - **Brief me** / **Read it** asks Astrid the shown question ("What's my day?",
    "Read the email from Ada about ..."), exactly as if the user had typed it;
    that ask follows its own rules (ADR 0028 approvals for mail, ADR 0041
    questions for calendar writes). Its answer is spoken only if the nudge was.
- Planning and deciding call no model. Nothing goes to a model until a yes.

### Mail checks (amends ADR 0028 for this use only)

- With "Important email" on, the host runs `astrid nudge-mail` every 10
  minutes: one Gmail search (unread, last 2 days, not promotions/social, and
  Gmail's Important label or a listed sender), or for Apple Mail one search per
  listed sender (Apple Mail has no readable importance marker). It prints only
  id, sender and subject of at most 20 messages; no snippet, body or model call.
- **This is a standing, header-only read without a per-call approval**, which
  ADR 0028 otherwise requires. Turning the nudge on in Settings is that
  approval. Turning it off stops the checks.
- The first check only learns which messages already exist, so turning it on
  never brings up old mail.

### Calendar

- Calendar nudges read the host's ADR 0041 `CalendarSource` (EventKit) every 5
  minutes, today and the next 24 hours. Nothing from the calendar is sent
  anywhere by a nudge.

## Alternatives considered

- **macOS notifications.** Focus would filter them for free, but they are a
  different surface from "the presence mark talking", persist in Notification
  Centre (on a shared screen too), and macOS shows them during calls.
- **A model deciding what's worth saying.** More helpful in time, but
  unpredictable and it would send calendar and mail to a provider unasked.
- **Showing nudges when the signals can't be read.** Rejected for speech; a
  silent bubble is allowed when only Focus is unknown, otherwise nudges would
  never appear for an unsigned build.

## Consequences

- Astrid can be proactive within a small, testable rule; every kind is opt-in.
- Signals are heuristics (sharing and full screen especially) and will miss
  cases; the defaults lean toward waiting.
- Answering a nudge by voice, and Astrid proposing new kinds, are not in this
  slice.

## Validation

PresenceCore `NudgeTests` covers each rule, quiet hours, rate and daily limits,
spoken versus silent, the planner's wording, memory names and mail baselines;
`src/nudge_mail.rs` tests the query bounds and header-only output. No live
calendar, mail, Focus, call or model was used.

## Cost of changing later

Low to medium. Limits and wording are easy to adjust; the mail amendment and
any non-opt-in kind would be permission decisions.
