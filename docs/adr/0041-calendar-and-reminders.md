# ADR 0041 — Calendar and Reminders through the presence host

Date: 2026-11-15
Status: **Accepted 2026-11-15 as the agent's call, pending the maintainer's review.**
The maintainer asked for Calendar and Reminders through EventKit, "the planned
next everyday slice after ADR 0028/0029", so that "What's my day?", "remind me
when I get home" and "move my 3pm" work from the presence ask, typed or spoken,
with writes confirmed in conversation. They asked for conservative calls,
recorded here, without stopping to ask. The read, disclosure and write choices
below are the conservative options and should be confirmed or changed.
Related: ADRs 0028 (read-only mail), 0029 (resident timers; its point 5 named
this slice), 0033 (ask surface), 0034 (voice), 0038 (operator cards), 0040 (memory).

## Context

ADR 0029 decided that reminders and calendar come through EventKit with a
confirmation before every create. The ask runs in `astrid presence-ask`, a child
of the presence app. The presence app already owns the capabilities that need
macOS grants (capture, Accessibility, microphone), and the runtime asks it for
them over the child's private pipes (ADR 0038).

## Decision

### Who owns what

- **The presence host owns EventKit** (`EventKitCalendar`, AstridPresence),
  like it owns capture. macOS asks for Calendars and Reminders access the first
  time, attributed to Astrid Presence. Contacts access is asked only for a
  "when I get home" reminder.
- **The runtime asks the host** over `presence-ask`'s existing JSON lines:
  stdout `{"type":"calendar","request_id","op",..}`, stdin
  `{"type":"calendar_result","request_id","ok","data"|"reason"}`. There is no
  socket or listener. The host validates every request again
  (`CalendarService`, PresenceCore) and answers with categorical reasons
  (`calendar_access_denied`, `event_changed`, `home_address_unavailable`, ...).
- **Opt-out:** `astrid-presence --ask-calendar off`; the runtime gets the tools
  only with `presence-ask --calendar`, which the host passes when it is on.
  It is on by default, like mail's Gmail default, because macOS's own prompt is
  the first gate. `astrid run` has no calendar.

### Tools

- `calendar_agenda {start_date, days ≤ 14}` reads whole local days: events and
  incomplete reminders due by the end of the range (at most 50 of each).
- `reminder_create {title ≤ 200, due | null, when_arriving: "home" | null}`.
- `event_move {event_id, start, end | null}` moves one occurrence and keeps its
  length unless `end` is given (at most 24 h, within a year).
- **There is no delete, and nothing else changes:** no event creation, edit of
  title/notes/attendees, completion, or reminder removal. A recurring event's
  series is never rewritten (`.thisEvent` only). Read-only calendars
  (subscriptions, holidays, birthdays) are refused before any question.

### What is read, and what goes to the model

- **Read by the host:** events' title, times, all-day flag, calendar name,
  location, URL and notes (notes and URL only to find a video-call link from a
  fixed list of hosts); reminders' title, due date, list and whether they have
  a location alarm; the user's own contact card's home address, only for a home
  reminder.
- **Sent to the ask's model provider** (as tool results, like mail): title,
  start, end, all-day, calendar name, location (200 characters), whether a
  meeting link exists, and whether the event can be moved; reminders' title,
  due, list and `when_arriving`. **Never sent:** notes, attendees, the link
  itself, the host's EventKit ids (the model sees `e1`, `e2`, ...), the home
  address or its coordinate.
- **Geocoding:** the home address is geocoded once per launch by Apple's
  `CLGeocoder`, so that address goes to Apple. The coordinate stays in memory
  and in the reminder's EventKit alarm.
- **Kept:** nothing new on disk. Results live in the private ask session like
  other tool results and never in metadata traces. Calendar text is untrusted
  data and can't grant anything.

### Writes are asked in words

- A write is validated, then worded as one question: "Can I move your 3pm
  Design review to 4pm?", "Can I remind you to water the plants when you get
  home?". The panel shows it with **Go ahead** / **Stop** (the operator card,
  ADR 0038), says it out loud, and voice answers it ("yes", "go ahead", "do it";
  stop words win). Nothing is sent to the host until Go ahead.
- One answer allows exactly that change once. Stop, Escape, cancelling the ask,
  anything unclear or 5 minutes without an answer means no.
- **Freshness:** a move carries the start the user was asked about; the host
  refuses `event_changed` unless the occurrence still starts there.
- Reading needs no per-call question: the ask is the request, and macOS's grant
  is the standing permission. This differs from mail (ADR 0028), whose reads
  are approved per call, because a day's agenda is the everyday answer itself.

### Location reminders

- "When I get home" uses an EventKit location alarm (`EKStructuredLocation`
  with a 150 m geofence, on arrival) at the coordinate of the home address on
  the user's own card ("My Card" in Contacts).
- If there's no card, no home address, Contacts is denied or geocoding fails,
  the host answers `home_address_unavailable`; the model is told to say so and
  ask for a time instead. Reminders' own location triggers then depend on the
  user's devices and Location Services, which Astrid can't check.

## Alternatives considered

- **AppleScript to Calendar.app.** No structured locations, slow, and needs
  Automation consent per app; EventKit is the supported API.
- **EventKit in the Rust runtime (objc bindings).** The grant would belong to
  whichever terminal runs `astrid`, the reason ADR 0027 moved capture into the
  host.
- **A per-call approval card for reads.** Safer by a little, but "What's my
  day?" would always ask twice. Reads are reversible and visible.
- **A CLLocation geofence of our own.** It needs Location Services for the
  presence app and a resident process watching location; Reminders already does
  this on the user's devices.

## Consequences

- The ask surface can answer the user's day and make two kinds of change, each
  after a spoken yes.
- The calendar read path is a new standing disclosure to the ask's model
  provider for every calendar question (bounded fields above).
- Proactive nudges (ADR 0042) read the same `CalendarSource` in the host.

## Validation

Tests use a fake host (`src/calendar_tests.rs`, `src/operator_bridge.rs`) and a
fake `CalendarSource` (PresenceCore `CalendarTests`). No live calendar,
reminder, contact or model call was made. Live use needs the bundle and the
maintainer's own Calendars, Reminders and (for home) Contacts grants.

## Cost of changing later

Medium. The tool shapes and disclosed fields are easy to adjust; adding writes
(create, delete, edit) would be a new decision.
