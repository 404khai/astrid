# Screenshot observation — public API audit and fixture review

Date: 2026-10-21. Branch: `feat/screenshot-observation`, same checkout, no worktree.
Baseline: `7d7f45d5` (main merge of #23); tree equals preceding RE-02 `7a425ace`.
CU-01 authorization covers concrete design and synthetic fixtures. No PR merge,
live/authenticated metadata, screenshots, inference or VM operation is authorized.
RE-02 live validation remains pending exact target/scope; its CLI mocks are not
live evidence. [ADR 0022](../adr/0022-screenshot-observation.md) was explicitly **Accepted with its gates** by the maintainer after the concrete
decision question; this does not authorize live/runtime integration.

## Public upstream audit

Fetched only public documentation/schema/package files without credentials;
no SDK install, package scripts or remote SDK execution. Web-rendered desktop/SDK
pages returned a JavaScript placeholder; search-indexed official docs described
the methods, and direct public schema/package retrieval resolved the wire gap.

| Source, retrieved 2026-10-21 | Verified fact | CU-01 implication |
| --- | --- | --- |
| [Official desktop guide](https://instabox.sh/docs/desktop), [SDK reference](https://instabox.sh/docs/sdk), [integration guide](https://instabox.sh/docs/integrate) | Public method descriptions distinguish desktop start, status, screenshot, inputs and bearer viewer links | Method availability is not Astrid authority or live evidence |
| [Official OpenAPI](https://api.instabox.sh/v1/openapi.json), info.version `1.0.0`, 203380 bytes, SHA-256 `d317b385641e504f4af1137691c8d902ea7eb44af2a82c3632ff417e7e5982a3` | `POST /v1/machines/{id}/desktop`, `DesktopRequest.action=status`; response includes required installed/installing/running/failed/agentPausable, plus sensitive log/actions/control fields | Use a strict result allowlist; POST status is not a guessed GET. Status schema has no width/height. No authenticated call made |
| Same schema | Other desktop paths are stream link creation/revocation, stream state and watch; no direct PNG screenshot route | Do not invent `/screenshot`; streaming/VNC/control are excluded |
| [Official SDK source](https://github.com/InsForge/instabox-sdk), published [0.5.3 tarball](https://registry.npmjs.org/@insforge/instabox-sdk/-/instabox-sdk-0.5.3.tgz), npm shasum `b91fae29342de5e8973f223df6dd442d8572d779` | `desktop.status()` posts status, then calls `xdotool getdisplaygeometry` when running; `screenshot()` uses scrot/ImageMagick, a random `/tmp` PNG, chunked file reads via exec and best-effort removal in finally | High-level status is not just one HTTP read. Screenshot is a command/file workflow with guest retention, cleanup uncertainty and possible auto-wake; no Rust live adapter is justified yet |
| Same package `dist/index.js` | Screenshot read ceiling is 32 MiB; general reads/mutations can retry, commands do not retry | SDK defaults do not satisfy proposed 1 MiB frame/no-retry contract; do not embed or invoke it |

Package inspection is protocol evidence, not adoption of its implementation or a
stable accepted command allowlist. No exact screenshot command sequence is approved
for live use. A future transport review must pin the commands, cwd, individual and
whole-operation timeouts, bounded file read/cleanup and all possible auto-wake,
including cleanup after uncertain dispatch. Status may touch guest control state;
an observe-only implementation must not claim side-effect-free server behavior.

## Review boundaries

The accepted design chooses a separate observe grant and ephemeral single-frame
slot, with zero model image inputs. The executable specification uses only
synthetic data and a fake monotonic clock. It must not be imported into the runtime,
advertised as a working screenshot tool, or used on real captured data. Production
permissions, CLI, provider, context, events, session and trace code remain unchanged.
No PNG data, paths, hashes or arbitrary screen content enters metadata traces.

Fixture coverage/validation and publication are recorded below.
Remaining gates: verified bounded live transport/cleanup;
maintained production decoder with bounded cancellation; headless runtime complete
exchange integration; explicit exact-machine live trial. Provider image disclosure,
retention/schema migration and CU-02 actions require later separate decisions.


## Fixture implementation and solo review

`scripts/screenshot_observation_fixture.py` is a Python executable specification,
not a Rust runtime tool or transport. Synthetic PNGs are generated in memory by
`scripts/test_screenshot_observation.py`; no captured screen, image file or user
reference asset is read. No new dependency, Rust export, tool registration, CLI
flag, provider input, session schema or trace field was added.

Sixteen deterministic tests cover every binding identity/canonical operation,
unknown/action arguments, unrelated local presets/exec/selection/resume values,
atomic single consumption across 32 contenders, no serialization, exact expiry,
cancellation racing admission, cancellation/deadline during receipt, failure with
no retry, categorical errors, status allowlist/inconsistency, replacement/failure/
geometry/close/expiry cleanup, stale call IDs, failed-frame traffic/count spending,
whole-run window, byte/pixel bounds, CRC/truncation/trailing PNG, ancillary/animated/
unsupported formats and decompression bombs. Accounting keeps PNG/RGBA/pixels
separate, image context zero, image tokens/cost unavailable.

Solo review checked the proposed contract against the existing text-only model,
context/admission, private v2, grants and metadata trace boundaries. No implementation
of those production boundaries changed. The fixture's Python lock demonstrates
single use only within this process; it is not authenticated trust. Chunk sources
already own their bytes and can block; fake time/deadlines cannot demonstrate hard
I/O cancellation or a bounded decoder worker. The restricted decoder is unsuitable
for real provider PNGs. Cleanup uses explicit sweep/access/close, not an idle timer;
Python allocation copies are not securely erased. These gaps are explicit live/
production gates, not passing claims. Exact targets remain test-only/private data.

## Validation

- `cargo test --locked --offline -- --test-threads=1`: **308 passed, 0 failed,
  1 ignored** (existing optional recording-overhead test).
- In `scripts/`, `python3 -m unittest -v test_screenshot_observation.py
  test_compare_routing.py`: **18 passed** (16 CU-01 and 2 existing comparison tests).
- `cargo fmt --check`: passed.
- `cargo clippy --locked --offline --all-targets -- -D warnings`: passed.
- Scoped whitespace and relative Markdown link checks: passed.
- Production `src/`, `tests/` and Cargo manifests/lock match the main baseline.

[Command and source evidence](../evaluations/screenshot-observation-2026-10-21.json).
No authenticated InstaBox/public provider call, live screenshot, cloud operation,
credential/private-session read, model disclosure, merge or installed CLI refresh.
CU-01 design/fixture slice is validated; full runtime/live observation remains
pending the explicit gates. RE-02 live trial is independently pending authorization.
