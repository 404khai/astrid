# ADR 0023 — Separate bounded desktop actions and frame admission

Date: 2026-10-21
Status: Proposed — awaiting explicit maintainer decision
Implementation: Design prepared; dependent action authority code awaits decision.
Related: ADRs 0004, 0008/0009, 0016, 0018, 0021/0022;
[active plan](../plans/remote-mobile-cli.md).

## Context

The maintainer requested CU-02: separately authorized clicks, typing, scrolling
and keys with stale-frame checks and execution limits, then a PR. CU-01's accepted
contract permits no implicit action grant, image disclosure or runtime observation
integration. A separate action/coordinate contract must precede dependent code.
CU-01 has only synthetic observation fixtures; a fresh real frame cannot yet be
obtained by the Rust runtime. This slice can implement headless Rust controls
with mocked transport without claiming end-to-end computer use.

## Proposed decision

- Add a distinct host-owned `gui.act` approval broker and nonserializable,
  noncloneable, atomic single-use action grant. Bind principal, run ID, runtime
  call ID, provider call ID, exact host-selected machine, unique frame ID, native
  dimensions and exact canonical action arguments. Approval is per action, never
  an action list, local preset, observe/exec grant, resume, selection or worker
  permission. Unknown fields/action types fail closed. No key loading in core.
- Implement one exclusive, run-owned controller for one exact target. It accepts
  host-attested observation metadata only: same principal/run/target, newly minted
  opaque frame ID, native decoded width/height and monotonic observation admission
  time. Frames must satisfy CU-01 pixel/dimension limits. Metadata is not proof of
  image capture, decoding, focus, pointer location or exclusive desktop state.
  No public model tool/CLI can mint receipts. The new host API is only a trusted
  in-process seam, not authenticated cross-device trust or a screenshot decoder.
- An action names exactly the current frame ID. Use native image pixels with
  origin at the top left, integer x/y within width/height, no crop/downscale/DPI
  transform or inferred scaling. Do not derive coordinates from unapproved model
  image inputs. Any transformed-image coordinate path remains gated. Reject a
  frame at age >=5 s measured from observation admission (including capture and
  approval time), replacement, known geometry change, cancellation or run end.
  Reject future/invalid times. Validate immediately before asking and again at
  admission. After any admitted action consume the frame even if the call fails;
  every next action needs separately authorized fresh observation. One mutable
  controller serializes observation replacement/invalidation and action dispatch.
  External changes may still race; this is not a remote compare-and-swap guarantee.
- Initially support only: one click (left/right/middle, no double click); exact
  UTF-8 typing (1–1024 bytes, no NUL; preserve spaces, Unicode and newlines);
  scroll (up/down/left/right, 1–10 notches at the existing pointer); one key or
  chord (one allowlisted base key, up to three unique Ctrl/Alt/Shift modifiers).
  Base keys are lowercase ASCII a–z, digits 0–9, Enter, Escape, Tab, Backspace,
  Delete, Left, Right, Up, Down, Home, End, PageUp, PageDown and Space. Modifier
  order is preserved in the canonical tuple; duplicate modifiers are rejected.
  Scroll does not move the pointer. Typing and keys use current focus. Approval
  explicitly states pointer/focus uncertainty and that even an apparently simple
  click/key/text can submit an irreversible operation. No safe-command or safe-text
  classifier; the user reviews the concrete action. No move/drag/open URL, arbitrary
  shell, clipboard, key-down/up state, sequences, lifecycle, takeover or viewer.
- Fixed limits: at most 2 admitted actions per run, 30 s action window from first
  observation receipt, 3 s whole transport wait per action, at most 2048 cumulative
  UTF-8 typed bytes, <=30 s grant expiry and <=5 s approval wait within the frame's
  freshness/window. Failures count after admission; no retries/replay or automatic
  limit increase. These limits preserve CU-01's at most two screenshots, 30 s
  observation window and one-frame retention; they do not enlarge observe grants.
  The earlier exploratory 10-action/60 s plan is not the current CU-02 policy.
- Before transport admission check binding, expiry, cancellation, freshness,
  coordinates and all limits; atomically consume the action grant immediately
  before invoking transport. Pre-admission denial/expiry/cancellation/staleness
  invokes no transport. Waiting for approval must be cancellable and bounded.
  Approval content displays complete JSON-escaped arguments, identities, target,
  frame/dimensions and all limits, without terminal control sequences. No truncated
  text approval. Grants and frames are invocation-only and cannot restore/replay.
- Transport is an injected headless trait with ONLY an action method, no production
  implementation or credentials in this slice. An observed transport acknowledgement
  means input was acknowledged, not that a task succeeded or the desired UI changed.
  After admission, timeout/cancellation/error means unknown remote outcome; stop
  this controller for the run and discard the frame. Dropping a future does not
  prove remote termination. The trait must cooperate with drop; a transport that
  blocks synchronously or spawns detached work can violate local wait guarantees.
  Such implementations are not approved production transports. No retry even if
  a transport never actually sent bytes. Late replies cannot reauthorize a run.
- No image bytes, typed text, key strings, coordinates, target or arbitrary transport
  errors enter metadata traces. This slice registers no runtime tool, event variant,
  session/result envelope, provider image input or CLI capability. Required checks
  run independently of optional observability. Private v2/provenance, original
  history/results, complete exchange admission and cancellation remain unchanged.
  Future runtime integration must define the private exact-call result envelope
  and preserve incomplete-batch rules, without synthetic completion/continuation.

## Alternatives

1. Recommended: implement the bounded headless controller with deterministic mock
   evidence under this contract. Keep live transport, observation decode/runtime
   integration, image disclosure/persistence and live trial separately gated.
2. Design/fixtures only, leaving even the Rust controller for a later slice.
3. Build a live exec-backed action loop now: requires bypassing unresolved CU-01
   transport/runtime gates and exact internal command review; rejected.

## Compatibility and consequences

This is a new security/coordinate contract, not authorization for any live desktop.
Current official OpenAPI has no direct click/type/scroll/key endpoint. SDK 0.5.3
implements these operations through exec/xdotool; do not invent wire routes or
reuse a general exec grant as action permission. Public transport research is
evidence only, not approval of exact internal shell commands or a new dependency.
Host-supplied metadata and mock acknowledgement are not screenshot/live evidence.
Rust remains authoritative, CLI first-class, Meister primary and Astrix specialized.
No local OS sandbox, remote workers, Host/mobile networking, Phase 9, daemon,
PTY, desktop client or automatic action loop is introduced. No merge is authorized.

Cost of changing later: High for authority/freshness; Medium for local bounds/API.

## Decision required

Accept the separate per-action grant, native-frame freshness contract and fixed
limits above for a mocked headless Rust controller, keeping observation/runtime,
live transport/trial and image persistence/provider disclosure gated?

No acceptance is recorded yet; the CU-02 task request is not a resolution of these
specific security and coordinate semantics.
