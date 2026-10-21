# Foreground InstaBox tool adapter — 2026-10-21

RE-01 implements accepted [ADR 0021](../adr/0021-remote-target-authority.md) on
`feat/instabox-tool-adapter`, from main `cc0fa964`, in the existing checkout.
The maintainer separately accepted the exact authority proposal and clarified
atomic consumption, the local OS boundary and future target-audit privacy/schema
work before dependent code. This PR must remain unmerged.

## Implemented guarantees

`src/remote.rs` exposes a small host installation surface: `InstaboxTool::new`,
`RemoteApproval`, `RemoteGrant::approve` and `Tools::with_instabox`. The host
supplies its local principal, immutable exact existing machine ID and an opaque
HTTP client. Only validated command, absolute cwd and bounded timeout are model
arguments. JSON field order and formatting do not affect binding; command/cwd
bytes are preserved exactly. Target selection, keys, principal, IDs, stdin and env
cannot be injected as extra arguments. Credentials are neither serializable nor
Debug-printable through the remote client/tool/grant boundary.

The runtime builds the approval request from its actual run ID, runtime tool-call
ID, provider call ID, host principal/target and canonical argument tuple. It emits
`RemotePermissionRequired` separately from local policy evaluation, then the
existing approval lifecycle. The host broker sees the full private description,
including the possible auto-wake race. Local ask/auto/unbound and local approval
handlers never supply remote authority; no broker means denial, and a read-only
worker policy also suppresses advertisement and dispatch. No scheduler installation
or worker grant changes. Existing broad shell authority may independently access
installed programs or credentials: this typed boundary is not an OS sandbox.

After approval, the grant is rechecked before any authenticated metadata request.
The executor fetches only the exact selected ID and requires running state. Before
POST admission it checks identity/arguments, monotonic expiry and cancellation,
then atomically consumes the grant without an intervening await. Only one compare-
exchange can succeed, even for concurrent attempts. Consumed grants never authorize
another attempt after success, timeout, cancellation, transport failure or unknown
outcome. A fresh deliberately approved call is separate authority; the executor
has no retry/replay loop.

`src/instabox.rs` is the extracted bounded HTTP executor; Clap/environment/dotenv
loading and related tests stay in `src/instabox_cli.rs`. Wire fields remain `cmd`,
`cwd`, `timeoutSeconds`. Fixed production origin, no redirects/proxy/retries,
5s connect bound, 15s metadata wait, provider timeout plus 5s POST/body wait and
256 KiB bodies remain intact. The extracted client explicitly sets
`reqwest::retry::never()`: locked reqwest 0.12.28 otherwise permits safe protocol-
NACK retries by default ([versioned ClientBuilder documentation](https://docs.rs/reqwest/0.12.28/reqwest/struct.ClientBuilder.html#method.retry)).
This removes reliance on a library default for the no-retry guarantee. Exact running preflight is not a lock: a subsequent
provider pause can cause POST auto-wake under the disclosed approval.

Reports preserve actual separate stdout/stderr, exit status only when observed,
provider timeout, truncation, elapsed time and private run/call/target attribution.
Pre-POST failures are not dispatched. After POST admission a local deadline,
transport/status failure or malformed/oversized response is unknown, never proof
of termination or permission to retry. Dispatched cancellation carries that unknown
report in `ToolCallCancelled` and leaves the original batch incomplete; no fabricated
ToolResult. Follow-up rejects it without changing original history.

No new session outcome variant or session v2 format is introduced. The existing
Success data envelope transports the report, with an explicit `successful` field;
`ToolResult::is_error` recognizes false/missing success on this tool, and the runtime
emits ToolCallFailed for nonzero/unknown/not-dispatched reports. Provider timeout
and post-admission local deadline use the existing TimedOut envelope; preflight
failures remain not-dispatched reports. Context summaries and terminal
summaries retain this distinction. Tool failure does not itself claim an entire
user task failed; subsequent inference can inspect the actual report.

OpenAI advertises the tool only when the runtime's current host installation is
present and eligible, using transient request options. Measurement/admission uses
the same schema, including RC-03's accumulated exchange checks. The model sees
no credential/configuration object. Restored sessions do not restore a broker or
grant. Original private history, provenance and continuation are unchanged.

Persistent traces add only the fixed tool/lifecycle name through the existing
allowlist/framing. Exact machine ID/principal, commands, paths, outputs, arbitrary
errors and credentials remain omitted. Attribution is private approval/result/event
data only. Future persistent target auditability for multiple machines, remote
workers or background recovery requires its own privacy/schema decision; no
migration is implemented. Observability off retains mandatory core events and
admission but writes no traces or added telemetry.

## Deterministic validation

Final checks: **302 Rust passed, 0 failed, 1 ignored; 2 Python passed**. Format,
strict all-target Clippy and whitespace passed. The ignored case remains the
optional recording-overhead experiment. An initial parallel run failed the existing
`observation_never_runs_fsmonitor_or_changes_index` dirty-entry assertion; its
isolated rerun and two complete serialized runs passed without Git-observer changes.
Timing is a possible explanation, not an established root cause. The final complete
run includes explicit no-retry configuration. Afterward a test-only binding-category
assertion was tightened and all thirteen remote tests passed again; Clippy/fmt also
passed. [Pinned source hashes/results](../evaluations/instabox-tool-adapter-2026-10-21.json).

Thirteen new remote boundary tests and one schema test cover:

- canonical tuple and every identity/argument binding, invalid/extraneous fields;
- atomic consumption under twelve simultaneous threads and two concurrent HTTP
  attempts (two approved preflights, exactly one POST);
- all local presets with no broker or denied remote grant, hard read-only ceiling,
  no remote inheritance on resume and no local permission-handler invocation;
- pending approval with zero authenticated requests, denial/expiry/changed grant,
  approval cancellation, cancellation/expiry after metadata and before POST;
- exact wire payload and call/parent/target correlation, actual separate output,
  nonzero exit, provider timeout and output truncation;
- metadata/body deadlines, transport closure, malformed/missing/oversized bodies,
  HTTP errors/redirect refusal, no replay with consumed grants or retry requests;
- dispatched cancellation preserving unknown/no result/incomplete-batch rejection;
- credential sentinels absent from model inputs/session/results, private target/
  command/output/path/metadata sentinels absent from trace JSON, observability off;
- disabled-by-default schema advertisement with only the three model arguments.

The existing synthetic executor/key-loading/CLI preflight suites also remain
required. No live model, authenticated public API or VM operation was performed.

## Remaining limits and next slice

This is a mocked foreground library integration, installable by a trusted host.
The ordinary CLI agent commands do not install a remote target/broker or load an
InstaBox key implicitly. Existing standalone `astrid instabox` discovery/explicit
exec stays supported. No interactive/session-resume configuration persistence,
remote workers, background execution, computer use, screenshot, mobile networking,
Phase 9, daemon, PTY or desktop work is added.

Next proposed vertical slice is RE-02: after separate exact live authorization,
connect a foreground CLI approval broker to a user-designated isolated existing
machine and inspect harmless command attribution, denial, timeout/cancellation
and the disclosed auto-wake race. Reconfirm the current public API before that
trial. No trial, provisioning or automatic phase advancement follows from RE-01.

## Publication

Implementation `5ec7fdd6` is published in
[PR #22](https://github.com/404khai/astrid/pull/22), targeting main and left
unmerged. Publication documentation changes no tested source hashes. The original
unrelated `.gitignore`, historical checkpoint modifications, recovery files and
untracked user assets remain outside the PR.
