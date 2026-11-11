# Bounded desktop actions — contract, upstream audit and review

Date: 2026-10-21. Branch: `feat/bounded-desktop-actions` from freshly fetched
main `2daf811c` (merge of #24); same preserved checkout, no worktree.
CU-01 PR #24 was already merged upstream at 2026-10-21T18:14:30Z. No merge is
authorized or performed by CU-02. The maintainer requested CU-02 and its PR.
[ADR 0023](../adr/0023-bounded-desktop-actions.md) is Accepted with all four
maintainer clarifications, committed as `b452ef23` before dependent implementation.

## Current public upstream evidence

Public documentation only; no credentials, SDK installation, authenticated
metadata, machine discovery, model inference or live VM action. Web-rendered
[desktop guide](https://instabox.sh/docs/desktop) and
[SDK reference](https://instabox.sh/docs/sdk) again returned JavaScript placeholders;
direct public schema/package inspection provided concrete capability evidence.

- [Official OpenAPI](https://api.instabox.sh/v1/openapi.json), retrieved 2026-10-21:
  info.version `1.0.0`, 203380 bytes, SHA-256
  `d317b385641e504f4af1137691c8d902ea7eb44af2a82c3632ff417e7e5982a3`.
  Its desktop paths remain status/lifecycle/control, stream creation/revocation,
  stream state and watch. There are no direct screenshot/click/type/scroll/key
  endpoints. Do not invent REST payloads.
- [Official SDK source](https://github.com/InsForge/instabox-sdk), published
  [0.5.3 tarball](https://registry.npmjs.org/@insforge/instabox-sdk/-/instabox-sdk-0.5.3.tgz),
  npm shasum `b91fae29342de5e8973f223df6dd442d8572d779`, retrieved 2026-10-21:
  click uses an xdotool pointer move followed by a button click; scroll emits
  wheel-button repetitions at the current pointer; type passes exact text via
  stdin; press maps keys and joins a chord for xdotool. These travel through the
  command channel, not a dedicated action REST endpoint. SDK defaults and wider
  scroll/key support are not Astrid's accepted bounds. No SDK code is adopted.

These are documented/published capabilities, not an approved live command sequence.
Future transport needs its own exact command/argument/cwd/deadline/auto-wake review,
bounded responses and cancellation evidence, under a separate action grant rather
than a generic remote-shell grant. A click may partially move before failing;
typing may partially insert text. Unknown outcome cannot be retried safely.

## Implemented scope and solo review

CU-01 still lacks live observation transport, production decoder, runtime receipt
integration and model image permission. CU-02 therefore cannot honestly deliver
an end-to-end runtime screenshot/action loop. The implemented slice is a bounded
Rust headless controller with only mock transport evidence. Native frame metadata
is a trusted host assertion, never evidence that a screenshot was taken or sent
to a model. No action registration, new provider input or session/event schema.

Review/tests challenge all binding dimensions, each action, stale and replaced
frames, native bounds, delayed approvals, expiry, cancellation races, single use,
count/text/window limits, unknown outcomes, no retry and private content handling.
Exact command transport and full runtime/live acceptance remain gated. CU-01's
two-frame/30 s observation limits are unchanged.

## Deterministic acceptance matrix

| Boundary | Required counterexample / evidence |
| --- | --- |
| Separate authority | No broker, denial and unrelated observe/exec/local preset cannot call action transport; invalid args rejected before asking |
| Full binding | Change principal, run, runtime/provider call, target, frame, dimensions or canonical action after approval; no dispatch |
| Canonical input | Reject unknown fields, extra action-list/command fields, duplicate modifiers, invalid key/scroll/text/coordinate values; preserve exact reviewed Unicode/newlines |
| Fresh frame | No frame, old ID, replacement, age exactly 5 s, future receipt time, changed geometry and cross-run/target receipt rejected |
| Approval race | Expire frame/grant or cancel while broker waits; final admission invokes no transport |
| One-shot admission | Atomic concurrent consume admits once; second action cannot reuse the frame or spent grant |
| Execution | Exercise all four action families; a mocked acknowledged input is distinct from task success |
| Limits | Third action, 30 s boundary, oversized text, >10 wheel notches, invalid native x/y and duplicate chord keys rejected; no automatic increase |
| Unknown | Pending/error transport leads to unknown, invalid frame and closed controller; no automatic retry or next action after a fresh receipt |
| Privacy | Approval shows complete escaped arguments; categorical report/debug output contains no typed text, target or arbitrary transport error; no persistent schema change |

## Separate future live trial

No live trial is authorized by the implementation request. A later trial needs a
user-supplied exact existing isolated machine and approved scratch application,
fresh screenshot operations, exact input sequence/text/coordinates/keys, internal
commands/cwd/deadlines, trial count, cancellation behavior, cleanup and possible
auto-wake. No production account, purchases, publishing, secret entry/transfer,
machine lifecycle, public port or viewer link. Stop on an unexpected dialog,
unknown/partial outcome, stale frame or exhausted limit. Do not treat an input ack
as visible success; inspect a separately approved fresh observation. CU-01's two
screenshots per run may require distinct freshly approved runs for a multi-case
trial; no implicit budget increase or resumed grant.


## Rust controller implementation and limits

`src/desktop.rs` exposes typed `Action`/`Arguments`, a distinct `ActionApproval`
broker and `ActionGrant`, a one-method `ActionTransport`, and one exclusive
`ActionController` per host-owned run/target. Canonical policy `cu02-native-v1`
binds the fixed limits; JSON rejects unknown fields. No production transport,
SDK dependency, shell construction, key loading, runtime/model tool registration,
provider image input, CLI flag or session/event schema is added. Existing trace
projection code is unchanged; its new test persists only synthetic metadata.

Host-attested metadata is capped at two installations/replacements. Action count,
UTF-8 byte and original-window accounting never reset on a new receipt. There is
no global run registry; the trusted host must retain one controller per run/target
and must not construct duplicate owners. The seam cannot verify screenshots or
CU-01 byte/decode provenance. Live integration must replace that assertion with
receipts originating exclusively from admitted validated observations. Changing
frame metadata does not acquire observe permission or capture an image.

A frame is consumed at admission; pre-admission rejection invalidates it. Staleness
never silently refreshes a frame or rebinds an approval. Five seconds is only an
experimental mock bound: human approval can exceed it and a practical production
strategy needs separate review. Focus, pointer position and concurrent external
UI changes remain unverified; native coordinate validation is not remote atomicity.

Grant consumption follows all final checks without an intervening await. Before
awaiting transport, the controller marks itself closed and outcome unknown; even
caller-future drop cannot reopen it. Only an observed acknowledgement before local
deadline/cancellation can clear uncertainty. Cancellation in the same poll as an
ack also stays unknown. Timeouts/errors close the run, preserve spent budgets,
return categorical unknown and never retry. Ack confirms input acknowledgement,
not the intended effect. Cooperative async transport is required; synchronous
blocking or detached tasks would violate wait/cancellation assumptions and remain
unapproved for production.

Tokio's existing dependency gains `test-util` only in dev-dependencies for paused
clock tests; no new package or Cargo.lock change. Tests do not sleep, authenticate,
read private sessions/reference images or invoke any real desktop/provider.
Development checks caught a test temporary-borrow issue, a test mutex held across
await, and placement of the dropped-caller uncertainty flag; these were corrected
without widening authority, bounds or production integrations.

## Required contracts before a live observation/transport milestone

1. **Observation transport and safe decode:** audit exact screenshot/status
   commands, cwd, bounded body/file reads, temporary guest artifacts, cleanup and
   possible auto-wake; choose a production decoder with finite allocation/CPU and
   cancellable worker lifetime. Preserve CU-01 byte/pixel/count/window bounds and
   implement idle retention cleanup even without a consumer.
2. **Actionable provenance and ownership:** mint non-replayable real receipts only
   after successful observation admission and validated native geometry, tied to
   the actual run/principal/target and its single authoritative controller. Share
   admission accounting so metadata replacement cannot bypass screenshot limits.
3. **Practical approval/freshness:** separately review human approval delays,
   explicitly authorized recapture and a new exact-frame action approval; never
   reuse stale approvals. Define handling of focus, pointer, resize and external
   changes. Crop/scale/DPI coordinates need a concrete transform contract first.
4. **Action transport and outcomes:** review a fixed typed-action-to-wire/command
   mapping, complete exact arguments/stdin/cwd/timeouts, bounded replies, partial
   effects and cancellation. Preserve single-use admission/no replay and distinguish
   input ack, observed UI effect and unknown outcomes. No guessed REST endpoint.
5. **Runtime integration and privacy:** separately authorize tool registration and
   define its private exact-call result envelope, required permission/cancellation
   events and complete-exchange/context admission behavior. Preserve private v2,
   original results/continuation, incomplete-batch rejection, ADR 0008 off and ADR
   0009 allowlisted traces. Model image disclosure and screenshot persistence need
   their own destination/accounting/retention/schema decisions; neither is implied.
6. **Exact live trial:** separately approve the existing machine, scratch app,
   screenshot/input sequence and text, commands/cwd/deadlines, trial count,
   cancellation/cleanup and auto-wake scope described above. No live evidence here.

Stop after the mocked CU-02 slice and its unmerged PR. No next milestone is started.


## Final validation and publication

- `cargo test --locked --offline -- --test-threads=1`: **327 passed, 0 failed,
  1 ignored** (existing optional recording-overhead experiment). Includes 18 new
  action controller tests and one new persisted metadata-trace privacy test.
- `cargo test --locked --offline --lib desktop -- --test-threads=1`: 19 focused
  tests passed before the final full-suite run, which covers the pinned sources.
- From `scripts/`, `python3 -m unittest -v test_screenshot_observation.py
  test_compare_routing.py`: **18 passed** (existing CU-01/comparison regression tests).
- `cargo fmt --check`, locked/offline strict all-target Clippy and whitespace pass.
- Relative documentation links checked; command/source hashes are in
  [evidence](../evaluations/bounded-desktop-actions-2026-10-21.json).

The final full run follows the corrected dropped-caller uncertainty flag. No tests
use a real desktop or infer a live target. Mocked controller exit is satisfied;
full runtime/live CU-02 acceptance is not claimed. [PR #25](https://github.com/404khai/astrid/pull/25)
is updated for review against main, without merge. Unrelated `.gitignore`, local
CURRENT history, recovery artifacts and user files remain preserved and unstaged.
