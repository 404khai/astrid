# ADR 0016 — Persistent dispatch provenance and resumable visible-history routing

Status: Accepted — 2026-10-16
Implementation: Implemented after meter/comparison slices; final checks passed.
See [follow-up review](../reviews/phase-7-followups.md).
Decision evidence: Maintainer chose “Approve v2 provenance and visible-history
migration” after the concrete compatibility question in this thread.
Supersedes ADR 0015's one-shot/inspection-only persistence restriction and ADR
0012's v1-only storage format. Other privacy, authority and idle-boundary clauses
remain binding. Earlier acceptance/history is preserved in those ADRs.

## Context and alternatives

One-shot routing retained mixed private responses without source provenance.
Resuming them through native replay could send private state to another model.
Keeping one-shot routing avoids migration but cannot meet the requested follow-up.
Recording provenance while replaying opaque state across models would invent a
compatibility guarantee. The accepted alternative stores dispatch provenance and
uses a permanent visible-history view whenever provenance/model continuity is lost.

## Decision

- Snapshot v2 records each committed response's dispatch provider identity, model
  and effective dispatch effort (equal to the request because unsupported effort
  fails closed), keyed by the existing model-call ID in the
  context ledger. This is dispatch configuration, not server-resolved identity or
  an invented default. Omitted effort remains unknown/provider-default.
- Responses with missing provenance remain explicitly unverified (absence in the
  known-origin map). Origin entries must refer to actual assistant ledger items;
  invalid/foreign IDs and invalid labels are rejected. No provenance is fabricated
  for v1 or manually assembled history. Unknown source forces visible replay.
- Store a durable session replay mode. Same provider/model native history with
  known origins retains private continuation. Routed or model/provider-changing
  submissions transition to visible history, and that mode never returns to native
  through normal APIs, even with routing disabled or after a restart. Effort changes
  alone do not assert a different model identity.
- Visible replay includes visible text and complete typed tool exchanges only;
  original private response bytes remain stored/inspectable. No tool side effect
  is replayed. Existing incomplete-batch continuation rejection remains.
- V1 files load conservatively with unknown origins and visible replay for prior
  assistant history. Empty/user-only legacy history needs no private handoff.
  Loading does not write. The next successful atomic idle save writes v2. Pre-publication
  validation/serialization/save failures retain the existing snapshot. The existing
  ADR 0012 post-publication sync-error contract remains: the new snapshot may
  already be published and durability is reported uncertain, not rolled back. V1 binaries reject
  v2 explicitly; there is no downgrade writer or silent destructive conversion.
- V2 requires explicit replay/provenance fields. Reject malformed/partial metadata,
  unsupported versions, workspace mismatches and existing store bounds violations.
  Unknown providers lacking visible-history support reject a required handoff before
  mutating the caller's session or authenticating/dispatching inference.
- Per-call route events record why the replay view is native, configured visible,
  migrated/unverified, changed-model or inherited visible history. Keep model
  selection reasons distinct from replay reasons.
- Policies, permissions, routing file contents and active jobs are not persisted.
  Interactive `/routing <file>` or `/routing off` controls subsequent runs through
  the same typed per-run policy. Model/session switches grant no new authority.

## Consequences and validation

Resumable routing becomes an idle-session capability, not crash recovery or a
worker/scheduler. Legacy hidden-reasoning continuity is deliberately lost on the
first follow-up, while private bytes survive for inspection. The change requires
newer Astrid to read saved v2 state. Local stored data is trusted under the existing
private filesystem/account boundary; no cross-device attestation is claimed.

Required evidence: routed save/reopen/follow-up excludes opaque state and preserves
private originals; native same-source restart retains continuation; native model
change stays visible after disabling routing; v1 load/save migration preserves
history and old file on failure; invalid origins/partial metadata fail closed;
incomplete batches cannot resume; permissions and event replay remain intact.
No Phase 8, background execution, harness adapter or desktop work is authorized.
