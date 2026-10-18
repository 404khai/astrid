# Terminal redesign — 2026-10-18

User-authorized CLI presentation work on `feat/tui-redesign`, created directly
from existing HEAD `31a3986b` (`feat/instabox-discovery`) in the same checkout.
The initial implementation did not stage or publish. The maintainer subsequently
requested this work be published and merged after InstaBox PR #17. That PR is now
merged at `5d423a7d`; this TUI PR targets updated `main` and excludes the InstaBox
implementation diff. No main switch or worktree was used.
No runtime phase advancement or desktop implementation.

## Presentation and compatibility

Directly inspected `references/tui.png` and `references/tui_styled_text.png`.
After the user's correction “Put my tui logo back in there”, the welcome retains
the original bracketed-face mascot from `src/logo.rs` alongside the geometric
Unicode/block ASTRID wordmark. Original pixel geometry is unchanged. The lettering
keeps its dark offset extrusion, quiet divider and centered live identity details.
`wordmark.rs` owns the glyphs; `theme.rs` interpolates the seven foreground rows
through the exact ARC stops from `references/mobile/svgs/logo.svg`:
`#4459F9` at 0%, `#1A32EC` at 50%, `#4459F9` at 100%.
No raster assets, generated images or new dependencies.

Welcome/input layout is shared with render tests. The reading column caps at
88 cells with side margins; the shaded composer includes mode/context/session/
model metadata and keyboard help. At 69+ columns and 22+ terminal rows the original
30×10 mascot sits beside the 35-column wordmark (plus its shadow); at 104 columns
the lettering doubles pixel width. Narrow windows with 36+ columns and 32+ rows
stack the original mascot and block wordmark. At 30+ columns and 22+ rows, shorter
windows retain the full mascot with compact ASTRID text. Smaller windows use
compact identity, prioritizing mode/model and input space. The startup viewport
caps at 40 rows and can grow after a small launch. Append startup also respects
available height. Compatibility fixed-viewport execution uses compact centered metadata,
leaving more rows for output. Default streaming still inserts completed rows into
ordinary terminal scrollback; the hero remains startup-only.

ADR 0011's original mascot palette is restored: blue arches with teal eyes in
normal modes, red edge/middle/edge arches (`#F94447` / `#EC1A1D` / `#F94447`) and
yellow eyes (`#F7C600`) in unbound. The wordmark retains its ARC gradient in normal
modes and the existing red gradient in unbound; red prompt/selection and explicit
mode labels remain. Other startup accents use ARC blue. Approval text, exact
command/workspace/account authority, render-before-arm and deny behavior are
unchanged. This corrects the initial redesign's removal of the mascot; the
wordmark is an addition to the original identity, not a replacement.

NO_COLOR removes color, backgrounds and dim modifiers. Capable monochrome
terminals retain mascot geometry and lettering where space permits. TERM=dumb and redirected/non-TTY output retain
the compact plain/canonical fallback and existing stream roles. Keyboard,
multiline/paste, menus, cancellation, context uncertainty, streaming, sessions,
private continuation and traces retain their existing behavior.

## Validation and visual evidence

- `cargo test --locked -- --test-threads=1`: **268 passed, 0 failed, 1 ignored**. The ignored test is
  the existing optional recording-overhead experiment.
- `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and
  `git diff --check`: passed.
- Console regressions verify the exact original mascot geometry in side-by-side,
  stacked and compact layouts, teal/yellow eyes, original arch colors and the
  unchanged wordmark gradient. Other coverage includes all unbound bands, monochrome
  styles, one-cell layouts, short append headers,
  composer growth, menus, and welcome reflow. Existing stream/approval/session/
  cancellation regressions pass. New direct-composer PTYs cover colored and
  NO_COLOR resize, draft retention, selection dismissal, small launch/enlargement
  and restored terminal modes. The direct-composer PTY checks exact mascot geometry
  and eye colors after enlargement in normal, unbound and monochrome modes. Real
  shell fixtures remain local. The restoration follow-up made no credential,
  cloud or model requests.
- The first mascot-restoration full run stopped at the unchanged
  `observation_never_runs_fsmonitor_or_changes_index` test because Git metadata
  was unavailable (the observer has a one-second Git deadline). Its isolated
  rerun passed without code changes. Initial log:
  `/tmp/astrid-tui-mascot-test-initial.log`. Final full rerun uses one test thread.
- Sandbox-only runs could not open canonical `/dev/tty`; the existing approval
  regression and full suite passed with local terminal-device access enabled.
- During development, a new conversation-based PTY test accidentally selected
  `/model` while typing `/mode`, triggering a live OpenAI model-catalog request.
  No inference ran; no InstaBox key was accessed. The test was replaced with a
  direct-composer fixture with synthetic choices and no auth/catalog dispatch
  path. Final regression runs use that fixture. This incident is not live-model
  acceptance evidence.

Inspected offscreen Menlo PNG projections of actual VT100 PTY cells and Ratatui
TestBackend cells: 80×24 welcome, 40×24 and 24×12 adaptations, unbound, NO_COLOR,
and a 24×12 launch enlarged to 80×24 with its draft intact. The restored side-by-side
mascot/wordmark, 120×30 wide layout and 40×32 stacked layout were visually inspected.
Also generated
post-startup composer and legacy execution samples. The PNGs are cell-buffer
visualizations, **not Terminal.app screenshots**. No manual Terminal.app/tmux,
light-theme, cross-platform or live-inference acceptance is claimed. Font cell
metrics/true-color support and terminal themes can affect the final appearance;
narrow footers truncate secondary metadata while mode/context remain first.

Local artifacts: `/tmp/astrid-tui-mascot-preview/` contains PNG projections, structured
cells, text and native `.ansi` captures. `/tmp/astrid-tui-mascot-test.log` records the full
suite. Reproduce native captures without model/catalog calls:

```sh
ASTRID_UI_CAPTURE_DIR=/tmp/astrid-tui-mascot-preview \
  cargo test --locked --bin astrid console:: -- --test-threads=1
cat /tmp/astrid-tui-mascot-preview/pty-restored-welcome_color.ansi
```

Launch the working-tree implementation:

```sh
cargo run --locked --
NO_COLOR=1 cargo run --locked --
ASTRID_FIXED_VIEWPORT=1 cargo run --locked -- run "your task"
```

The last command uses the existing one-shot execution surface and runs a real
task when submitted; it was not used for visual verification. Bare interactive
startup can load the model catalog when no model is configured. These are launch
instructions, not claims of offline/live-provider acceptance. Use the mock PTY
command above for offline presentation inspection.

Solo diff review: production changes are confined to `src/console*`; no runtime,
provider, tool, permission, storage or dependency change. Unrelated untracked user
assets, private sessions and recovery files are preserved. No files staged.

## Current publication review — 2026-10-18

Read the referenced MonoCode session's latest exchanges to verify scope: redesign
the terminal welcome/composer and then restore the original mascot alongside the
wordmark. The session was idle. No session was restarted, altered or deleted.
Reviewed all production/test changes and re-inspected the saved normal/unbound
native-cell PNG projections. The exact original mascot and its permission palette
remain visible beside the gradient wordmark; no Terminal.app screenshot or live
inference claim is added. UI source bytes match the reviewed work.

Only nine `src/console*` files, this review, validation evidence and a scoped
CURRENT checkpoint belong to the new PR. `.env`, private histories, local diagnostic
reports, ignore changes, screenshots/reference assets, build output and other user
files are excluded. Main already contains the reviewed InstaBox work, so this diff
must contain no runtime/provider/tool/permission/session or InstaBox source changes.

The staged tracked tree is exported as plain files and tested separately, without
relying on untracked source files or credentials. Updated exact-source results
supersede no historical evidence and are recorded in
[publication validation](../evaluations/tui-redesign-validation-2026-10-18.json).

Final isolated export checks: **276 Rust passed, 0 failed, 1 ignored**, across
25 suites; **2 Python passed**. Formatting, strict all-target Clippy and source
hash comparison passed. Tests used mocks/PTY fixtures, without cloud credentials
or live VM commands. Only documentation changed after these source checks.

Publication: [PR #18](https://github.com/404khai/astrid/pull/18), targeting
`main` after PR #17 merge `5d423a7d`; implementation commit `8ddc037c`.
The PR diff contains exactly the twelve scoped presentation/evidence files.
GitHub checks and merge are the remaining publication steps at this checkpoint.
