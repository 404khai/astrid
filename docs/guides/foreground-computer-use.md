# Testing foreground computer use

The Rust path connects InstaBox capture/status, validated native frames, model
vision and bounded desktop inputs. [ADR 0024](../adr/0024-foreground-computer-use-integration.md)
accepts the integration; actual live trials still need exact target/scope approval.

## Deterministic tests, no credentials

From the repository root:

```sh
cargo test --locked --offline --lib computer -- --test-threads=1
cargo test --locked --offline --test computer_png_worker -- --test-threads=1
cargo test --locked --offline --lib desktop -- --test-threads=1
```

The first suite exercises real HTTP adapters on loopback and a synthetic RGB PNG:
model asks to capture → capture approval → decode/native receipt → separate image
disclosure approval → mock model proposes click → action approval → input ack →
text-only follow-up. It checks denial, stale frames, binding/single-use grants,
byte overflow, no uncertain retry, metadata privacy and saved-session image absence.
The helper suite runs the actual Astrid PNG child on piped synthetic input and
checks deadline/cancellation kill and reap. Desktop controller tests cover all
four action families, geometry, counts, timing, concurrency and unknown outcomes.
These are mocked integration evidence, never live provider/VM evidence.

## Live invocation, only after separate trial approval

Prerequisites: an explicitly supplied existing running InstaBox machine with an
already-running X11 desktop on `:1`, ImageMagick `import`, `xdotool`, shell
`pipefail`, `head`, `base64` and `tr`; an existing host-side InstaBox key configured
by the supported CLI key mechanism and normal Astrid provider authentication.
ImageMagick pixel-cache memory is capped at 64 MiB with map/disk caches disabled.
No tools are installed, no desktop is started, and no machine is discovered or
provisioned by this mode. Native PNG must be <=1 MiB, <=2560 per dimension and
<=4,096,000 pixels; only noninterlaced 8-bit RGB/RGBA PNG is admitted.

Example template (replace target/task only under an approved live scope):

```sh
cargo run --locked --offline -- --computer-use --instabox-machine m_EXACT_ID run \
  --model gpt-5.6-sol "Observe the approved scratch screen, click its specified harmless target once, and report acknowledgement only."
```

Supported exact models: `gpt-5.6-sol`, `gpt-5.6-terra`, `gpt-5.6-luna`. Account
availability and image support through the configured authentication remain
unverified live. No routing, silent model substitution, notes, resume or interactive
CU mode. Ordinary local tool permissions still apply independently; shell Allow
does not bypass any CU approval, and selecting a target grants no operation.

Read each approval in the foreground terminal. Capture approval shows exact
machine/principal/run/calls and fixed command/cwd/timeout/limits. The next distinct
approval names the frame, native geometry, OpenAI destination/model and whole-screen
disclosure. Action approval shows exact typed input and wire plan, including typing
stdin. No approve-all setting. Denial, cancellation or stale/unknown outcome stops
CU; no automatic screenshot refresh, action retry or synthetic task completion.

Only one image is attached to the next model request with `detail: original` and
`store:false`. Local PNG/encoding buffers expire within ten seconds of capture
admission and are not saved/replayed. Provider policies still apply: `store:false`
is not zero retention. Screens may contain sensitive content; no redaction is
implemented. Model output may describe that content in the ordinary private text
history. Metadata traces never store screenshots, hashes, exact target or typed text.

An input acknowledgement does not verify UI effect. The experimental five-second
frame freshness includes capture/decode/model/approval latency and may expire in
normal use. Reject and inspect the actual result; approval cannot be reused for a
new frame. A practical production approval strategy needs separate review.
Existing context-byte limits can reject even a legally sized PNG; this mode neither
shrinks it nor raises budgets. Image tokens use a documented estimate plus margin,
not bytes/4; actual image-only provider tokens/cost are unavailable.

## Capture failures

The foreground summary now retains fixed safe categories instead of hiding every
capture error behind a generic unsupported-tool message. It never prints raw HTTP
bodies or guest stderr. The remote outcome remains unknown; no automatic retry.

| Category | What is known |
| --- | --- |
| Command not found, exit 127 | A pipeline executable is unavailable; an approved tool-path check can identify it. No automatic install/fallback. |
| Command exited unsuccessfully | The provider returned nonzero; desktop/capture/resource prerequisites need separately scoped diagnosis. |
| Command timed out | The fixed 2-second provider limit was reached; it is not increased automatically. |
| Output truncated by provider | Exec stdout is capped at 1 MiB in the audited public contract; base64 expansion can hit it before the PNG's 1 MiB limit. No silent resize/chunked recapture. |
| HTTP rejection, response/encoding/byte error | Transport/account/protocol failure; no valid frame admitted and no image sent. |

The maintainer's one approved diagnostic checks only tool paths and the display-1
socket; it does not capture, click, start a desktop or authorize model disclosure.
The agent process lacked an environment key, so that diagnostic was not dispatched.
Do not paste credentials into chat; use the existing configured terminal.

## Disclosure approval and expiry

A successful capture still needs a separate provider-disclosure approval. The
image's ten-second lifetime starts at capture admission, including preflight,
capture, decode, context preparation and terminal rendering. It does not start
when the approval prompt appears. `expires_in_ms` shows remaining time when the
prompt is constructed, not a refreshed allowance or a live countdown.

The CLI distinguishes explicit refusal (`image_permission_denied`), frame expiry
(`image_permission_expired`) and unavailable/failed input (`image_permission_failed`).
A `yes` at/after the deadline fails as expiry, never as a human refusal. Closed
terminal input or EOF is an input error, not a `no`. Entering `yes` and Return on
a fresh armed prompt remains required; pasted/typeahead text cannot grant authority.
No image is sent in these failure cases, and no approval transfers to a new frame.

This fixes misleading reporting, not the experimental timing UX. Ten-second image
retention and five-second action freshness remain unchanged. A more practical
approval/retention strategy requires a separately reviewed contract; the runtime
does not silently retain the screen longer or recapture it.

## Exact live trial contract still needed

Before any authenticated call, specify the existing machine ID and already-running
scratch desktop, exact harmless target/actions/typed data, provider/model and allowed
screen disclosure, maximum captures/disclosures/actions/runs, cwd/timeouts, possible
auto-wake, cancellation/unknown-outcome handling and the evidence to retain. No
viewer/ports, install/start/lifecycle, secret transfer or image persistence is covered.
A good first trial includes capture denial, disclosure denial, one acknowledged
harmless action, stale rejection and cancellation; production timing may require
separate contract changes instead of another attempt under the same approval.
