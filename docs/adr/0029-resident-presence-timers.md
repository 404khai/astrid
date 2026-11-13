# ADR 0029: Resident presence host owns timers and alarms (gameplan D5 a)

Date: 2026-11-11
Status: **Accepted 2026-11-11.** The maintainer answered "make all the decisions, yes for both".
Related: ADR 0026 (presence host), v2 gameplan D5 and V2-03a.

## Decision

1. **Resident mode.** `astrid-presence --resident` keeps the presence host
   running as a menu-bar app after its input ends. It still opens no network
   listener. It accepts the same protocol v1 input (stdin, or a script file),
   plus new local events:
   - `timer_start {id, seconds, label}`
   - `timer_cancel {id}`
   - `alarm_set {id, at, label}`, where `at` is a UTC ISO-8601 time
   - `timers_list`
2. **Local state.** Timers persist in
   `~/Library/Application Support/Astrid/timers.json`, written atomically and
   readable only by the user (0600).
   - **Limits:** 32 active timers, timers of 1 s to 24 h, alarms within 7 days,
     labels of at most 120 characters.
   - **Restart:** a timer that expired while the host wasn't running fires once
     on the next launch, marked late.
3. **Firing.** When a timer fires, Astrid shows itself with a subtitle,
   plays the system alert sound and posts a notification.
   - **Notification permission:** when running as the app bundle, it uses a
     standard notification, which macOS asks permission for.
   - **Cancelling:** the menu bar lists active timers and cancels them.
4. **Runtime role.** The Rust runtime is a client of the host for timers. It
   never runs a daemon of its own.
5. **Reminders and calendar.** These are created through EventKit, with an
   in-overlay confirmation card before every create. They are a follow-up
   slice; this ADR records the decision.

## Consequences

Timers keep working while no ask is running, as long as the menu-bar host is
running. If the host is quit or the Mac is asleep, timers fire late, on the
next launch. Launching at login is not part of this decision.

Cost of changing later: Medium.
