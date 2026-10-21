# Runtime correctness → remote execution → mobile → terminal integration

Date: 2026-10-18. Audit baseline: `8dea6765`, existing `feat/tui-redesign` checkout.
This is the active scoped direction requested by the maintainer, layered on the
implemented foundations. Phase IDs and historical acceptance are unchanged.
Meister remains the primary assistant. Astrix are specialized identities, distinct
from models, workers, tasks, terminal panes and execution targets. Rust owns the
harness. CLI and mobile are clients; InstaBox is an execution target.

## Audit: implemented facts and gaps

| Area | Repository evidence | Consequence |
| --- | --- | --- |
| Context | `src/context.rs`, `tests/context_budget.rs`, ADRs 0006/0007/0010 | Historical grouping pins the prior transcript; fix selection, not token defaults. Latest multi-tool aggregate can still exceed admission. |
| Cancellation | `src/runtime.rs`; `tests/runtime.rs`, `tests/sessions.rs` | Completed exchanges survive; unmatched terminal batches reject follow-up. Model cancellation without such a batch can continue. Preserve both cases. |
| Persistence/routing | `src/session_store.rs`, ADR 0016; `tests/session_store.rs`, `tests/routing.rs` | V2 provenance supports conservative visible-history routing. Do not assume every legacy/one-shot session is resumable; validate actual replay mode, origins and completeness. |
| Usage | `src/observability.rs`, `src/console/context_meter.rs`, `src/main.rs` | Traces/stats retain known input/output/cached usage; console shows a local non-opaque bytes/4 estimate. RC-02 adds opt-in live provider usage and per-metric coverage; reserve remains planning headroom. |
| InstaBox | `src/instabox.rs`, `tests/instabox_cli.rs`, `tests/support/instabox_mock.rs`; [merge review](../reviews/instabox-completion-2026-10-18.md) | Standalone CLI GET discovery and explicit exact-machine exec exist. Agent tool integration does not. Adapter is binary-private and mixes command parsing, credentials and HTTP. |
| Tools/authority | `src/tools.rs`, `src/permissions.rs`, `src/runtime.rs`, `src/openai.rs` | Fixed tool definitions, read/write/execute policy and command/workspace approval request assume local tools. Remote execution needs explicit target authority; adding a tool name to local Execute is insufficient. |
| Scheduler | `src/scheduler.rs`, accepted ADRs 0017–0019; [review](../reviews/phase-8.md) | Process-local bounded DAG with fresh read-only workers; no remote worker grant, concurrent writes/shell, daemon or durable jobs. Terminal persistence cannot supply these guarantees. |
| Host | `src/runtime.rs`; paused M02/ADR 0014 | Headless runtime exists; authenticated network Host does not. Superseded Tauri IPC is not a cross-device trust contract. |
| Mobile | `mobile/src/App.tsx`, `NativeSheet.tsx`, `data.ts`, `mobile/README.md` | Existing Expo concept uses demo projects/threads and unconditional SwiftUI/SymbolView imports. No live runtime, permissions or network; Android compatibility is not established. README's Phase 4 marker is historical. |
| CLI | `src/console/*`, existing PTY rendering tests | Supported terminal client with event projection, approvals and session navigation; PTY tests are not a PTY multiplexer implementation. |

The earlier incident report diagnoses unfinished InstaBox code before merge. Its
HTTP field/default problems were subsequently fixed by PR #17; do not re-report
them as current defects. Current exec uses `cmd`, `cwd`, `timeoutSeconds` (1–600),
`/data/home`, exact-ID/running-state preflight, bounded response and waits, explicit
grant and no retry. Cancellation after admission reports unknown remote outcome;
it cannot prove remote termination. Existing local `unbound` shell authority can
invoke arbitrary programs; a remote policy must not claim to sandbox that authority.

## Dependency-ordered slices

Each row is a separate reviewable outcome. “Draft” means design prepared, not
implementation acceptance. Decisions changing security, persistence or remote
trust need their concrete ADR before dependent implementation.

| Ticket | Dependencies | Behavior / independently testable exit | Status |
| --- | --- | --- | --- |
| RC-01 | Accepted ADR 0020 | Oversized previous-submission follow-up fits unchanged allowance; no replay; retained originals, omission lineage and checked events | Done; [milestone/evidence](../milestones/runtime-recovery.md) |
| RC-02 | RC-01; metadata projection compatibility review | Normal CLI displays valid known input/output/cached counts, unknowns and aggregate coverage; budgets separately labeled | Done; [compatibility/review](../reviews/live-token-usage-2026-10-18.md) |
| RC-03 | RC-01/02 | Bound one multi-tool exchange before further costly work; test many individually legal large outputs, Unicode and latest-exchange overflow | Done; [RC-03 guard/review](../reviews/aggregate-tool-admission-2026-10-21.md) |
| RE-01 | RC-01/02/03; target-authority ADR | One foreground `instabox_exec` tool on an explicitly selected existing machine, mocked HTTP; deny remote access in every local preset absent separate grant | Authorized 2026-10-21; [ADR 0021](../adr/0021-remote-target-authority.md) decision pending; HTTP extraction validated separately |
| RE-02 | RE-01 | Authorized isolated live trial: exact selected running machine, harmless `printf`/`pwd`, denial/timeout/cancel and attribution inspected separately | Draft; exact live request including possible wake requires authorization |
| CU-01 | RC-03, RE-01; observation/data-retention decision | Bounded screenshot/status observation only, separate observe grant; fixture image retention, privacy and context accounting | Draft; no desktop action implied |
| CU-02 | CU-01; action grant, image input and coordinate contracts | One click/type/scroll/key action at a time with step/time/byte bounds, stale-frame rejection and no uncertain retry | Draft; isolated harmless desktop trial after explicit live scope |
| MO-01 | Existing mobile concept/reference | Shared RN Meister conversation shell, `.ios` SwiftUI adapters and Android components, mock streams/tool/approval states; both exports/typecheck | Draft; no credentials or live Host needed |
| HO-01 | RC-01/02; authenticated Host ADR | Foreground single-user Host with bounded sessions, runs, event stream, cancellation and one authoritative store owner; synthetic clients | Draft; no daemon or public listener by implication |
| MO-02 | MO-01, HO-01 | Real one-to-one Meister conversation, model/effort settings, streaming, tool cards, permissions, activity, session navigation | Draft; IOS/Android device acceptance separate |
| MO-03 | MO-02, RE-01/02 | Host-selected InstaBox status and attributed execution cards; uncertain outcome stays visible; no key on device | Draft |
| CL-01 | Runtime evidence and protocol review; independent of mobile | Optional native run-state integration with existing terminals, plain output fallback; no PTY ownership | Research complete enough to recommend, implementation Draft |
| CL-02 | CL-01; measured reuse experiment | Isolated tmux/Herdr subprocess prototype compares attach, resize, disconnect, output backpressure and status; no core coupling | Draft; full multiplexer deferred |

Recommended first slice is RC-01, implemented under the separately accepted
amendment. Finish its deterministic validation before remote work. RC-02 is now validated and complete after
[recovery PR #19](https://github.com/404khai/astrid/pull/19). The maintainer authorized
RC-03 on 2026-10-21 before RE-01, resolving the earlier prose/table ordering
inconsistency. Complete aggregate-exchange admission, then the RE-01 authority
contract and mocked adapter. MO-01 can use mocks independently; its apparent working
state must never imply a real run. Do not combine these tickets in one large PR.

## Smallest safe InstaBox runtime adapter (RE-01 design)

Extract the already tested HTTP executor behind a small library boundary; keep
Clap/environment/dotenv loading in the host/CLI adapter. Do not shell out to
`astrid instabox exec` as the model tool: that confuses credential inheritance,
structured results and cancellation. Reuse reqwest and the documented HTTP
contract; no SDK dependency or general remote execution framework is necessary.

Start with one configured existing exact machine ID per foreground run, no list
selection by model, provisioning, lifecycle, transfer, Vault, port, stdin or env
payload. Host configuration supplies an opaque credential handle. Tool arguments
are only command, absolute cwd and bounded timeout; the run's target cannot be
swapped by model text. A remote grant binds requesting principal, run, tool-call,
target ID and canonical arguments. Approval is distinct from local shell allow;
permission expiry, denial, cancellation or changed arguments prevents POST.

Proposed target-authority ADR must define that exact grant and metadata exposure,
including running-state check's possible pause/wake race. First adapter remains
outside scheduler workers: read-only worker grants cannot be elevated remotely.
No run/permission/session resume confers remote authority. Desktop observe/act
are separate later grants; remote exec itself is broad authority on that machine.

Map results to typed terminal outcomes with target, operation/call ID, elapsed,
separate stdout/stderr, exit status only when observed, provider timeout and
truncation. Preserve explicit `remote_outcome=unknown` after POST cancellation,
local deadline, transport error or malformed response. Unknown is not success,
termination or permission to retry. Pre-admission cancellation means no dispatch.
Per-request deadlines, total wait and byte caps remain independent. Avoid copying
arbitrary remote output/command text or credentials into metadata traces; private
session content and public metadata require deliberately separate projections.

Acceptance: actual wire payload fixtures; no auth before permission; exact-target
mismatch; every preset denied without grant; denial during approval; pre-POST and
in-POST cancellation; metadata/body deadline; oversized/malformed body; nonzero
exit; provider timeout; no retry; event parents/target correlation; credential
sentinels absent from model inputs, tool outputs and trace JSON. Live tests remain
separate and cannot create machines or transfer secrets.

## Computer-use investigation and bounded design

Official [desktop guide](https://instabox.sh/docs/desktop) and
[integration guide](https://instabox.sh/docs/integrate), inspected 2026-10-18,
document screenshot PNG, click, type, scroll, key/chord press, pointer operations
and status. Desktop startup is an operation with effects; do not automatically
call it under observation permission. Documented APIs are capability evidence,
not evidence that Astrid supports them. Verify and pin the current
[OpenAPI](https://api.instabox.sh/v1/openapi.json) schema before implementing Rust
payloads; this session's web renderer could not retrieve that JSON. Existing
merge research verified exec fields, not every future desktop payload.

Proposed first loop bounds: 10 actions, 60 s total, one action per fresh
observation, 1 MiB/frame and 5 MiB observation traffic. These are experiment limits,
not published API limits. Bound decoded pixels too, downscale before inference,
retain at most two image observations in model context and store earlier frames
only as bounded opt-in private artifacts. Keep IDs, dimensions, timestamp,
transform/scale and an incomplete text observation; never collect all full-size
frames in conversation. Reject stale coordinates after resize/new frame. Current
text-only request/accounting boundary needs image support and tests first; PNG
bytes/4 is not an image token count. Use provider vision support only when verified.

Observe authorization covers screenshot/status and their explicitly disclosed
inference destination; act authorization covers approved action types, exact
machine, expiry and limits. Typing may disclose secrets and a click/key may submit
an irreversible operation: pause for the concrete consequential action. Separate
lifecycle, network exposure and stream-link capabilities. Do not issue viewer
links by default: official docs describe bearer links with separate revocation
behavior; link creation is not mere screenshot observation.

Harmless acceptance uses a user-designated isolated existing machine and a local
scratch page/editor without credentials. Test click targets, Unicode typing,
scroll and chords against a fixture; stop on timeout, uncertain action result,
unexpected dialog, revoked grant or budget exhaustion. No automatic retries after
uncertain desktop actions and no production accounts, purchases, publishing,
secret transfer, machine creation or port exposure.

## Mobile and minimum Astrid Host proposal

Reuse `mobile/`; do not create another app. Inspected
`references/mobile/monocode reference.png` and both Superlogical images. Carry
forward the dark canvas, compact session rows, clear working/waiting labels,
model subtitle and anchored composer. Personal conversations must work without a
repository/project/branch. Display Meister as the initial conversation; Astrix
identity management/group coordination remains a separate future feature.

Use React Native for the shared conversation, virtualized history, tool cards,
permission sheet content and custom Android navigation/style. Split existing
`App`/`NativeSheet` and SymbolView usage behind platform files. Selectively retain
native SwiftUI sheets/navigation on iOS. Android should not import SwiftUI or
require Jetpack Compose/Material 3 as its presentation architecture. Expo's
[SwiftUI documentation](https://docs.expo.dev/versions/latest/sdk/ui/swift-ui/)
and [UI overview](https://docs.expo.dev/versions/latest/sdk/ui/) confirm the Apple
platform boundary. Installed package versions are in `mobile/package-lock.json`;
latest online examples require checking against those installed declarations.
No framework upgrade is required by this plan.

HO-01 proposes a versioned HTTPS JSON interface and bounded sequenced SSE stream
(fetch-stream feasibility on both installed RN targets must be measured; choose
WebSocket only if that bounded experiment warrants it). It is a single-user,
foreground Host initially, independent of GUI lifetime, not a new daemon. One
Host owns a session-store lock. Concurrent CLI access must fail clearly or use
that Host's client interface; never allow two writers. Unattended durability and
host restart recovery are out of scope. Losing the mobile connection leaves the
Host-owned run active; Host shutdown cancels, joins and saves only valid idle state.

| Interface proposal | Minimum contract |
| --- | --- |
| `GET /v1/capabilities` | Version, available models/efforts, configured targets, limits, run/continuation support; no credentials or fabricated capabilities |
| `GET/POST /v1/sessions`, `GET /v1/sessions/{id}` | Principal-bound cursor pages, visible conversation only, explicit resumable/inspection-only/incomplete state; never export private continuation |
| `POST /v1/sessions/{id}/runs` | Idempotency key, expected session revision, text and validated model/effort; return owned run ID, reject conflicting active writer |
| `GET /v1/runs/{id}`, `GET /v1/runs/{id}/events?after=cursor` | Bounded current snapshot and sequenced text/tool/task/usage/permission projections; cursor gap requires explicit snapshot resync |
| `POST /v1/runs/{id}/cancel` | Idempotent owner-scoped cancellation, accepted versus terminal status distinct; remote effects can remain unknown |
| `POST /v1/permissions/{id}/decision` | Principal/run/tool/target/argument binding, expiry, one consumption; stale, revoked or duplicate grant cannot dispatch again |
| `GET /v1/targets/{id}` | Configured InstaBox metadata/results under read grant, state timestamp, stale/unavailable indication; no model credential access |

Proposed trust baseline for the Host ADR: explicit local pairing ceremony issues
one revocable device credential, stored in platform secure storage; short-lived
scoped access credentials over validated TLS. No public bind, port-forward or
trust-on-first-use default. Local bootstrap must bind device to the same owning
principal; device labels are not identity. Revocation invalidates streams and
pending decisions. Model/provider/InstaBox credentials stay server-side and must
never appear in mobile config, bundle, crash logs, state persistence or messages.
Host authorization rechecks every read, run and decision. This is a proposal;
exact pairing/rotation and certificate provisioning need the remote-trust ADR,
not inheritance from local-account or superseded desktop IPC permissions.

Prototype limits to freeze before tests: 50 session rows/page, 100 visible
messages/page, 64 KiB text/event, 1 MiB page, 256 buffered events/client; disconnect
slow clients with resync rather than blocking inference. Coalesce incremental
rendering, virtualize history and keep screenshot assets outside transcript JSON.
Measure stream cancellation/background/reconnect on physical iOS and a stated
low-memory Android device; record peak memory, list responsiveness and payload
volume against a numerical baseline before acceptance. Test duplicate sends,
event gaps, expired credentials, revoked device, stale approvals, Host death and
unsupported effort. No user-visible promise of resumability before validation:
v2 routing can resume valid visible history, while incomplete or unverified
legacy/one-shot shapes can require inspection or `/new`. Phone backgrounding does
not grant automatic execution, notifications, overlays, Live Activities, widgets
or phone automation.

## CLI infrastructure evaluation

Upstream sources inspected 2026-10-18; no downloaded binaries or copied code.

| Candidate | Overlap / reusable concept | Integration, license and coupling |
| --- | --- | --- |
| [Superlogical Rex](https://www.superlogical.com/updates/public-testing-beginning) | Persistent terminal blocks and remote presentation; supplied images show sessions, split agent/tool panes and activity | Inspiration plus optional protocol adapter; public testing availability is not a reusable server SDK or source license. Do not embed or assume a stable RPC contract. |
| [Rex program status](https://www.superlogical.com/rex/docs/build/program-status) | Program-reported working/blocked/done/error/idle, independent of presentation | Preferred first experiment: small CLI-only OSC 7501 encoder from trusted runtime state. Version 0.4 dated 2026-10-10. No prompts/paths/secrets in status by default; no raw model/tool escape sequences. Verify plain/non-TTY and terminal fallback. |
| [Herdr](https://github.com/herdrdev/herdr), [state docs](https://herdr.dev/docs/session-state/), [socket API](https://herdr.dev/docs/socket-api/) | Rust terminal sessions, agent-awareness, attach and distinct restore paths | Start as external subprocess/socket experiment, not harness replacement. Current upstream LICENSE inspected is Apache-2.0; older releases have different historical licensing, so pin a revision/license before any code reuse. Separate terminal process authority from Astrid permissions. |
| [tmux control mode](https://github.com/tmux/tmux/wiki/Control-Mode), [license](https://github.com/tmux/tmux/wiki/Contributing) | Mature terminal process lifecycle and machine-readable commands/notifications over SSH | Preferred persistence prototype, optional subprocess backend; ISC upstream terms. C/server infrastructure stays outside Rust core; Astrid learns/owns agent state, task scheduling and authority rather than reimplementing commodity PTYs. |

No selected implementation dependency yet. Before choosing one, pin version,
license, macOS/Linux/Windows implications, process ownership, escape handling,
backpressure and shutdown behavior. Compare harmless isolated sessions with no
model auth: detach/reattach, resize, two panes, output flood, process death and
server restart. Screens restored after restart are not proof that a process or
conversation resumed. Herdr's explicit distinction is useful here.

Astrid's scheduler decides task readiness, ownership, budgets, cancellation and
result reconciliation. A multiplexer owns terminal processes, layouts and attach.
Neither terminal activity nor a shell exit proves an Astrid task succeeded. Show
only evidence-backed run/task state; do not parse animated terminal text to confer
authority. The first agent-aware CLI outcome can be a status protocol adapter in
an existing multiplexer, preserving the standalone CLI. Full PTY multiplexing,
remote writes and worktree isolation remain deferred to separately scoped work.

## Review and stopping conditions

RC-01–03 have passed their deterministic recovery/admission checks. RE-01 still
requires its concrete target-authority contract before dependent code.
A screenshot capability waits for bounded image admission. A real mobile connection
waits for the Host trust/lifetime contract. Terminal work cannot silently implement
a scheduler, durable jobs or Phase 9. Preserve private sessions, credentials,
recovery data, unrelated files and the existing CLI. Document actual mocked/live
validation separately; no live cloud or computer action is authorized merely by
writing these tickets.
