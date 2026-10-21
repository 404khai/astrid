# Foreground computer use — integration review and evidence

Date: 2026-10-21. Branch `feat/foreground-computer-use` is stacked on open CU-02
PR #25 (`4d10f91f`), same preserved checkout, no worktree. The maintainer requested
actual CU, reviewed the concrete privacy/runtime proposal, then explicitly accepted
[ADR 0024](../adr/0024-foreground-computer-use-integration.md) option 1. Contract
commit `390bddec` precedes implementation. No merge or actual live operation.

## What changed

The missing application path is implemented: invocation-only foreground tools,
fixed InstaBox wire adapters, maintained PNG helper/native receipts, distinct
observation/action/image disclosure approvals and one-request OpenAI vision.
[Testing/usage](../guides/foreground-computer-use.md) describes deterministic tests
and the future authorized live invocation; [milestone](../milestones/foreground-computer-use.md)
records scope. The Rust runtime remains authoritative; CLI is first-class, Meister
primary and Astrix specialized. No mobile/Host/worker/desktop client work.

`computer_wire.rs` maps bounded typed actions to audited exec payloads; typing is
stdin, never shell source. It uses no inventory, viewer, port, retry, redirects,
proxy, lifecycle/start/install calls. Capture is one fixed ImageMagick PNG24 stdout
pipeline, including memory limit and disabled mapped/disk pixel caches. Shell
pipefail, capped stdout/base64, provider timeout and response limits fail closed.
Guest tool availability and behavior are unverified live; this is narrower than the
SDK, with no scrot/file fallback or implicit package/desktop installation.

`computer_png.rs` bounds framing, CRC, chunk count, dimensions, compressed bytes
and decoder allocations. It accepts noninterlaced 8-bit RGB/RGBA only, rejects
animation and unknown critical chunks, strips ancillary metadata before decoding,
then canonically re-encodes. Production uses a 2-second child with cleared
environment, bounded pipes and kill/reap; no image files or OS sandbox claim.
The child stdin is explicitly closed after writing so decode cannot wait for EOF
until the deadline. Actual helper-process fixtures verify that behavior.

`computer.rs` owns one controller/frame/run. Successful native decode precedes real
receipt admission. Exact observe grants consume before authenticated preflight;
a failed preflight reports no execution POST dispatch. Input grants consume
atomically immediately before transport admission. Existing original windows,
counts, retention and stale checks do not reset on replacement. An invalid/denied/
stale/cancelled/uncertain operation closes CU without recapture/retry/continuation.
An acknowledgement means only that the input command was acknowledged, never
verified UI effect or synthetic task completion.

`runtime.rs` installs CU only for a fresh one-shot fixed supported model, explicit
context budget and foreground capability. Read-only workers cannot inherit it.
Selecting a machine, local permission presets, resume and shell grants cannot
approve observe/action/disclosure. Per-run terminal replies use existing isolated
foreground channels. Every private tool result keeps original actual outcomes;
remaining batch tools are skipped on failure, complete-exchange/cancellation
invariants retained. Interrupted unmatched batches still cannot continue.

`openai.rs` attaches one native original-detail input image only on the next model
call, under an exact frame/provider/model/destination single-use disclosure grant.
Production destination is fixed OpenAI Responses; test-only loopback override is
compiled only for unit fixtures. Existing request bytes and text allowance plus
documented patch estimate are checked before disclosure; no automatic budget growth.
Actual image-only usage/cost is unavailable; provider aggregate input includes it
when reported. A ten-second frame deadline also bounds preparation/auth/network
wait and encoding buffers. Already-transmitted provider data cannot be recalled.

Images have no Message/session representation. Fresh CU sessions use existing
permanent visible-history replay; original provider outputs and provenance remain
unchanged, later requests contain text metadata/results only. Numeric transient
image context admission extends checked event replay without weakening text
selection equality. Persistent traces project categorical kinds/allowlisted tool
names only; no target/frame/digest/image/typed data or approval contents. Session
v2 is unchanged. Ordinary model text may itself describe sensitive screen data.
`store:false` does not promise zero provider retention or local forensic erasure.

## Current public evidence

Only public unauthenticated sources were inspected, no account/key/session reads.

- [InstaBox OpenAPI](https://api.instabox.sh/v1/openapi.json), retrieved 2026-10-21:
  version `1.0.0`, 203380 bytes, SHA-256
  `d317b385641e504f4af1137691c8d902ea7eb44af2a82c3632ff417e7e5982a3`.
  Exec accepts cmd/cwd/stdin/timeoutSeconds; desktop status accepts action=status.
  No direct screenshot/action route. No guessed endpoints/payloads.
- [Official published SDK](https://github.com/InsForge/instabox-sdk), npm 0.5.3,
  shasum `b91fae29342de5e8973f223df6dd442d8572d779`: command-based X11
  screenshot/input and stdin evidence. SDK writes a temporary screenshot; Astrid's
  separately audited stdout pipeline deliberately does not adopt that storage.
- [ImageMagick import](https://imagemagick.org/import/),
  [stream output](https://imagemagick.org/command-line-processing/) and
  [format/resource options](https://imagemagick.org/command-line-options/) document
  root capture, stdout, depth/interlace and disabled disk/mapped pixel caches.
  [Formats](https://imagemagick.org/formats/) documents PNG24.
- [xdotool primary source](https://github.com/jordansissel/xdotool/blob/main/cmd_type.c)
  documents --file - reading stdin. Its command documentation supports native
  mousemove/click, wheel buttons and key chords. No model text interpolated as shell.
- [OpenAI images/vision](https://developers.openai.com/api/docs/guides/images-vision)
  documents the exact `gpt-5.6-sol/terra/luna` image models, data URLs, original
  detail preserving this geometry and 32-pixel patch accounting times 1.2.
  Admission adds one token rounding margin. This is an estimate, not actual
  provider image usage. No model-prefix capability inference.
- [OpenAI data controls](https://developers.openai.com/api/docs/guides/your-data)
  distinguishes store:false application state from provider retention policies.

## Deterministic acceptance matrix

| Boundary | Executed evidence |
| --- | --- |
| Complete runtime path | Real reqwest OpenAI/InstaBox adapters on loopback; capture → disclosure → click → text-only follow-up; checked event replay equals actual state |
| Separate authority | Capture/disclosure/action denial blocks corresponding HTTP; shell/presets do not auto-grant; exact observe binding and eight concurrent consumers admit once |
| Native admission | Maintained synthetic PNG decode precedes receipt; wrong target/stopped preflight cannot POST; status retains only booleans |
| Decoder bounds/privacy | CRC/truncation/trailing, animation, unknown critical chunks, oversize geometry/bytes, invalid compressed IDAT, chunk count and compressed ancillary stripping |
| Helper lifetime | Actual Astrid child on piped PNG, pre-cancel no spawn, owned stalled child deadline/cancellation kill and reap |
| Disclosure | Exact model/destination/current frame, cancellation and atomic single use; idle ten-second cleanup, no old image replay |
| Context | Image request-byte overflow stops before disclosure/second provider request; base64 excluded from text bytes/4, separate patch estimate; original complete-exchange admission regressions |
| Action invariants | Native 5-second stale rejection without refresh/input, injection text only stdin, bounded keys/wheel; existing 19 controller tests cover geometry/window/count/text/concurrency/unknown races |
| Unknown outcome | Mock action503 produces actual unknown report and no retry/third inference; existing timeout/cancellation controller regressions |
| Persistence/traces | Actual private v2 save/load has no image data or ImageInput selection; persisted metadata excludes credentials, target, typed/arbitrary remote content and image payload |
| CLI scope | Opt-in requires exact target; fresh run/supported model only; notes/routing/interactive/unsupported model rejected before authentication |

## Validation

Final executed results are recorded in the [machine-readable evidence](../evaluations/foreground-computer-use-2026-10-21.json).
**345 Rust passed, 0 failed, 1 ignored; 18 Python regressions passed.** Fmt, strict
all-target Clippy and whitespace passed. The final focused computer suite passed
14 tests after tightening the capture cache limits.
No live APIs are used by these tests. The existing environment-specific overhead test remains
ignored, not counted as passed. Development failures (worker EOF, session-fixture
workspace and checked image-context admission) were corrected and revalidated.

## What remains for actual live use

The application path is present, but no live evidence is claimed. A separate exact
existing-target/trial contract must name scratch screen/actions/typed data, allowed
capture/provider disclosure, exact supported model, run/call bounds, cwd/timeouts,
auto-wake and cancellation/unknown behavior. No target has been supplied. Verify
installed tools/native geometry/account support and measure real latency/effects
only within that trial. RE-02 live evidence remains pending independently.
Five-second freshness is experimental and can make normal human/model latency fail;
no practical production approval strategy is accepted here. Large legal PNGs may
still exceed existing context-byte limits. No redaction, image persistence, silent
resize/model substitution, workers, background loop or successor work. Stop after
this integration slice and its unmerged PR.

Publication: [PR #26](https://github.com/404khai/astrid/pull/26) targets main,
stacked on open #25. Contract `390bddec`, implementation `17d5f70c`; both PRs
remain unmerged. No live operation or successor implementation was performed.
