# ADR 0022 — Separate screenshot observation and ephemeral image handling

Date: 2026-10-21
Status: Accepted — 2026-10-21, with the explicit gates below
Implementation: Synthetic fixture specification only; no runtime capability installed.
Related: ADRs 0004, 0006–0009, 0012, 0016, 0018, 0020, 0021;
[CU-01 milestone](../milestones/screenshot-observation.md),
[public API audit](../reviews/screenshot-observation-2026-10-21.md).

## Context

CU-01 authorizes design and fixture/mock work, not live screenshots, image
persistence, provider disclosure or a computer-use loop. Existing Rust messages,
context estimation, session v2 and provider replay are text-only. Observation
changes permission semantics; durable image references would change persisted
state. Neither can be inferred from RE-01 shell authority or local presets.

Public InstaBox SDK 0.5.3 implements screenshots using several exec/file commands
and a temporary guest PNG. The current OpenAPI has desktop status but no direct
screenshot endpoint. A Rust screenshot transport and its exact internal commands,
guest artifact cleanup and uncertain outcomes therefore remain a separate gate.
Do not manufacture a screenshot route or expose arbitrary commands under observe.

## Accepted decision

1. Introduce a separate host-owned `gui.observe` broker for exactly one `status`
   OR `screenshot` operation per approval. Bind principal, run ID, runtime call ID,
   provider call ID, exact host-selected machine and canonical arguments. Arguments
   are operation plus fixed policy revision `cu01-ephemeral-v1`; the revision binds
   every limit and destination below. Reject unknown fields/operations. Selection,
   local ask/auto/unbound, resume, workers and `instabox_exec` grants confer no
   observe authority. An observe grant cannot be used as an exec/action grant.
   Atomically consume a nonserializable single-use grant immediately before the
   first authenticated request, after binding/expiry/cancellation checks. One
   bounded internal operation may require audited subrequests, all within that
   same approved operation; never acquire general shell authority or retry/replay.
   Expiry is at most 30 s. No persisted grants, discovery or target defaults.
2. Approval must disclose full canonical JSON, exact target and identities, whole
   screen sensitivity, memory-only retention, no inference destination, finite
   limits, cancellation uncertainty, any guest temporary artifact and possible
   auto-wake. Before future live code, resolve transport and cleanup commands and
   obtain exact live scope authorization. Running preflight is point-in-time and
   cannot promise no wake. No install/start, takeover/handback, viewer link, VNC,
   watch stream, cursor, click/type/scroll/key/open, lifecycle or remote worker.
   A status operation may read only installed/running booleans; discard logs,
   action history, controller details and arbitrary server content. Unknown fields
   do not enter results. Dimensions come from validated image decoding, never an
   implicit geometry shell call. Status never silently captures an image.
3. Fixed initial limits: 1 MiB encoded PNG per frame; 2560 px per dimension and
   4,096,000 pixels; decoded RGBA at most 16,384,000 bytes; 10 s whole operation,
   5 s connect within it; 4 admitted operations per run (failures count), at most
   2 screenshots and 2 MiB received PNG traffic per run, 30 s observation window
   from the first admission. Stream and reject before allocating beyond limits;
   status body at most 16 KiB. No automatic budget growth. An individual
   screenshot receipt is a local monotonic interval, not a verified server capture
   timestamp. Reject cancelled, late, malformed, oversized or inconsistent data.
4. A future maintained PNG decoder must validate signature, chunk framing/CRC,
   dimensions and checked allocation before decompression; bound compressed input,
   decoder allocations, decompression output and CPU time. Reject animation,
   truncated/trailing streams, unsupported formats and unneeded metadata. Do not
   trust MIME, header dimensions alone or renamed files. A local decode worker
   must have a bounded lifetime; cooperative cancellation alone cannot stop a stuck
   decoder. The fixture decoder supports only synthetic RGB8/filter-0 PNG, and
   is explicitly unsuitable as the production decoder.
5. Keep at most ONE validated image in a run-owned ephemeral slot; replacement
   releases the prior image, failure/cancellation invalidates it, run end closes
   it. Retention ends at the earlier of 10 s since admission or run end. Expiry
   must release bytes even without a consumer (a future owner timer); all reads
   check expiry/cancellation. No exported byte handles, clones or durable image
   references. Explicit cleanup on all paths, including approval/transport/decode
   failure. Memory release is not guaranteed forensic erasure, crash-dump/swap
   exclusion or cleanup of unknown guest artifacts. Fixture tests use a fake
   clock and explicit expiry sweep; they do not prove an idle production timer.
6. Image bytes, thumbnails, hashes, OCR, paths and exact target/principal are
   absent from metadata traces and session v2. No base64 in ToolResult, summaries,
   error strings or private continuation. No new serialization/schema migration.
   Any future runtime result needs a concrete non-content observation envelope
   consistent with complete exchange admission and private exact-call attribution;
   do not delete or rewrite committed historical results. This ADR does not
   authorize installing that envelope. Privacy is minimization, not automatic
   screen redaction. Observability off still enforces permission, accounting,
   retention and cancellation but creates no telemetry or traces. No new persistent
   trace fields under ADR 0009; fixture accounting is local test state only.
7. Image context allowance is ZERO and image input is disabled in this slice.
   Account encoded/decoded bytes, pixels, operations and resident frames separately
   from existing text estimates. Provider image tokens, cost and model-window
   occupancy are unavailable, never bytes/4 or zero usage. A future separate
   disclosure decision must bind provider/model/destination, supported image
   format/transform, image count/byte/pixel limits, verified provider accounting
   and conservative request admission before any model transmission. Downscale
   before any authorized future inference, keep original-to-input transform, and
   retain at most two bounded derived inputs subject to total request allowance;
   no silently evicting a mandatory exchange to fit, no full-resolution history.
8. An observation becomes stale on replacement, failure, expiry, cancellation,
   run/target change or known desktop geometry change. It never establishes
   exclusive desktop state or action coordinates. No CU-02 action can consume it
   until a future freshness/coordinate contract is accepted. Cancellation before
   admission means not dispatched. After admission, timeout/cancel/transport or
   malformed response leaves remote execution/cleanup unknown, releases local
   images and never retries. Dropping a response is not proof of guest termination.
   Future runtime integration must use existing incomplete-batch cancellation
   semantics; no synthetic successful results or automatic continuation.

## Alternatives considered

- Persist private image artifacts/references and add provider vision now: useful
  replay, but requires image schema, deletion, disclosure and accounting decisions
  beyond the accepted text-only contracts.
- Reuse shell grants or assume screenshot REST payloads: defeats separate observe
  authority or relies on undocumented transport; rejected.
- Defer even synthetic fixtures: avoids prototyping but leaves the proposed bounds
  and privacy behavior without independent executable evidence.

## Consequences and compatibility

Accepted: the ephemeral, no-model-disclosure contract above, leaving
transport, production decoder, runtime integration, persistence and image-provider
disclosure gated. CU-01 can publish the accepted contract and fixture evidence while
those gates remain closed. This is not full runtime observation acceptance.
Rust harness identity, Meister primary/Astrix specialized, CLI support and existing
text history/provenance/permissions remain unchanged. No local OS sandbox claim.

Cost of changing later: High for authority/privacy; Medium for reversible limits.

## Accepted decision question

Accept this separate single-use observe authority with memory-only one-frame/10 s
retention and zero model image disclosure, while keeping live transport and all
image persistence/provider/schema changes gated?

Decision evidence: After reviewing this concrete proposal and its gates, the
maintainer explicitly answered “Accept ADR 0022 with its gates” on 2026-10-21.
This accepts the observation contract; it does not authorize live transport,
production decoding, runtime integration, image persistence or provider disclosure.
