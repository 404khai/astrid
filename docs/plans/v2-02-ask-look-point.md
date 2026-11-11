# V2-02 — Ask, look, point

Status: **Planned; blocked on [ADR 0027](../adr/0027-local-look-and-presence-tools.md)**
(Proposed). Branch `feat/ask-look-point` from main `6161d3c3` (V2-01, PR #29, merged
2026-11-11 at the maintainer's instruction "merge work on v2-02").

## Exit criteria (from the gameplan)

A typed ask leads to one granted frame, then the vision model, then
`point_at`/`sketch` on the right target. A grounding-suite baseline is
recorded.

## What exists

| Piece | Where | Reuse |
|---|---|---|
| Overlay, ink, subtitles, protocol v1 | `apps/presence-mac` (ADR 0026) | As is; it becomes the runtime's sidecar |
| Single-use grants, frame slot, expiry, PNG validation | `computer.rs`, `computer_png.rs` (ADR 0022/0024) | Generalize the target from an InstaBox machine to local display *n* |
| Per-frame disclosure to exact image models | `openai.rs`, `visual_upload.rs` (ADR 0024/0025) | As is |
| Native-coordinate and freshness model | `desktop.rs` (ADR 0023) | Frame-bound coordinates for presentation tools |
| Runtime and model tool loop, approvals, events | `runtime.rs`, `tools.rs`, `permissions.rs` | New tools and a look approval |

## Slices (after ADR 0027 is answered; option 1 assumed)

1. **LP-01 Sidecar seam.** The runtime spawns `astrid-presence --stdin`,
   writes protocol v1 lines and reads `ready`/`displays_changed`/`error`.
   The host's lifetime is tied to the run, with cancellation and cleanup.
   Tests use a fake host binary.
2. **LP-02 Frame message (protocol v1, additive).**
   - Runtime → host: `capture {request_id, display}`.
   - Host → runtime: `frame {request_id, display, width, height, png_bytes}`,
     followed by exactly `png_bytes` raw bytes on stdout.
   - Bounded by ADR 0022 limits, then validated by the existing decoder.
   - The host has a `--capture-source synthetic` mode for tests.
3. **LP-03 Host capture.** ScreenCaptureKit at 1× point resolution, excluding
   the host's own windows, for one display. A script builds a bundle
   (`scripts/bundle.sh` → `Astrid Presence.app`) that owns the Screen
   Recording grant. A clear error explains how to grant it.
4. **LP-04 Look grant.** One combined single-use approval per frame, covering
   capture and disclosure, at most two per run. It is generalized from the
   InstaBox observe grant with the local display as target. Every limit,
   expiry and history/trace exclusion is unchanged.
5. **LP-05 Presentation tools.** `point_at`, `sketch` and `say` are validated
   against the current frame's display bounds, limited to 24 per run, and
   rendered through the sidecar.
6. **LP-06 CLI.** `astrid run --look [--display n] "<ask>"` runs one-shot in
   the foreground, separate from the InstaBox `--computer-use` path.
7. **LP-07 Grounding suite.** About 30 synthetic and real-app screenshots with
   labeled target boxes. It measures the hit rate of `point_at`, ask-to-first-point
   latency and cost. Following Phase 6 methodology, it runs with mock inference
   first. The live baseline needs separate trial approval.

## Out of scope

Voice, figures, walkthrough re-looks or watching grants (D2 b), input injection,
a resident or daemon mode, timers (D5), phone support and InstaBox changes.
