# Bounded desktop actions — contract, upstream audit and review

Date: 2026-10-21. Branch: `feat/bounded-desktop-actions` from freshly fetched
main `2daf811c` (merge of #24); same preserved checkout, no worktree.
CU-01 PR #24 was already merged upstream at 2026-10-21T18:14:30Z. No merge is
authorized or performed by CU-02. The maintainer requested CU-02 and its PR.
[ADR 0023](../adr/0023-bounded-desktop-actions.md) is initially Proposed; dependent
authority implementation awaits the specific decision.

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

## Scope and review plan

CU-01 still lacks live observation transport, production decoder, runtime receipt
integration and model image permission. CU-02 therefore cannot honestly deliver
an end-to-end runtime screenshot/action loop. The proposed slice is a bounded
Rust headless controller with only mock transport evidence. Native frame metadata
is a trusted host assertion, never evidence that a screenshot was taken or sent
to a model. No action registration, new provider input or session/event schema.

Review/tests must challenge all binding dimensions, each action, stale and replaced
frames, native bounds, delayed approvals, expiry, cancellation races, single use,
count/text/window limits, unknown outcomes, no retry and private content handling.
Exact command transport and full runtime/live acceptance remain gated. CU-01's
two-frame/30 s observation limits must not grow implicitly to fit CU-02.

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


## Preparation checkpoint

Baseline `cargo test --locked --offline -- --test-threads=1`: **308 passed,
0 failed, 1 ignored**. This is unchanged-runtime regression evidence, not CU-02
controller or live evidence. Scoped whitespace and 50 local Markdown links pass.
No dependent action controller implementation exists while ADR 0023 is Proposed.
The authorization decision is pending; no acceptance is inferred from elapsed time.
