# Visual upload and response deadlines — implementation review

Date: 2026-10-23. Branch `feat/foreground-computer-use`, same preserved checkout,
no new branch/worktree. The maintainer instructed “Switch back to the branch for
this session. I accept option 1”. [ADR 0025](../adr/0025-image-upload-response-deadlines.md)
is Accepted; contract commit `fb332b86` precedes dependent implementation. PR #26
remains the integration PR against main, no merge authorized.

## Implemented boundary

`OpenAiProvider::generate_visual` now has two supervised phases. Preparation,
authentication, exact disclosure-grant consumption, upload-source consumption and
valid SSE headers must all fit within the original frame expiry, ten seconds from
capture admission. The application JSON/base64 Value is explicitly dropped before
sending. `visual_upload.rs` owns the bounded serialized body in one shared pending
slot; the source checks cancellation/expiry at polling time, yields it once to the
HTTP transport, and marks it consumed. Content-Length is the exact serialized body
length, retaining existing JSON accounting and wire framing. No new dependency.

Valid headers alone cannot admit a longer wait: pending bytes or expired/cancelled
source state reject admission. Successful source consumption plus valid headers
releases the canonical frame and closes/drops application upload ownership before
signalling the response phase. A drop guard clears pending source bytes even when
the transport retains its source. An image drop guard also covers cancellation or
dropping the provider future. Invalid destinations reject before authentication.

The response phase uses an immutable deadline sixty seconds from POST admission,
including header latency. It never resets on headers, text or streaming chunks.
Cancellation, either deadline or invalid/incomplete protocol leaves an error and
actual partial text, not a fabricated ModelResponse/tool execution/continuation.
Only the existing valid response.completed event establishes completion/usage.
Transport/TLS/kernel/provider copies already handed off cannot be recalled or
forensically erased; source consumption does not prove server receipt or effects.

Five-second action freshness, original thirty-second operation windows, grants,
counts, pixels, bytes and context allowances are unchanged. A slow visual response
cannot make its old receipt actionable. No image Message/session migration,
persistence, replay, silent recapture, retry, limit growth or worker installation.
The approval description names the unchanged image/upload deadline and sixty-second
response allowance. Ordinary text requests retain their existing behavior.

## Deterministic evidence

| Boundary | Evidence |
| --- | --- |
| Useful text then completion | Real loopback HTTP adapter streams text, frame is already released at first delta, terminal completion arrives after simulated 11 s; response valid only after terminal event |
| Exact context/history | Prepared image snapshot equals measured snapshot; returned original response serialization has no copied input image; existing full v2/context/replay regressions pass |
| Upload bounds | Authentication or headers stalled until original ten-second expiry cannot use response allowance; image is released, no retry |
| Body ownership | Early headers with unconsumed source reject; drop clears pending payload while a transport source remains; expired/cancelled polling cannot yield pending bytes |
| Response bound | Missing terminal completion stops at fixed sixty-second attempt deadline with actual partial text and no synthetic success |
| Header latency | Deliberate eight-second header latency counts toward the same attempt deadline; it cannot create a fresh sixty-second allowance |
| Cancellation/drop | Cancel after upload or drop during header wait clears frame/source and leaves uncertainty; no new POST |
| Privacy/destination | Wrong destination rejects before auth; HTTP/content-type rejections omit private error bodies, no image payload retained in returned response |
| Action/authority | Slow vision leaves original receipt stale; no machine call, refresh or grant reuse; existing permissions/cancellation/trace tests remain |

Executed:

- `cargo test --locked --offline -- --test-threads=1`: **362 passed, 0 failed,
  1 existing environment-specific overhead test ignored**.
- Final `cargo test --locked --offline --lib visual -- --test-threads=1`:
  **12 passed**. This includes one extra header-latency regression added after the
  full run; no production code changed after that run.
- `cargo clippy --locked --offline --all-targets -- -D warnings`, fmt, whitespace,
  scoped links and evidence JSON checked. Python was not rerun for Rust-only code.
- [Machine-readable evidence](../evaluations/visual-response-deadlines-2026-10-23.json).

No authenticated API, live screenshot/input or provider inference was performed.
Past user-supplied streamed descriptions remain partial live evidence, not this
mock validation and not a successful end-to-end live completion. Exact live trials
remain separately scoped operations. No desktop experiment implementation was
resumed; its branch, committed source and untracked build/user files are preserved.

## Remaining limitations

Ten-second human approval/upload can still expire, and five-second action freshness
is still experimental. This amendment solves response streaming lifetime only.
The source/transport boundary is observable application ownership, not proof of
remote processing or erasure. Production account/network behavior remains unverified
by these mocks. Model image-only usage remains unavailable unless reported; no
provider metrics, UI effects or task completion are invented.
