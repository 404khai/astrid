# Foreground InstaBox CLI approvals — 2026-10-21

RE-02 uses parent-created `feat/instabox-cli-approvals` from RE-01 `2f8f9358`,
same checkout, no worktree. Latest maintainer direction authorizes CLI connection,
mocked preparation and a concrete live plan, superseding RE-01's stop restriction
only for this slice. [ADR 0021](../adr/0021-remote-target-authority.md) is unchanged.
No exact live machine or action scope was supplied; live acceptance is pending.

Read-only fetch/recheck found [PR #22](https://github.com/404khai/astrid/pull/22)
already merged upstream at `56880270`. Its dependency source matches this branch's
baseline. Publish against main; no PR was merged by this work.

## Implementation and solo review

`astrid --instabox-machine <exact-m_ID>` and
`astrid run 'task' --model <model> --instabox-machine <exact-m_ID>` explicitly
install a foreground target for this invocation only. No environment/default
machine, inventory lookup, name resolution or saved target. Omission installs
nothing and does not consult the InstaBox key loader. Reject this flag for notes,
batches/workers and other subcommands before loading remote credentials/provider
setup. The existing standalone discovery/exec commands remain separate.

The binary loads the existing local key source only after valid explicit opt-in;
no key is loaded by the library or inserted into model messages/tool arguments.
The host asserts the effective local uid as principal. The immutable exact target
and opaque client feed the existing adapter. Selection is not a remote grant and
sends no metadata request. Model inference is a separate network operation, as
with existing CLI runs, and requires separate authorization in our live trial.

Each run installs a fresh terminal broker using the existing foreground answer
transport. The broker explicitly returns an exact single-use grant only after
`Ok(true)`, valid for 60 seconds from approval. Denial, UI error/closure and dropped
pending answers grant nothing. Local permission presets/handlers never mint a
remote grant. Sharing a terminal input channel does not share authority. Runtime
binding, cancellation, expiry and atomic consumption remain in RE-01. No retry,
replay, automatic budget increase or remote-worker installation.

Interactive `/new` and session switching keep the explicit invocation target,
but each run gets a new answer channel and each call a fresh grant. No target,
broker or permission is restored from session storage; restarting without the
flag disables remote execution. Private v2 history/provenance/continuation and
incomplete-exchange rejection are unchanged. Existing broad local shell authority
still is not an OS sandbox.

Remote approval displays complete command/cwd as JSON strings, preserving exact
control bytes as visible escapes, plus target, principal, run/runtime/provider call,
timeout and the possible pause/auto-wake race. The terminal labels remote authority
rather than suggesting local workspace path containment. Existing fresh-input
arming/paste rejection and cancellable nonblocking terminal reading remain intact.

The foreground result display now shows private target/run/runtime/provider-call
attribution, POST admission and elapsed time, actual separate stdout/stderr when
observed, and the existing actual exit/timeout/truncation summary. Dispatched
cancellation explicitly shows unknown outcome, unconfirmed termination and no
automatic retry. Unobserved output/exit is labeled unavailable. Exact attribution
stays in private content-bearing events/results/terminal display; the metadata
trace schema/projection is unchanged and still omits targets, commands, paths,
outputs, arbitrary errors and credentials. Observability off preserves mandatory
approvals/cancellation/admission without added telemetry or trace writes.

Solo review examined opt-in/default paths, notes/worker ceilings, fresh per-run
channels, stale answer disposal, full display/control escaping, cancellation
projection, credential separation and the main-based diff. Two display findings
were fixed: remote approvals previously inherited a workspace-containment label,
and cancelled remote reports did not display their retained unknown outcome.
No consequential authority/schema decision or new dependency is introduced.

## Deterministic evidence

**308 Rust passed, 0 failed, 1 ignored; 2 Python passed; formatting, strict all-target Clippy and whitespace passed.**

Final results are recorded in
[the evidence file](../evaluations/instabox-cli-approvals-2026-10-21.json).
New checks cover valid/invalid opt-in and unsupported scope rejection before key
loading, fresh/denied/closed broker replies, disposal of cancelled approval answers,
exact JSON-escaped review bytes, full remote authority display, observed separate
output/exit and private attribution, and cancellation/unknown-output rendering.

The unchanged RE-01 loopback/runtime suites remain the dispatch-boundary evidence:
all local presets without a grant, denial with zero authenticated requests,
cancellation during approval, after metadata/before POST and after admission,
expiry, exact binding/single consumption, timeout/uncertain transport/malformed
outcomes, actual output, attribution, trace privacy and incomplete-batch rejection.
CLI broker tests exercise answer transport; they do not pretend to be a live HTTP
trial or a new complete end-to-end CLI/network test. Provider fixtures and remote
HTTP fixtures are deterministic local mocks; no authenticated public request,
provider inference or live VM command was performed.

One preflight assertion initially expected the wrong existing key-error wording;
it was corrected to match the existing fixed error. No credential text was exposed.

## Concrete live trial awaiting separate authorization

**Do not execute this plan until the maintainer supplies an exact isolated existing
machine ID and explicitly approves the scope below, including a provider/model.**
Do not discover the machine from .env, keys, account inventory or private sessions.
The target should contain no production accounts or secrets. Reconfirm the public
unauthenticated API documentation before the authorized trial; stop if its contract
has changed. No authenticated discovery is needed: approval permits only the
exact-target running-state preflight required by the adapter.

Proposed bound: **five foreground one-shot invocations, at most five provider
inference requests, three approved exact-target metadata GETs and three admitted
exec POSTs, with zero retries**. Each invocation uses `--max-model-calls 1`,
`--observability off`, `--mode ask --read-policy deny --write-policy deny
--shell-policy deny`, and the same explicit `--instabox-machine <approved-ID>`.
A successful tool execution intentionally ends at the model-call ceiling: no
follow-up inference or claim of overall task success. Deny every unexpected or
additional tool request; it has no live authorization. Stop on unexpected state,
ID/argument mismatch, unavailable service or unknown outcome; do not continue to
the next trial after uncertainty.

All calls use exact cwd `/data/home`. Command strings below use literal backslash-n
inside shell single quotes, as displayed and bound by the approval:

| Order | Exact command string | Provider timeout | Decision / evidence |
| --- | --- | --- | --- |
| 1 | `printf 'astrid-re02-ok\n'; printf 'astrid-re02-stderr\n' >&2; pwd` | 5s | Approve once; inspect actual separate stdout/stderr, exit, exact private target/run/call attribution and POST admission. |
| 2 | `printf 'astrid-re02-denied\n'` | 5s | Deny at remote approval; expect permission denial, no authenticated metadata/POST and no output claim. |
| 3 | `sleep 3; printf 'astrid-re02-timeout\n'` | 1s | Approve once; retain actual provider timeout/exit only if observed. A local deadline/malformed/transport result stays unknown and ends the trial. |
| 4 | `printf 'astrid-re02-approval-cancelled\n'` | 5s | Ctrl-C while approval is pending, without answering yes; no remote dispatch. |
| 5 | `sleep 3; printf 'astrid-re02-cancelled\n'` | 5s | Approve once, Ctrl-C after approval while execution is pending. Inspect retained POST-admission evidence: if admitted, unknown remote outcome; otherwise not dispatched. Do not repeat to force admission. |

Each prompt requests exactly its table tuple, with no other tool use. Review the
actual broker tuple before answering; model compliance is not assumed. The host
has a 15s metadata wait, 5s connect bound and a total POST/body wait of provider
timeout + 5s, with 256 KiB responses. Cancellation is local observation cancellation;
it sends no lifecycle/kill request and never proves remote process termination.
The last remote sleep can continue to its provider deadline after Ctrl-C. If
cancellation wins before admission, post-dispatch live coverage remains unproven.
No live run is repeated to make evidence look complete.

Preflight permits only an already-running exact ID. It is point-in-time: a pause
between GET and POST can still cause provider auto-wake and charges. Approval must
include this race. A non-running machine stops the trial; never start/resume it.
No creation/lifecycle/ports/secret transfer, screenshots/computer use, remote workers,
Host/mobile networking, Phase 9, daemon, PTY or desktop operations.

Record a sanitized manual evidence table with counts, observed classifications and
which checks remain unproven, keeping exact target/call attribution in private
foreground evidence only. Do not publish exact IDs, prompts/commands beyond these
fixed fixtures, output paths, credentials, arbitrary server errors or session data
in metadata traces/repository evidence. Inspect only newly authorized trial data;
preexisting private sessions remain untouched. No grant carries beyond this trial.

## Disposition

CLI connection and deterministic preparation are reviewable. The isolated live
trial and its RE-02 exit criteria remain pending exact machine/provider/action
authorization. Stop after publishing the unmerged PR and reporting this limit;
no successor implementation or claim that mocked checks prove live behavior.
