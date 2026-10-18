# InstaBox CLI completion and merge review — 2026-10-18

The maintainer requested completing and merging InstaBox PR #17, then publishing
and merging the terminal redesign from the named MonoCode session. This completes
only the optional standalone CLI integration. It does not introduce a runtime/model
remote tool, scheduler worker, automatic provisioning or phase advancement.

## Completed scope

- `astrid instabox discover --allow-discovery [--machine <exact-id>]`: bounded
  metadata GETs and usage/limits; no VM connection or implicit discovery grant.
- Local key setup: a non-empty `INSTABOX_API_KEY` environment value wins; otherwise
  read that assignment from the invocation directory's `.env`. No parent search,
  expansion, environment mutation or unrelated credential-store access. A restricted
  `machines:read` key suffices for discovery; explicit exec needs `machines:write`.
  `.env` stays ignored and was not inspected or included in source exports.
- `astrid instabox exec --machine <exact-id> --command <exact-command>
  [--cwd /data/home] [--timeout-seconds 60] --allow-execution`: separate explicit
  grant, compared against the exact typed request. No machine-name resolution,
  provisioning, file/env/stdin transfer, port/desktop/snapshot/Vault operation or
  automatic retry. Existing local shell permission presets do not grant this
  operation implicitly; existing broad approved shell authority is not a sandbox.
  The flag is a local caller grant, not a new cross-device principal/trust model.

No live exec command was run. The command above contains placeholders and is
reference syntax, not an instruction to send a request during review.

## Review findings resolved

The interrupted local executor was not merge-ready. Current public
[OpenAPI](https://api.instabox.sh/v1/openapi.json), fetched without authentication,
confirms the request is `cmd`, `cwd`, `timeoutSeconds`, with maximum 600 seconds
and default HOME `/data/home`. Corrected those fields/defaults and updated mocks
to assert the external wire contract, not the interrupted implementation's fields.
Rejected empty/whitespace/NUL commands, invalid IDs, non-absolute/NUL cwd and invalid
bounds before dispatch. The existing borrowed-temporary compile fix is retained.

An explicit metadata GET must return the exact selected ID and `running` state;
all other states fail before POST. Metadata wait is bounded to 15 seconds. The
command POST/body wait is bounded to the requested provider timeout plus 5 seconds.
These are local admission/wait bounds, not a guarantee the remote process stopped.
InstaBox can wake idle-paused machines if state changes after the status check;
`--allow-execution` help explicitly includes that possibility. No strict no-wake
promise is made. The user must approve that possibility for any later live trial.

JSON execution reports retain machine attribution, total elapsed time, separate
stdout/stderr, actual exit status, provider timeout and truncation flags. All wire
completion fields are required. Nonzero/provider-timeout results remain observed
command outcomes, not transport errors. The CLI exits unsuccessfully for nonzero,
timeout, cancellation or failure after printing its JSON report.

On post-admission cancellation, local deadline, HTTP/transport failure, malformed
or oversized response, keep the machine and categorical reason, report remote
outcome **unknown** and output unavailable; do not invent output/exit status or
claim termination. Ctrl-C signals the owned execution future and waits for its
phase-aware result instead of manufacturing a zero-duration report. No retry.
Response bodies remain bounded to 256 KiB. Larger responses fail explicitly and
cannot establish command completion. No output streaming, background jobs,
reconnect, active recovery, runtime trace/session schema or worker grant changes.

## Acceptance and limits

Fifteen private synthetic HTTP/file tests cover discovery, dotenv precedence and
privacy, actual exec payload, exact grant mismatch, invalid requests, every
non-running state, mismatched machine, no dispatch before grant/pre-cancellation,
nonzero exit, provider timeout, cancellation and deadlines during metadata and POST
bodies, auth/scope/not-found/quota/unavailable errors, malformed/oversized responses,
missing exit status, attribution/unknown output and no retries. Actual CLI preflight
uses isolated HOME/cwd and synthetic values; it checks default denial and rejects
601-second timeouts without contacting a server. No live credentials or account
entitlements were verified. Runtime cancellation/tracing guarantees remain unchanged
because this management command is outside agent runs.

This is a reversible private adapter and explicit invocation surface under the
maintainer's scoped request. Broader execution-target authority, remote model tools,
trace attribution/persistence and scheduler integration remain future proposed
contracts, rather than inferred ADR acceptance. Original research and evidence
remain linked from [the investigation](../research/instabox-discovery.md).

## Publication isolation and validation

InstaBox source/tests and documentation are staged separately from all console
redesign files. The PR is checked in a plain exported tracked-source directory,
not a worktree and without `.env`, private sessions, apps/build/user assets or the
untracked TUI wordmark. Its full checks and source hashes are recorded in
[completion evidence](../evaluations/instabox-completion-2026-10-18.json).
Prior combined-checkout counts remain historical and are not reused as isolated
PR evidence. The terminal redesign receives its own subsequent PR and review.

Final isolated-tree results: **271 Rust passed, 0 failed, 1 ignored; 2 Python
passed**. Format, strict all-target Clippy and whitespace passed. The ignored
case is the existing optional recording-overhead experiment. No current TUI source
or unpublished wordmark was present in the exported tree. Only documentation and
result metadata changed after these checks; Rust source hashes remain exact.
