# ADR 0027 — Local Mac look, ephemeral disclosure and presentation tools

Date: 2026-11-11
Status: **Accepted 2026-11-11 (option 1)**. The maintainer answered "go for it, open the PR, when done" to the decision below.
Extended 2026-11-14 by [ADR 0038](0038-operator-walkthroughs.md) for operator sessions only: up to 12 looks per session (still one per run, each approved by one press) and bounded "Do it" input on this Mac. `astrid run --look` is unchanged.
Implementation notes: presentation calls validate against the run's latest look frame; two metadata-only events (`PresentationAllowed`, `ImageDisclosureCoveredByLook`) make the no-prompt paths explicit in the event state machine; the app bundle is launched through LaunchServices with private FIFOs.
Related: ADRs 0022–0025 (remote observation, actions, disclosure, deadlines),
0026 (presence host and protocol v1); [v2 gameplan](../plans/v2-gameplan.md) D2, V2-02;
[V2-02 plan](../plans/v2-02-ask-look-point.md).

## Context

V2-02 ("Ask, look, point") is the first slice where Astrid sees the user's own
Mac: a typed ask leads to one granted frame, then a vision model call, then
`point_at`/`sketch`/`say` on the right target. The maintainer instructed "work
on v2-02" after merging V2-01 (PR #29).

Everything accepted so far for observation and disclosure (ADRs 0022–0025) binds
an **exact remote InstaBox machine**. Its capture, decoder, limits and
`image.disclose` grant are reusable, but the target is not. Capturing the local
screen is a new authority over the user's most sensitive surface. That changes
permission and privacy semantics, so under the escalation policy it needs an
explicit decision. The model's new presentation tools also need a defined
authority. They are lower risk than ADR 0023 actions because they inject no
input.

## Existing constraints

- ADR 0022: single-use observe grants, a memory-only frame (at most 10 s, one
  per run), bounded PNG validation, no image in history or traces.
- ADR 0024: a separate per-frame `image.disclose` grant, only the exact models
  `gpt-5.6-sol/terra/luna`, `store:false`, the identity transform (native
  coordinates), at most 2560 px per side and 4,096,000 px in total, at most two
  disclosures per run.
- ADR 0025: 10 s upload/header admission, then 60 s for the response.
- ADR 0026: the presence host is a thin client, local-only, and currently has
  no capture capability.
- Gameplan D2 is still open. V2-02 needs only one look per ask.

## Options

1. **Recommended: the host captures, the runtime grants and discloses, and one
   approval covers both capture and disclosure.**
   - The Rust runtime (`astrid run --look`) spawns the presence host as its
     sidecar (ADR 0026 stdin seam). It reads host status from stdout, extended
     with a bounded framed frame message.
   - The host captures one display with ScreenCaptureKit
     (`SCScreenshotManager`, macOS 14+). It **excludes Astrid's own overlay
     windows** and captures at **1× point resolution**. Frame pixels are then
     exactly display points, so `point_at` needs no transform, and a
     1512×982 laptop frame fits ADR 0024's pixel limits without
     downscaling. Displays over 2560 points per side are rejected rather than
     resized.
   - **One single-use "look" approval per frame** names the display,
     whole-screen sensitivity, the provider, the model, `store:false` and
     memory-only retention. Approving it both captures and discloses that one
     frame, which is the gameplan's "approval as one keypress" for D2(a).
     Declining captures nothing.
   - The ADR 0022/0024 limits, decoder, expiry, history and trace exclusions
     and the ADR 0025 deadlines apply unchanged. The limit is at most two looks
     per run, with no saved, standing or "watching" grant.
   - Screen Recording permission belongs to a minimal `Astrid Presence.app`
     bundle built by a script. It is not granted to the user's terminal,
     because that grant would extend to every program run from it.
2. **Same as 1, but keep two prompts per look** (observe, then disclose),
   exactly like ADR 0024. This is the most conservative option and changes no
   existing grant semantics. The cost is a slower, clunkier ask, the opposite
   of v2's polish goal.
3. **The runtime captures directly** (for example `screencapture`) with no host
   involvement. This is simpler IPC, but Astrid's own ink and cursor appear in
   the frames, or must be hidden around each capture. It also gives Screen
   Recording permission to the terminal.

## Presentation tools (part of every option)

- `point_at`, `sketch` and `say` are presentation-only. They take no approval
  and inject no input.
- Each call must reference the run's **current frame**. Coordinates must lie
  within that frame's display bounds, in points. Calls without a valid frame,
  or after the frame or run has ended, are rejected. `say` needs no frame.
- Limits per run: 24 presence calls, ADR 0026 protocol limits, and text of at
  most 1,000 characters.
- Results are recorded as ordinary tool results: the categorical outcome and
  the coordinates, never the image.
- The host renders only validated events. No model output reaches the host
  directly.

## Recommendation

Option 1.

## Why

- It keeps every byte and time limit, the history rules and the provider
  restrictions already accepted. Only two things change: the target (this Mac)
  and the number of prompts (one per look). The approval still names exactly
  what is sent, and to whom.
- Capturing in the host excludes Astrid's own overlay cleanly. Capturing at 1×
  makes `point_at` exact with no coordinate transform.
- Scoping Screen Recording to an Astrid app bundle is much narrower than
  granting it to the terminal.

## Cost of changing later

High. It sets privacy semantics, approval UX and permission ownership.

## Live validation

Development uses synthetic frames, a mock capture source in the host, loopback
HTTP and mock inference only. Capturing the maintainer's real screen and sending
it to OpenAI needs a separate exact trial approval: which display, the model,
the number of looks and the asks.

## Decision required

Accept option 1, with one combined look approval per frame, host-side
ScreenCaptureKit capture at 1× excluding the overlay, and a Screen Recording
grant scoped to an Astrid app bundle? The alternatives are option 2 (two prompts
per look) or option 3 (runtime capture).
