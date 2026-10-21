# ADR 0024 — Foreground computer use, real observations and ephemeral vision

Date: 2026-10-21
Status: Accepted — 2026-10-21; maintainer explicitly selected option 1
Related: accepted ADRs 0008/0009, 0016, 0021–0023.

## Context and remaining work

The maintainer requested: “Tell me what’s left to get the actual CU working, and
implement it.” CU-01 has Python observation fixtures; CU-02 has a Rust action
controller with trusted metadata and mock transport. Actual use still needs:

1. A bounded InstaBox screenshot/status and typed input transport.
2. A maintained PNG decoder with bounded worker lifetime, native geometry and
   cleanup; real receipts derived only from successfully admitted observations.
3. Foreground CLI/runtime wiring with separate observe/action approvals.
4. Explicit model image disclosure and request accounting without image history
   persistence or silent replay.
5. An exact existing-machine live trial, separately authorized after development.

New implementation authorization supersedes the CU-02 stop restriction for this
integration task; it does not by itself resolve image disclosure, persisted-state
or freshness semantics. No target or live scope has been supplied. PR #25 remains
open; work is on `feat/foreground-computer-use`, stacked on `4d10f91f` in the same
preserved checkout, no worktree. Do not merge either PR.

## Accepted integration contract

- Build the real adapters and CLI/runtime path, validate against loopback HTTP,
  synthetic PNGs, mock inference and isolated subprocess fixtures. Development
  must not read credentials/private sessions or contact authenticated APIs. This
  explicitly permits production adapter *code* before live authorization, amending
  ADR 0022's “before future live code” gate; actual live use still requires exact
  existing-target/trial approval and per-call grants. No lifecycle, ports, viewers,
  provisioning, secret transfer, desktop client, workers or background service.
- Start with one-shot foreground `astrid run --computer-use` plus invocation-only
  `--instabox-machine <exact-ID>`. No default target, discovery, saved capability,
  resume, scheduler installation or interactive-mode authority. Use the existing
  Rust runtime/model-tool loop, with human approval for every observe/action.
  No independently running autonomous action loop or automatic retry/recapture.
- Preserve all existing finite limits: four observe operations, two screenshots,
  1 MiB PNG/frame, 2 MiB received PNG/run, native dimension <=2560 and <=4,096,000
  pixels, one memory-only frame for <=10 s, two actions/receipt installs, 30 s
  original run windows, <=5 s frame age and action approval, 3 s action transport,
  10 s whole observation. No automatic budget growth. Five seconds remains an
  experimental, possibly inconvenient human/model latency limit; reject stale
  actions and require separately approved new capture AND new action approval.
  Do not claim this solves the final production approval UX.
- Observation authority binds host principal/run/runtime-call/provider-call/exact
  target/canonical operation and fixed protocol revision. Consume atomically before
  the first authenticated request. Exact-target running-state preflight is covered
  by that reviewed operation; no inventory read. Native screenshot capture uses
  fixed audited commands on display `:1`, cwd `/data/home`, provider timeout 2 s,
  capped PNG stdout (base64 over exec), no guest image file or installed package.
  Missing capture tools fail clearly; never install/start the desktop. Status uses
  documented `POST /v1/machines/{id}/desktop` with `{"action":"status"}` and retains
  only installed/running booleans. No invented screenshot/action REST endpoint.
- Typed action mapping uses only validated click/type/scroll/key arguments and
  published exec/stdin support. Exact reviewed command/cwd/stdin/timeout are shown
  in approval alongside the typed operation; text is stdin, never shell source.
  No arbitrary command/env/path accepted from model input. No request retry,
  redirects/proxy, takeover or viewer link. Auto-wake and partial effects remain
  disclosed; post-admission uncertainty stops the run, not just the local wait.
- Real actionable receipts are minted only after bounded PNG validation succeeds;
  they bind the run/target, admission time, native geometry and current frame.
  Keep one authoritative controller per run. Metadata replacement cannot reset
  screenshot, operation, action or time budgets. No mock receipt in the real path.
  Use a killable/reaped local decode helper with piped bounded data, no image file,
  no inherited credentials and a 2 s deadline within the observation deadline.
  Validate framing/CRC/allocation bounds, reject animation and unsupported content,
  strip nonvisual metadata by canonical re-encoding; no OS sandbox claim.

## Accepted image disclosure and history amendment

- Default remains no image disclosure. For each frame, obtain a distinct
  single-use `image.disclose` grant before provider admission, bound to principal,
  run, model-call, source frame/target, exact canonical image and selected provider,
  model and fixed `https://api.openai.com/v1/responses` destination. Declining
  never sends an image or fabricates an image-enabled model response.
- Only explicitly documented exact image-capable model IDs are allowed initially:
  `gpt-5.6-sol`, `gpt-5.6-terra`, `gpt-5.6-luna`; no prefix inference or silent model
  substitution/routing. Use `input_image` data URL with `detail: original`,
  `store:false`, one image in exactly one model request, at most two disclosures
  per run. No Files API, remote image URL, upload cache or provider image replay.
  Account availability is unknown until separately authorized live validation.
- Preserve native coordinates with an identity image transform. This explicitly
  amends ADR 0022's future downscale-before-inference clause for this bounded
  native-image path: one native frame, no full-resolution image history. The
  documented model limits preserve these native dimensions with original detail.
  Oversized requests fail; do not resize silently, change coordinates or raise
  request/context allowances. Crop/DPI/other transforms remain out of scope.
- Compute the documented patch estimate `ceil(ceil(w/32)*ceil(h/32)*1.2)` as an
  estimate, plus one token rounding margin for admission. Count image tokens
  separately from text; never bytes/4 for an image. Enforce the existing request
  byte and input allowances on text plus image estimate, with no auto increase.
  Actual image-only provider usage/cost stays unavailable unless reported; provider
  aggregate input usage includes images when present. Unverified model rejects
  before authentication/disclosure.
- Image bytes/base64, hashes and references are never serialized to session v2,
  summaries, private continuation or metadata traces. The optional image is a
  request-local attachment to the next invocation, not a persisted Message variant.
  Retain transient encoding/network buffers only within the same frame expiry;
  cancellation/expiry ends the wait and releases local buffers. This is not
  forensic erasure or the ability to recall bytes already sent to a provider.
- Private text tool results may retain exact target/run/call/frame/native geometry,
  categorical observation/action outcomes and observed acknowledgement. That is
  the concrete non-image result envelope amending ADR 0022's runtime envelope gate;
  no image/session schema migration. Later history contains that text metadata and
  actual model outputs, never the old screenshot. Never reconstruct omitted images
  or private reasoning. Preserve original v2 history/provenance and complete tool
  exchanges; incomplete cancelled batches still cannot continue. Reopened frames
  cannot become actionable. Trace allowlists and observability-off remain unchanged.
- `store:false` is not a zero-retention guarantee at the provider: its data-control
  policies and account settings still apply. Approval names the provider and model,
  warns about screen-sensitive content and makes no screenshot redaction promise.
  Ordinary model outputs may themselves describe sensitive screen content under
  the existing private transcript policy; no promise that screenshots leave no
  derived text. No new persistent image opt-in preference or retained grant.

## Alternatives and recommendation

1. Recommended: accept this complete foreground integration contract, including
   per-frame explicitly approved ephemeral image disclosure, real adapters and
   deterministic validation; keep all actual live operations separately gated.
2. Implement observation/action transport and a human-driven CLI only, keeping
   zero model image disclosure. This can test inputs but cannot give the agent
   visual computer use; agent vision would need a later accepted amendment.

Cost of changing later: High (privacy, disclosure, history and runtime authority).

## Accepted decision question

Accept option 1, including the explicit native-image/disclosure and runtime
envelope amendments above, or choose human-driven integration with no model vision?

Decision evidence: After reviewing this concrete privacy/runtime proposal, the
maintainer explicitly answered “Accept ADR 0024 option 1” on 2026-10-21. This
authorizes implementation and mock validation, not an actual live trial.

## Official evidence

- [OpenAI image input and model sizing/accounting](https://developers.openai.com/api/docs/guides/images-vision)
  documents data URLs, original detail for exact named models and patch estimates.
- [OpenAI data controls](https://developers.openai.com/api/docs/guides/your-data)
  distinguishes application-state storage from provider data retention.
- [InstaBox OpenAPI](https://api.instabox.sh/v1/openapi.json) documents exec cmd/cwd/
  stdin/timeoutSeconds and desktop status; no direct screenshot/action routes.
- [Published InstaBox SDK](https://github.com/InsForge/instabox-sdk) supplies
  command-based capture/input capability evidence, not a live authorization grant.
