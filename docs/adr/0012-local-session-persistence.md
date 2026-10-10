# ADR 0012: Local session persistence

Date: 2026-10-09
Status: Accepted
Implementation: Implemented; deterministic validation passed
Decision evidence: Maintainer instruction on 2026-10-09: "Sessions should not be temporary, and the session name should be derived from the first chat sent for that session".
Related: ADR 0010 ephemeral sessions; ADR 0009 metadata-only traces

## Context

The maintainer reports that conversations disappear when Astrid exits and is
reopened in the same or another terminal. This is the explicit ADR 0010 behavior,
not a restoration bug. Metadata traces cannot restore provider-valid history.

## Decision

Supersede only ADR 0010's in-memory restriction with local, workspace-scoped
session snapshots. Keep Phase 4 active; do not introduce background execution,
cross-device synchronization, task recovery, or tool replay.

- Store versioned snapshots in private user storage outside the repository,
  partitioned by canonical workspace identity. Restore the session list, labels,
  active selection, run counts, last outcomes, original conversation, context
  provenance, and provider continuation needed for subsequent model requests.
- This stores conversation and tool contents, including potentially sensitive
  repository data and provider-private continuation. No credentials or permission
  grants are serialized. This is a separate store from ADR 0009 metadata traces;
  observability on/off does not control session persistence.
- Atomically save at idle boundaries: accepted session creation/selection and
  after run termination. Saving failure is reported separately from run outcome;
  never imply durability if a snapshot failed. Crash recovery is limited to the
  last successfully saved idle snapshot.
- Validate schema, workspace identity, ledger, and conversation history before
  restoration or submission. Preserve incomplete terminal tool batches for
  inspection, but keep existing rejection of continuation; never replay tools or
  manufacture missing outcomes.
- Bound files/session counts and reject unsupported/corrupt/oversized stores
  explicitly without overwriting or silently deleting them. Initial numeric
  limits remain local implementation choices.
- One active writer per workspace store, enforced by a local lock. Another
  terminal reports that the store is in use rather than racing its writes.
- Headless session snapshot APIs contain no terminal or stdin dependencies.
  The client owns storage selection and idle-boundary saves.

## Alternatives considered

1. Keep temporary sessions and clarify the UI: smallest scope, but does not meet
   the reported expectation of restart continuity.
2. Store visible transcripts only: useful history, but insufficient to resume
   provider tool exchanges and private continuation faithfully.
3. Persist full executable sessions (recommended): more retention and validation
   work, but preserves the existing headless follow-up contract.

## Consequences and compatibility

This introduces persistent content retention and a versioned state contract.
Earlier releases have no session store to migrate. Existing traces retain their
metadata-only semantics. Permissions and repository instructions are recomputed
for each run; opening a stored session grants no authority.

Required evidence: restart restores multiple sessions and the active selection;
follow-up preserves IDs/history without replay; separate workspaces stay isolated;
concurrent writers, corrupt versions, symlinks, bounds, and failed saves report
clear errors; interrupted tool batches remain non-continuable.

## Resolution

Accepted by the maintainer's explicit persistence instruction on 2026-10-09.
Session names are derived deterministically from the first submitted chat, with
whitespace normalized and an 80-character display bound. The active SessionId
remains compact in the input footer so notices do not disappear behind long names.
Initial storage ceilings are 8 MiB per session and 32 MiB per workspace snapshot,
with at most 32 sessions. Exceeding them reports a failed save and retains the
previous snapshot; a post-publication directory sync failure reports uncertain
durability. Historical sessions lost before this feature cannot be recovered.


## Validation

`tests/session_store.rs` covers private restart snapshots, writer locks, workspace
isolation, rejected corruption/versions/provenance, symlinks, byte limits and failed
saves, provider completion validation, no replay of a real file write, and stored
incomplete batches that cannot resume. The PTY restart test creates two sessions,
exits/reopens, resumes the selected history, checks first-message names, and switches
to the other restored session. Existing session/context/permission tests remain green.

Passed: `cargo test --locked --test session_store --test sessions`,
`cargo test --locked --bin astrid`, `cargo test --locked`,
`cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings`.
