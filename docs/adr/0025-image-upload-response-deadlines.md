# ADR 0025 — Separate image upload and visual response deadlines

Date: 2026-10-21
Status: Accepted — 2026-10-23; maintainer explicitly accepted option 1
Related: accepted ADRs 0008/0009, 0016, 0021–0024;
[foreground integration](../milestones/foreground-computer-use.md).

## Context

The maintainer supplied an observation-only trial transcript: after separate image
approval, the model streamed a screen description, then the CLI reported
`visual image deadline; remote outcome unknown` before response.completed.
This is user-supplied partial live evidence of image interpretation, not a complete
model exchange, independent assistant trial or proof of desktop action execution.
No screenshot or raw screen description is retained in this decision record.

`OpenAiProvider::generate_visual` currently selects the frame's original expiry
against the entire generate_input future, including the SSE response. The frame
expires ten seconds after capture admission, not ten seconds after upload or the
approval prompt. A correctly approved request can therefore stream useful text
and still be interrupted. Text deltas alone cannot establish completion or usage.

Accepted ADR 0024 requires transient encoding/network buffers within frame expiry
and says cancellation/expiry ends the wait. Removing the outer timeout without
resolving buffer ownership would both conflict with that contract and risk keeping
the JSON/base64 request alive throughout a longer response. Simply trusting visible
text as task completion would violate the original-result/complete-exchange rules.

## Accepted amendment to ADR 0024

1. Keep the original ten-second deadline for the admitted native image, human
   disclosure approval, authentication/preparation and request-body upload/response
   header admission. No deadline resets at prompt display, yes, first byte or
   transport retry. A late approval/upload fails closed; no image is recaptured.
2. After the exact single-use disclosure grant is consumed, permit a separate
   fixed **60-second total visual provider-attempt deadline from POST admission**.
   This is an explicit new response-wait allowance, not adaptive budget growth.
   It includes upload time; streaming chunks never reset it. The already uploaded
   request can complete after the source frame expires, subject to this deadline.
3. Do not transition to the longer response wait until the bounded outbound body
   source has been consumed and response headers have arrived within original
   frame expiry. Validate successful status/SSE content type. An early response
   with an undrained body fails closed; headers alone do not prove upload. Body
   consumption/headers are transport observations, not proof of model processing
   or recall/erasure at the provider. Mock validation must exercise this seam.
4. Release the canonical frame and all application-owned image, base64, JSON and
   request-body copies before entering the longer response wait, and always by
   original frame expiry. Explicitly scope their ownership/drop boundaries; do not
   leave a body Value captured across SSE reading. Use a bounded, observable body
   source instead of treating request-builder serialization as upload completion.
   Cancel/drop the upload source on expiry/cancellation. Transport/TLS/kernel or
   provider copies already handed off cannot be forensically erased or recalled;
   do not claim otherwise. If the adapter cannot enforce its application-buffer
   lifetime or establish the upload boundary, fail closed within ten seconds.
5. During the longer wait retain only the response/decoder/assembly and ordinary
   text/tool metadata needed for the existing checked protocol. Images/hashes or
   upload references never enter saved Message/session v2, summaries, continuation
   or metadata traces. No image reconstruction/replay, provider upload cache,
   Files API, remote image URL or silent model/destination substitution.
6. Preserve cancellation, unknown outcomes and no retry. On cancellation or either
   deadline, close the local wait, keep actual streamed text as interrupted output,
   and admit no completion, usage, tool execution or continuation without the
   existing valid provider completion event. No synthetic successful ModelResponse
   from a readable description; missing provider metrics remain unavailable.
7. Keep all observation, pixel/byte/count/context limits and independent grants.
   The new 60-second response wait does not extend the original 30-second computer
   operation windows, the five-second action freshness, grant expiry or receipt
   validity. A model action on an old frame still fails stale; no automatic refresh,
   reused approval, worker, background loop, new persistence or live-trial authority.
   Approval UI names the new response deadline and unchanged image deadline.

This amendment applies only to the explicitly selected supported visual OpenAI
request path. Ordinary text-provider behavior is unchanged. It does not resolve
the separately experimental human-approval/five-second action UX.

## Alternatives

1. Recommended: accept the two-stage contract above and implement the narrow
   transport/body-lifetime separation with deterministic mocked validation.
2. Keep the accepted ten-second whole-request behavior. Preserve interruption
   even after a useful description; no completed observation-only result promised.
3. Extend image retention together with the whole response timeout. This keeps
   screen buffers longer and broadens privacy beyond what this failure requires;
   not recommended.

## Required implementation and evidence after acceptance

- Smallest upload/send helper and response-reader boundary; explicit source/drop
  accounting and an immutable 60-second overall attempt deadline.
- Clock-controlled mock streams: successful completion after ten seconds, no
  retention of application image buffers during response wait, no completion
  before the provider's terminal event, cancellation, upload/approval expiry,
  early headers/undrained source, response timeout and unknown/no retry.
- Preserve exact context measurement/admission, original complete exchanges,
  private v2/history, trace privacy and observability-off. No image token fabrication.
- Keep stale-action rejection after slow vision, with no recapture/extra budget.
- Existing runtime/provider/CLI regressions and strict fmt/Clippy/whitespace checks.
- No authenticated call, screenshot, provider inference or desktop input during
  development. A later exact approved live trial remains a separate operation.

## Recommendation and decision

Recommend option 1. It gives an already-approved/uploaded request time to finish
without making image retention follow model response latency. Cost of changing
later: High for buffer/privacy semantics; Medium for the explicit 60-second limit.

Decision evidence: on 2026-10-23 the maintainer instructed “Switch back to the
branch for this session. I accept option 1”. This explicitly accepts the narrow
amendment and authorizes implementation and deterministic mock validation, retaining
every other gate. Acceptance is committed before dependent provider implementation.
No live trial, image persistence, grant expansion, successor work or merge is implied.
