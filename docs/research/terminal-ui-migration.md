# Terminal UI migration audit and plan

Date: 2026-10-08. Status: implementation authorized by the maintainer's
instruction “implement @terminal-ui-migration.md”; implementation and validation
record below. Audit baseline: `8d765c2` plus the existing worktree (including the local
provider-label edit in `console.rs`). Scope remains Phase 3; no inspection
dashboard, new runtime semantics, or phase transition.

Recommendation: migrate incrementally to Ratatui + Crossterm. Terminal input and
cell rendering are infrastructure commodities; Astrid should continue owning its
event projection, transcript policy, permission adapter, and identity. Begin with
the composer, leaving run output and approvals unchanged until inline streaming
has acceptance evidence. Keeping the existing append renderer throughout gives a
small first slice and a reliable fallback. No separate frontend or new crate.

## Current behavior and guarantees

| Area | Audited implementation |
| --- | --- |
| Identity | `console.rs:87–188,715–733`: pixel logo, blue/turquoise palette, model/provider, cwd, AGENTS.md presence, tool list, version; adaptive header. Provider label is hardcoded; tool definitions are read directly for startup metadata. |
| Default output | `Console::new/emit`: ordinary scrollback, model text on stdout; tool output, lifecycle, metadata, diffs, and optional context inspection on stderr. Colors respect terminal detection, TERM and NO_COLOR. |
| Opt-in viewport | `ASTRID_FIXED_VIEWPORT`: main-screen clear, pinned header/footer, ANSI scroll margins, 2,000 wrapped history rows. No alternate screen. Resize checked every 150 ms, suppressed while canonical approval input is echoed. |
| Input | Composer temporarily disables canonical input, echo and signals with termios; manually parses bytes/UTF-8/arrows. Enter submits; Ctrl-C interrupts; empty Ctrl-D exits/cancels selection; Backspace deletes; Tab completes commands; arrows wrap menu selection; Esc clears input/cancels model selection. `/model`, `/help`, `/quit`, and `/exit` alias remain CLI behavior. |
| Streaming | Model deltas display provisionally; completion adds spacing, failure/cancellation marks interruption. Tool bytes use per-call/per-stream incremental decoders, stream labels, final omission counters, and duplicate-output suppression. Completed tool lifecycle remains distinct from shell exit/task success. |
| Permissions | CLI implements `PermissionHandler` with channels/oneshot replies. Input begins only after `PermissionRequested` is rendered. Full command/cwd/authority are shown; oversized fixed-view approvals revert to append output. Canonical `/dev/tty` input accepts case-insensitive `yes`; unavailable tty denies. |
| Lifecycle | Run-time Ctrl-C calls `Cancellation`; rendering failure requests CLI cancellation and detaches the event receiver. Final queued events are drained. Drop guards restore composer termios and fixed-view margins/cursor. |

The renderer already consumes `ExecutionEvent`, but formatting, presentation
state, layout, input, terminal ownership, and theme share a 1,494-line file.
Composer width is captured once; input is append/backspace only, with no multiline
or bracketed-paste handling. Character widths do not establish grapheme-correct
editing. Fixed-view history stores already wrapped rows, so widening cannot fully
reflow original logical lines. Bare startup checks stdin alone and can emit cursor
controls to redirected stderr or TERM=dumb. These are migration targets, not
reasons to alter runtime behavior.

`main.rs` owns provider creation/catalog retrieval and orchestration. Keep those
outside rendering. `runtime.rs` exposes ordered bounded events, cancellation,
`PermissionHandler`, and `RunResult`; `events.rs` provides correlated lifecycle,
text/output, context and workspace evidence. Preserve [ADR 0002](../adr/0002-phase-1-runtime-and-event-model.md),
[ADR 0003](../adr/0003-bounded-tool-output.md), and [ADR 0004](../adr/0004-execution-authority.md).
Presentation state never becomes authoritative execution state.

## Inline fit and compatibility risks

Upstream reviewed: Ratatui **0.30.2**, Crossterm **0.29.0**, ratatui-textarea
**0.9.3**. Resolve and lock a compatible dependency graph during implementation;
textarea's manifest uses the Ratatui 0.30 component family and Rust 1.86 minimum.
Local Rust is 1.96.0. Libraries declare MIT licensing; integrate as UI dependencies,
without copying their implementation. Sources: [Ratatui manifest](https://github.com/ratatui/ratatui/blob/main/Cargo.toml),
[Crossterm manifest](https://github.com/crossterm-rs/crossterm/blob/master/Cargo.toml),
[Textarea manifest](https://github.com/ratatui/ratatui-textarea/blob/main/Cargo.toml).

`Viewport::Inline` reserves a live region; `Terminal::insert_before` appends rows
above it into scrollback. Draw handles size changes. Manual construction permits
explicit mode ownership; avoid initialization that enters an alternate screen.
Start with the portable insertion path, evaluating the optional scrolling-regions
path separately. [Ratatui terminal documentation](https://docs.rs/ratatui/0.30.2/ratatui/struct.Terminal.html).

Use a one-time logo/metadata header and a small live region for status, composer
or approval, and unfinished output. Insert completed display rows once; render
the unfinished tail immediately, including output without newlines. Flush the
tail at stream switches, completion, interruption and exit. Retain logical text
for the live tail so resize reflows it without duplicating transcript rows.

Compatibility constraints:

- Inline does not reproduce the opt-in pinned top header. Preserve startup
  identity in scrollback; retain the legacy viewport during rollout. Do not retire
  its environment switch silently. A future full-screen adapter may reuse state
  and widgets, but is deferred and would use application-managed history.
- Raw mode removes terminal-driver Ctrl-C, echo and line editing. Handle Ctrl-C
  as an application action calling the existing cancellation handle; keep OS
  signal handling too. Replace canonical approval reading only when one input
  owner handles the entire interactive run. [Crossterm raw-mode behavior](https://docs.rs/crossterm/0.29.0/crossterm/terminal/index.html).
- Textarea fits multiline editing and soft wrapping, but defaults conflict:
  Enter inserts a newline, Ctrl-C copies, Ctrl-D deletes, arrows move the cursor.
  Intercept Astrid actions/menu keys first. Keep Enter sends; propose Ctrl-J for
  newline, support multiline paste, and never depend on Shift-Enter being distinct.
  Paste must not submit tasks or approve commands. [Textarea key mappings](https://docs.rs/ratatui-textarea/0.9.3/ratatui_textarea/).
- One owner must write all interactive terminal output; independent stdout/stderr
  writes would invalidate cursor bookkeeping. When either output is redirected,
  use the append renderer, preserving the stdout/model and stderr/diagnostic split.
  Do not enable raw mode/ANSI UI for pipes, dumb terminals, or unusable sizes.
  Keep `/dev/tty` approvals independent of redirected streams and deny without it.
- Native terminal selection/scrollback must remain usable: no default mouse
  capture. Terminal/font Unicode widths still vary; test combining marks, CJK,
  emoji/ZWJ and split deltas. Already inserted rows belong to terminal scrollback
  and cannot be retrospectively restyled/reflowed by the application.

## Smallest incremental implementation

1. **Extract without changing behavior.** Keep `clap` and one Cargo package.
   Split CLI modules into input/actions, application/presentation state,
   layout, widgets, theme, terminal guard and append renderer. Pass startup
   metadata as a DTO from the CLI; renderer imports no providers/tool executors.
   Add injected output sinks and event-to-presentation tests.
2. **Replace only startup input.** Use Crossterm and inline Ratatui with textarea
   behind the composer interface. Preserve logo, notices, command/model menus
   and shortcuts. Restore terminal before starting the existing run path.
3. **Prove inline streaming, then integrate.** One async CLI loop consumes runtime
   events, input, resize and render ticks. Drain events promptly; batch only drawing
   at an initial 30 Hz, with immediate permission/terminal flushes. Bound pending
   presentation work, preserve event order, and yield between batches. Measure
   burst responsiveness instead of claiming a speedup. No runtime channel changes.
4. **Unify approvals and enable default inline.** Route decisions through the
   existing `PermissionHandler` adapter. Accept `yes` only after full command,
   cwd and authority have been displayed; oversized commands use scrollback.
   Restore raw mode, paste mode, cursor and owned region on success, errors,
   cancellation and panic. Flush transcript and leave shell below the UI.
   Switch defaults only after acceptance; defer full-screen inspection.

## Validation gates

Baseline: `cargo test --locked --bin astrid` passed **12 tests**. Existing coverage
checks formatting, widths, viewport wrapping, composer-region clearing and
fragmented/cancellable permission reads. It does not prove actual termios cleanup,
input decoding, or ordered event rendering in a PTY. No live-model/UI acceptance
was performed for this audit.

Add deterministic action tests for all preserved shortcuts, menu routing,
multiline/paste, Unicode editing and resize. Use Ratatui TestBackend for layouts
and event sequences: partial/interrupted model text, split tool UTF-8, stream
switches, omission counters, context/diffs and final-event draining. Snapshot
plain stdout/stderr with redirection, NO_COLOR, TERM=dumb and tiny dimensions.
Use child-process PTY tests for raw-mode restoration, panic/setup failures,
Ctrl-C during output/approval, approval render-before-input, denied/absent tty,
and shell process cleanup. Test scrollback insertion and long no-newline output
in an emulator/real terminal, not only a cell buffer. Replay burst fixtures and
measure input/render latency and bounded pending memory.

Before enabling default inline, run the full test suite, formatting and Clippy
checks from `docs/development.md`, plus macOS terminal and a second terminal/
tmux acceptance pass. The maintainer subsequently authorized this UI migration;
that authorization does not advance Phase 3.

## Implementation record

All four slices are implemented in one Rust package. `src/console.rs` is now a
CLI adapter over `console/{state,input,composer,layout,widgets,theme,identity,
format,terminal,inline,append,legacy}.rs`. Runtime, agent, provider, tools, events,
permissions, and context implementations are unchanged. `clap` remains in use.
Ratatui 0.30.2, Crossterm 0.29.0 and ratatui-textarea 0.9.3 are locked; vt100
0.16.2 is a test-only emulator dependency.

Default inline rendering preserves the startup logo/palette/metadata and
transcript scrollback. Full-screen inspection is deferred. The existing fixed
viewport remains available. The textarea adapter preserves Astrid's command,
menu, cancellation and exit actions; Ctrl-J adds multiline entry. Plain navigation
and deletion use grapheme boundaries. Input has a 64 KiB ceiling with a notice;
oversized paste is rejected as a whole. Layout uses `Frame::area`, including tiny
dimensions and resize during approvals.

The CLI uses nonblocking Crossterm `poll/read` on one task instead of EventStream:
10 ms polling avoids introducing a background terminal reader, while 33 ms draws
batch visual updates. Runtime event transport remains ordered and lossless.
Inline approvals use the same permission relay after the request is rendered;
queued typeahead is discarded before arming, and paste is never an approval.
Fallback output preserves canonical `/dev/tty` approvals and stdout/stderr roles.

Validation includes event projection, output formatting/deduplication, split UTF-8,
multiline/paste/menus, grapheme editing, tiny layouts, colors, bounded bursts and
live-tail resize. Child-process macOS PTYs run the actual CLI drive loop with a
mock provider and real shell tools: approval/denial, cancellation during model
streaming/permission/shell, shell-leader cleanup, canonical redirected approvals,
broken pipes, absent tty, composer resize, panic and setup failure. A VT100 parser
responds to cursor queries and verifies that cleanup leaves the primary screen
and cursor visible. Terminal modes are checkpointed after guards unwind but before
child exit, since macOS invalidates slave ioctls after session exit; kernel-managed
PENDIN is excluded from the local-mode comparison.

These PTY/emulator checks are the automated acceptance gate used for default inline.
A manual Terminal.app/tmux visual pass and live model acceptance remain unperformed.
Terminal.app access was attempted but the computer-use tool disallows that app;
no GUI workaround was attempted. No compatibility or latency claim beyond the
tested paths is made. Solo review:
the renderer imports domain event/result types but no provider/tool implementation;
startup metadata is assembled in `main.rs`, and cancellation/permission semantics
remain those of the accepted ADRs. Existing unrelated worktree edits are preserved.

Final validation on macOS with Rust 1.96.0, 2026-10-08:

- `cargo test --locked`: **128 passed**, including 30 CLI tests.
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `git diff --check`: passed; core execution modules have no diff.
- The TestBackend burst fixture rendered 600 rows plus 50,000 no-newline bytes
  in **117.495 ms**, preserving every row/byte and keeping pending text below
  25 KiB at the assertions. This measures virtual-backend processing, not terminal
  I/O throughput or an improvement against the previous renderer.
- PTY cancellation tests passed their two-second request-to-clean-exit ceiling.
