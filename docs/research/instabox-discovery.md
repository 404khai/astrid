# InstaBox: optional Linux execution investigation

Date: 2026-10-18. Baseline `c9efb084`, branch `feat/instabox-discovery`, created
from the existing checkout HEAD without switching to main. User authorization:
official-source research and a small reversible local integration/read-only spike.
The initial slice excluded staging/commit/publication; the subsequent maintainer
request explicitly authorizes a scoped conventional commit, push and PR. No live
machine changes, repository transfer or credential inspection is authorized. Phase 8 remains implemented/reviewed with maintainer
acceptance pending; PR #16 remains the scheduler publication. This experiment does
not close Phase 8 or advance Phase 9. GPUI remains future Phase 11 work.

## 1. Current boundary assessment

The concrete implementation is one Rust package, not the directional crates diagram.

| Source | Existing contract and implication |
| --- | --- |
| [tools.rs](../../src/tools.rs), `ToolExecutor`, `Tools::shell` | Executor owns a local `Workspace`; shell uses `/bin/sh -c`, local cwd, process-group cleanup, bounded separate output and timeout. Killing a local HTTP client would not prove a remote process stopped. |
| [workspace.rs](../../src/workspace.rs), `Workspace::resolve` | Canonical local directory, traversal/symlink validation. These checks cannot validate cloud filesystem paths. |
| [permissions.rs](../../src/permissions.rs), [ADR 0004](../adr/0004-execution-authority.md), [ADR 0011](../adr/0011-permission-modes.md) | Typed local Read/Write/Execute; shell Allow/unbound is broad account authority, not a remote target grant. Repository/model text cannot grant authority. Existing generic shell can already invoke arbitrary clients; this spike neither restricts nor promotes that limitation into an InstaBox permission contract. |
| [runtime.rs](../../src/runtime.rs), tool dispatch loop | Checks capability/policy, approval and cancellation before `ToolCallStarted`/executor dispatch. Unknown tools fail. New remote execution cannot be hidden behind an existing local tool name. |
| [cancellation.rs](../../src/cancellation.rs), [ADR 0003](../adr/0003-bounded-tool-output.md) | Cooperative token, local output/cleanup guarantees. Remote cancellation needs a separate observed-outcome contract. |
| [events.rs](../../src/events.rs), [observability.rs](../../src/observability.rs), [ADR 0009](../adr/0009-persistent-trace-contract.md) | Checked per-run lifecycle and metadata allowlist; credentials, commands, arguments, output and arbitrary errors excluded from persistence. A new target attribution schema must be reviewed before runtime integration. |
| [model.rs](../../src/model.rs), [routing.rs](../../src/routing.rs) | Inference selection is separate from tool execution. Provider isolation is not permission policy or OS confinement. |
| [scheduler.rs](../../src/scheduler.rs), [ADRs 0017](../adr/0017-bounded-task-dag.md), [0018](../adr/0018-worker-authority-and-results.md), [0019](../adr/0019-batch-events-and-traces.md) | Fresh owned process-local workers, hard native read-only ceiling, shell denied, explicit result edges. No remote worker, automatic provisioning, durable jobs or cloud execution grant follows. |
| [session_store.rs](../../src/session_store.rs), [ADR 0016](../adr/0016-persistent-routing-provenance.md) | Private v2 idle history/original continuation retained; not a running-job checkpoint. |

## 2. Official evidence and options

Research used web browsing of every requested page: [docs](https://instabox.sh/docs),
[integrate](https://instabox.sh/docs/integrate), [machines](https://instabox.sh/docs/machines),
[SSH](https://instabox.sh/docs/ssh), [desktop](https://instabox.sh/docs/desktop).
The web reader returned a JavaScript shell; the hidden research browser was
unavailable. Public HTML and its referenced documentation asset were then fetched
and inspected as text, without executing downloaded JavaScript or installing software.
The actual guide bodies and API reference definitions are embedded in that asset.
The public [OpenAPI](https://api.instabox.sh/v1/openapi.json) was also fetched without
authentication on October 18; it advertises API version 1.0.0. Guide asset inspected
October 17: `/assets/fd3c814207460205/js/app.js`, SHA-256
`6355b256f1f589460d64c1daf330b18489175310e173f95430fe1726b27d8d52`.
OpenAPI SHA-256: `ce2b0d95eccaf21b1d8136252bc4d64956c3edecf3a5169eea38d8a5c798d096`.
These are research snapshots, not a promise of future API stability. No upstream
source was copied into runtime code; only independently written HTTP/JSON handling
using existing dependencies was added. No SDK license/dependency was imported.

Documented facts:

- Official CLI has `ls --json`, `get ID --json`, `usage --json`, `exec ID -- CMD`,
  lifecycle, files, snapshots, SSH and public-port commands. `INSTABOX_API_KEY`
  avoids interactive login; GitHub/Google login is also documented. The SDK guide
  advertises `@insforge/instabox-sdk` 0.5.0. No CLI version was installed or verified.
  [CLI reference](https://instabox.sh/docs/cli), [authentication](https://instabox.sh/docs/keys-sdk).
- HTTP uses `https://api.instabox.sh`, `X-API-Key` or Bearer authentication.
  `machines:read` covers machine metadata and account usage. Write scopes also
  permit exec/lifecycle; cloud scope breadth must not become Astrid authority.
  Missing key, expired/revoked keys and insufficient scope are documented failures.
  [API overview](https://instabox.sh/docs/api), [keys](https://instabox.sh/docs/keys-sdk).
- Discovery is `GET /v1/machines` (paged with `nextCursor`), `GET /v1/machines/{id}`
  and `GET /v1/usage`. IDs are `m_` plus 26 characters. Status reads do not record
  activity according to OpenAPI. Avoid SDK `connect`, whose default resumes;
  SDK inspection can specify `resume: false`.
  [machine API](https://instabox.sh/docs/api/machines), [account API](https://instabox.sh/docs/api/account),
  [integration guide](https://instabox.sh/docs/integrate).
- Exec is `POST /v1/machines/{id}/exec`: machine-user `bash -lc`, explicit cwd,
  `envs`, stdin and timeout; response has stdout/stderr/exitCode/timedOut/truncated.
  Nonzero exit is HTTP 200, SDK `commands.run` throws, raw `commands.exec` does not,
  CLI exits with the command's status. Output arrives at completion, not streaming.
  Default command timeout 60 seconds, maximum 600; timeout exit 124. Exec has no
  idempotency protection: retry executes again. Network interruption does not
  establish termination. [command API](https://instabox.sh/docs/api/commands),
  [integration](https://instabox.sh/docs/integrate), [keys/HTTP](https://instabox.sh/docs/keys-sdk).
- Files: SDK read/write/list/remove/exists; CLI `files`; HTTP live file/directory
  GET reads as root, at most 256 KiB text or 5,000 entries. File access and exec
  can auto-resume an idle-paused machine, so even a file GET is outside this spike.
  SSH certificates/tokens are capability issuance; SSH connections close on
  pause/stop/delete. No agent forwarding is supported.
  [command/file API](https://instabox.sh/docs/api/commands), [SSH](https://instabox.sh/docs/ssh).
- Desktop supports VNC, screenshots/actions and bearer viewer links; public ports
  can expose services. Viewer-link expiration stops new connections but does not
  end existing viewers; revocation does. All desktop/browser/exposure operations
  are future scope. [desktop](https://instabox.sh/docs/desktop), [ports](https://instabox.sh/docs/ports).
- Published defaults: 5 machines including paused/stopped, 10 snapshots, storage
  guide says 200 GB; wire field is `maxStoredGiB` (default 200) against `storedBytes`.
  Account usage is measured roughly hourly; `computedAt: null` means not measured.
  The user reports 5/10/200 GiB, but no authenticated account entitlement or usage
  was verified. Preserve wire units rather than equating GB and GiB. Exhausted
  machine/storage quota blocks new machines/snapshots; `429 quota_exceeded` also
  covers shared request concurrency (documented 32). [limits](https://instabox.sh/docs/limits),
  [account API](https://instabox.sh/docs/api/account), [OpenAPI](https://api.instabox.sh/v1/openapi.json).

| Option | Benefit | Cost/risk | Disposition |
| --- | --- | --- | --- |
| Official CLI through authorized shell | Fast manual use; existing local output/exit capture | Install/version/config/token-store/subprocess coupling; local shell grant cannot express target-specific exec; killing client does not prove remote cleanup | Useful future manual acceptance reference; do not install or wrap broad commands now |
| Small Rust HTTP adapter | Existing reqwest/serde, fixed allowlisted GETs, bounded responses, deterministic wire tests, no subprocess or SDK | Own a few DTOs/error mappings; exec/lifecycle would need new contracts | Selected only for metadata discovery experiment |
| Defer first-class integration to execution-target milestone | Avoid premature universal-computer abstraction and security/schema changes | No remote execution benefit today | Selected for execution/lifecycle/transfer, alongside today's independent discovery spike |

InstaBox supplies optional Linux compute, overlapping future execution targets,
not Astrid inference, scheduling policy or personal-agent identity. Protocol adapter
is the intended integration form; no service embedding, SDK runtime, new crate or
universal remote-computer trait. Understanding target authority and uncertain
remote effects remains Astrid-owned learning work.

## 3. Actual local implementation and minimal CLI plan

Implemented [instabox.rs](../../src/instabox.rs) as a private CLI module, wired in
[main.rs](../../src/main.rs). No library/public runtime adapter or model tool.
`astrid instabox discover --allow-discovery` reads one page (limit 100) then account
usage. `--machine m_…` reads only that explicit machine then usage, without waking it.
Selecting a machine is inspection, not connection or execution authorization.

Output is JSON: `target: instabox`, machine IDs/states, `list_complete`, usage and
wire limits, storage-measurement availability and total `elapsed_ms`. Unknown states
become `unknown`; names, labels, cursors, credentials and arbitrary error text are
omitted. A partial page reports incomplete; a selected machine never claims complete
account inventory. Lists/usage are separate observations, not an atomic snapshot.
No default entitlement, quota headroom or provisioning readiness is fabricated.

The command requires its own explicit grant, independently of local permission
presets. Missing grant denies before reading the key; invalid IDs deny before
network. Production origin is fixed HTTPS; URL overrides, redirects, proxy environment
configuration and TLS bypass are unsupported. Only `INSTABOX_API_KEY` is read;
no CLI credential store, provider authentication or GitHub token is consulted.
Header is marked sensitive; transport/status/JSON errors are categorical, never raw
server/request errors. Bodies are bounded to 256 KiB, lists to 100; overall deadline
15 seconds includes all request bodies, connect timeout 5 seconds. Cancellation
uses existing `Cancellation`; Ctrl-C drops read-only HTTP work. No retries or owned
background tasks. No traces, sessions, models or workspace changes are produced. If either metadata
request fails, no partial discovery result is printed. Storage-measurement age is
not exposed; the availability boolean cannot establish freshness or quota headroom.
This management command is outside run lifecycle; it does not claim runtime tracing
coverage or replace existing permission events.

Safe local commands (no cloud access):

```sh
cargo run --locked -- instabox discover --help
cargo run --locked -- instabox discover
cargo test --locked instabox
cargo test --locked --test instabox_cli
```

The second command intentionally fails with discovery denied. Proposed future
read-only account check, **not performed here**: after the user supplies a restricted
`machines:read` key through environment/secret manager (never a literal shell
argument), run:

```sh
cargo run --locked -- instabox discover --allow-discovery
cargo run --locked -- instabox discover --allow-discovery --machine m_REPLACE_WITH_EXACT_26_CHAR_ID
```

For your own optional account check in **zsh**, enter the key at a hidden prompt
rather than pasting it into chat or a command argument/history. Use a restricted
`machines:read` key. The following is user setup guidance only; it was not run
against a real key during development or publication:

```zsh
read -rs 'INSTABOX_API_KEY?InstaBox API key: '
printf '\n'
export INSTABOX_API_KEY
cargo run --locked -- instabox discover --allow-discovery
unset INSTABOX_API_KEY
```

The key exists only in that shell environment until unset or the shell exits.
Astrid does not load `.env` files or persist this key; do not commit a key or `.env`
file. A secret manager may also populate `INSTABOX_API_KEY` in the launched process.
This optional integration uses existing HTTP dependencies and adds no SDK dependency.

The ID placeholder is intentionally invalid; replace it with an observed ID.
`INSTABOX_API_URL` is intentionally ignored. There is no offline fixture flag in
production; loopback HTTP injection exists only in private unit tests.

## 4. Proposed security and lifecycle contracts (execution not implemented)

Separate explicit target grant should bind requesting local principal, exact machine
ID/account, operation, command, cwd, bounds and expiry for one invocation. Model or
repository instructions cannot widen it; local shell Allow/unbound does not imply
remote grant. Discovery, exec, transfer, lifecycle, snapshot and public exposure need
separate grants. Provider sandboxing and a restricted cloud key are defense layers,
not permission policy. No automatic machine provisioning or fallback to local exec.

Future exec must require an already running, explicitly selected existing machine;
refuse paused/stopped/transitional/unavailable states rather than auto-resume. Recheck
before dispatch but acknowledge the race: cloud exec can wake idle-paused machines.
A strict no-wake execution guarantee requires provider support/conditional execution,
not a status-read assertion. Never silently start/resume on user denial or quota failure.

| State/action | Documented retention | Astrid consequence |
| --- | --- | --- |
| Running | Files/software and live processes | Execution still needs its own grant |
| Paused (`suspended`) | Files and saved memory/processes | Resume may boot fresh if restoration fails (`runtime.processesPreserved: false`); no guaranteed job survival |
| Stopped | Files/software retained, processes lost | Start is a separate lifecycle grant |
| Deleted | Machine/disks gone; existing snapshots retained | Destructive grant required; deleted IDs get 404 and disappear from lists |
| Snapshot | System files and `/data`, including home/sign-ins; no memory/processes | Snapshot is sensitive data retention, not an Astrid run checkpoint |

These distinctions are documented in [concepts](https://instabox.sh/docs/concepts),
[machines](https://instabox.sh/docs/machines), [lifecycle API](https://instabox.sh/docs/api/lifecycle)
and [snapshots](https://instabox.sh/docs/snapshots). The snapshot guide describes
saving stopping the machine; newer OpenAPI also permits live snapshots when
`limits.liveSnapshots` allows. Treat that as a capability/version distinction,
never silently choose a lifecycle-changing snapshot mode.

Before any separately approved disposable-machine provisioning, refresh actual quota
and storage-measurement coverage, account for stopped/paused machines and snapshots,
then require explicit create grant and disable automatic Vault injection: HTTP
`"vault": false`, SDK `{ vault: false }`, CLI `create --no-vault`. OpenAPI describes
false as permanently opting that machine out; absent/true injects at boot/resume.
The SDK also documents `disableVault()` for existing machines, but changing an
existing machine is separate scope and is not a credential-erasure guarantee.
Existing disks/snapshots may already contain secrets; disabling injection cannot
prove they are clean. Never transfer local provider/GitHub/Git/SSH credentials,
repository files, entire environment or auth stores. Do not read Vault values.
[Vault guide within machines](https://instabox.sh/docs/machines),
[key warning](https://instabox.sh/docs/keys-sdk), [create schema](https://api.instabox.sh/v1/openapi.json).

Lifecycle calls return operations and require idempotency keys; operation acceptance
is not readiness. Exec retries must be prohibited automatically. Future exec must
retain bounded separate stdout/stderr, actual exit code, elapsed time, explicit
machine attribution and output coverage. Nonzero exit is an executed failed command,
not transport failure. Timeout/cancel/disconnect must distinguish observed remote
termination from **unknown remote outcome**; never claim local process-group cleanup
terminates cloud jobs. No durable background/reconnect/crash-recovery promise follows
from VM files, memory preservation, tmux or snapshots.

## 5. Deterministic tests and acceptance

[HTTP mocks](../../tests/support/instabox_mock.rs) run private loopback servers with
synthetic keys, no cloud or real authentication. [CLI checks](../../tests/instabox_cli.rs)
spawn the actual binary with an isolated empty HOME and cleared environment.

| Requirement | Implemented evidence / deferred acceptance |
| --- | --- |
| Discovery request/auth shape and attribution | Exact GET paths, synthetic X-API-Key only, projection and quota numeric values tested |
| Permission denial / invalid IDs / missing key | No-dispatch unit checks plus real CLI rejection; no files created in isolated HOME |
| Auth failures | 401 expired/revoked/invalid class; 403 scope/account class. No per-code diagnostic inferred; body discarded |
| Unavailable/deleted/service failures | Mock 404/503/500 and redirect refusal; no follow-up mutation or retry |
| Quota failure | Mock 429 maps to quota; full usage is displayed honestly, never used to provision |
| Cancellation / timeout | Pre-cancel before dispatch; cancel and deadline during stalled body, explicit results and joined test-server cleanup |
| Output bounds / invalid wire data | Oversize body, malformed JSON, missing/invalid cursor, wrong machine ID, unknown state, privacy sentinels |
| Remote nonzero exit and exec timeout | Deferred: mock HTTP 200 exit 7/124 with separate stdout/stderr/truncation, no retry; no exec code exists today |
| Target exec grant denial and cancellation race | Deferred: executor spy proves zero POSTs before exact target grant, cancellation checked after approval, local unbound cannot widen it |
| Remote execution cancellation | Deferred: during/after dispatch disconnects retain uncertain outcome; demonstrate server cancellation before claiming termination |
| Lifecycle/transfer/exposure | Deferred: independent grants, no automatic wake/provisioning, quota failure sends zero creates, Vault false serialized, no inherited secrets |
| Runtime trace integration | Deferred: categorical metadata and target attribution schema review; content/credentials absent, existing run ordering/session v2 unchanged |

Validation results are recorded in the completion evidence below. No live smoke, VM command,
SSH certificate, transfer, machine lifecycle, snapshot, browser or desktop action
was performed. HTTP mocks validate our discovery behavior, not actual entitlement,
provider availability, cloud isolation or remote command semantics.

## 6. Placement and next concrete action

This is a separately authorized scoped discovery experiment adjacent to the existing
roadmap's M07 execution-environment objective; it does not activate historical M07
prerequisites or start Phase 10/12. [Canonical roadmap](../roadmap.md),
[active Phase 8 plan](../plans/phase-8.md) and acceptance markers remain authoritative.

Next implementation action: propose one narrow execution-target ADR covering exact
machine grants, auto-wake race, uncertain cancellation and metadata attribution;
prepare mocked command results before wiring a model tool. Keep this contract
Proposed until the maintainer accepts it; no universal device abstraction needed.

Concrete future live acceptance gate: only after that implementation and separate
user authorization, name the exact account and already-running machine ID and approve
one POST to `https://api.instabox.sh/v1/machines/<EXACT_ID>/exec`, JSON
`{"cmd":"printf 'astrid-instabox-probe\\n'","cwd":"/data/home","timeoutSeconds":10}`,
no envs/stdin/transfer, no automatic retries or lifecycle calls. Use a dedicated
key from a secret manager/environment, never place its value in this report or logs.
Potential effect includes waking an idle-paused machine between check and dispatch;
approval must cover that documented possibility or execution must remain blocked.
Observe stdout, stderr, exitCode, timeout/truncation and elapsed time with attribution.
This is a proposed approval payload, not authorization or an executable Astrid command
today. No approval is needed to review the completed local spike.

## Solo scope review

The adapter remains private to the CLI and exposes only three fixed metadata GET
shapes. No runtime capability mapping, worker ceiling, inference/provider path,
trace/session schema, permission preset, local tool executor or dependency changed.
Production rejects arbitrary origins and redirects; synthetic loopback origins are
reachable only through test-private construction. Denial/invalid-ID preflight is
routed before the ordinary CLI's provider-auth-directory setup. Free-form response
and error strings are excluded from diagnostics/output; the decoded in-memory
HTTP response itself is not a persisted trace or model input.

Reviewed limitations: discovery is one bounded page and two non-atomic observations;
no custom proxy/server support, automatic auth refresh, pagination traversal, quota
reservation, remote exec, worker dispatch, cancellation proof for remote processes,
file transfer or live interoperability evidence. Existing broad local shell access
remains broad; this change cannot sandbox arbitrary programs a user has authorized.
No claim is made that the CLI flag creates a new authenticated remote principal.
There is no material unresolved implementation finding within the discovery scope.
Execution authority and target trace attribution stay proposed future contracts.

Validation history: the initial focused run could not bind localhost under the
sandbox and failed before HTTP assertions. It passed all seven cases when rerun
with loopback permission. The first full suite then failed the existing
`personal_tasks::coding_fixture_still_inspects_edits_and_verifies_with_real_tools`
fixture because its nested Rust compile exceeded its 30-second shell deadline
while a separate Clippy build was also running. The isolated rerun passed in
20.65 seconds after Clippy finished. The fixture/deadline was not changed; the
failed attempt is retained rather than counted as a passing full run.

## Completion evidence — 2026-10-18

Final exact source checks: `cargo test --locked` **263 passed, 0 failed, 1 ignored**
(25 suites including doc tests); `cargo fmt --check`,
`cargo clippy --locked --all-targets -- -D warnings` and `git diff --check` passed.
`python3 -m unittest discover -s scripts -p 'test_*.py'` passed **2/2**.
The seven new HTTP unit tests and one real CLI preflight test all passed in the
full run. The final full run had no concurrent Clippy/build-check invocation.
The ignored test is the existing optional recording-overhead experiment.
[Machine-readable evidence and source hashes](../evaluations/instabox-validation-2026-10-18.json).
Test runs requiring localhost listeners used the permitted unsandboxed local
execution path; the only network peers were synthetic test servers.

Changed-file inventory:

- `src/main.rs`: CLI registration and early management-command dispatch.
- `src/instabox.rs`: private bounded metadata discovery adapter/CLI.
- `tests/support/instabox_mock.rs`: seven synthetic HTTP tests.
- `tests/instabox_cli.rs`: actual CLI preflight test, no authentication/network.
- `docs/research/instabox-discovery.md`: research, contracts, plan and review.
- `docs/evaluations/instabox-validation-2026-10-18.json`: exact-source check evidence.
- `docs/CURRENT.md`: current branch/scope/result checkpoint.
- `docs/roadmap.md`, `docs/plans/phase-8.md`: narrow experiment navigation only.

No dependencies, local tool behavior, accepted permission/session/trace contracts,
scheduler code or phase acceptance markers changed. Original unrelated untracked
files and recovery data were left alone; additional untracked reference images
observed during validation were also left alone. At implementation completion there
was no staging, commit or PR publication. The later publication request is separate
authorization; see the publication record below. No reset, worktree creation, switch
to main or cloud side effects.

## Publication preparation — 2026-10-18

The maintainer requested a conventional commit and PR after implementation completion.
Fetched `origin/main` is `f3d2f3cd825e230cab3a69ee802cce21bdf36de1`, the merge of
[scheduler PR #16](https://github.com/404khai/astrid/pull/16). Its tree is identical
to the original `c9efb084` baseline, which is its ancestor. The InstaBox PR therefore
targets `main`, with no rebase/merge, branch switch or scheduler diff.

Solo pre-publication review covered all nine files, including new source/tests.
The four code/test SHA-256 values still match the original validation evidence;
only documentation/setup/publication notes changed after those passing checks.
No additional runtime tests or live authentication are claimed for publication.
The scoped index, source hashes, local document links and whitespace are checked
before committing. Credential examples contain prompts or synthetic test values only.
