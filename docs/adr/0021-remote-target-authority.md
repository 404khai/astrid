# ADR 0021 — Foreground exact-target remote execution authority

Date: 2026-10-21
Status: Accepted — 2026-10-21
Implementation: Authorized mocked foreground slice, after this contract.
Decision evidence: Maintainer explicitly accepted ADR 0021 with the clarifications
below on 2026-10-21, then instructed implementation.
Related: ADRs 0004, 0008, 0009, 0012, 0016, 0018; RE-01 in
[the active plan](../plans/remote-mobile-cli.md).

## Context and constraints

The maintainer authorized one mocked `instabox_exec` runtime tool, after RC-01–03,
with an exact existing target and a separate bound grant. Local account shell
permissions are broad but cannot implicitly grant this typed remote capability.
This is a foreground local-host contract, not authenticated Host/mobile trust.
Meister remains primary, Astrix specialized; Astrid remains the Rust harness.

## Options and recommendation

Recommend an ephemeral per-call approval broker and single-use grant, separate
from all local presets. An alternative is reusable per-run remote shell approval;
that reduces prompts but authorizes commands the caller has not reviewed and is
outside the requested exact-argument boundary. No remote tool is the fail-closed
state when the host has not explicitly configured a broker, principal and target.

## Concrete contract

- A host supplies one immutable exact machine ID, an opaque HTTP executor holding
  credentials, and its invoking local principal. The principal is a host assertion
  for this local-account boundary, never model text or a remote identity scheme.
  No key loading, Clap, environment or dotenv access in the library. No model
  target/credential fields, list selection or other remote capability.
- Only `command`, absolute `cwd`, and integer `timeout_seconds` (1–600) are model
  arguments. Reject unknown fields. Canonical arguments are the validated typed
  tuple, not raw JSON formatting. Preserve command and cwd bytes exactly; do not
  shell-normalize, resolve remote paths, trim or rewrite the reviewed command.
- The runtime supplies run ID, runtime tool-call ID and provider call ID. Approval
  binds principal, those identities, exact target and canonical tuple. A host
  broker returns either denial or a non-serializable, single-use grant with a
  monotonic expiry. Check binding, expiry and cancellation after approval, before
  metadata GET, and immediately before POST. An expired/changed/denied/cancelled
  request sends no POST; no authenticated metadata request before approval.
  Atomically consume the grant immediately before POST admission, following final
  identity-binding, expiry and cancellation checks. Concurrent attempts cannot
  both consume it. A consumed grant cannot authorize another attempt after timeout,
  transport failure, cancellation or uncertain remote outcome; no retry or replay.
- Every local ask/auto/unbound preset, a resumed session and all read-only scheduler
  workers lack remote authority. The optional foreground broker is explicitly
  installed by the host; workers continue using only native read tools. Existing
  broad shell authority can independently access installed tools or credentials.
  This protects Astrid's typed remote-tool boundary, not the local OS account;
  it is not system-wide prevention of remote execution or a sandbox.
- Approval displays exact target/command/cwd/timeout and possible auto-wake. The
  sole metadata preflight reads that exact target, validates exact ID and running
  state, and retains no account inventory, names, labels or arbitrary server text.
  Running is a point-in-time observation, not a no-wake lock: a provider pause
  after GET can cause POST to wake the machine. The grant explicitly includes
  that race. No explicit lifecycle call or automatic retry.
- Private tool results retain target, run/call attribution, elapsed duration,
  separate stdout/stderr, actual exit status only when observed, provider timeout,
  truncation, dispatch status and remote outcome. Failure before dispatch is
  explicitly not dispatched. Cancellation after dispatch retains unknown outcome
  in the existing cancellation event and leaves the batch incomplete; no invented
  ToolResult, synthetic completion, termination claim or automatic continuation.
  After POST admission, local deadline, transport/status failure, oversized or
  malformed response remains unknown. Nonzero exit/provider timeout preserves the
  observed response; neither is task success.
- Public in-memory lifecycle events may carry reviewed arguments/results under
  the existing content-bearing event contract. Persistent metadata traces omit
  exact machine ID, principal, command, cwd, output, arbitrary error text and
  credentials. They retain ordinary run/model/tool parent IDs, fixed tool name
  and categorical lifecycle only. Exact target attribution is available in the
  private result/approval, not persisted traces. This preserves ADR 0009's existing
  schema/allowlist without extending stored target-identification semantics.
  Persistent execution-target attribution may require a separate privacy/schema
  decision for remote workers, multiple machines or background recovery. Do not
  implement that migration in RE-01.
- Optional observability off keeps required approval/cancellation/admission events
  but collects no added telemetry or traces. No session schema change, persisted
  grant, budget increase, worker grant, background job or reconnect semantics.
- Reuse the bounded HTTP executor: fixed production origin, no redirects/proxy/
  retries, 5s connect timeout, 15s whole metadata wait, requested timeout plus 5s
  whole POST/body wait and 256 KiB body cap. Test seams stay private. A conservative
  unknown begins at POST admission even if a transport failure prevents wire send.

## Acceptance and limits

Deterministic loopback HTTP and mocked runtime/provider tests cover binding,
expiry, single consumption, all local presets without remote authority, approval
cancellation, preflight/pre-POST cancellation, dispatched cancellation, exact wire
payload, deadlines/body caps/malformed/status/nonzero/provider timeout, no retries,
parent/target attribution and credential/trace privacy. No authenticated public
API, live model, VM operation, provisioning, lifecycle, transfer, screenshot,
Host/mobile networking, Phase 9, daemon, PTY or desktop work is authorized.

Cost of changing later: High for authority; Medium for optional adapter API.
Resolution: Accepted with atomic consumption/concurrency, local OS scope and future
auditability clarifications above. No live machine execution or provisioning grant.
