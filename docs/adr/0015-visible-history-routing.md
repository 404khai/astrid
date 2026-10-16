# ADR 0015 — Explicit visible-history model handoff

Status: Accepted — 2026-10-15. One-shot/inspection-only restriction superseded
2026-10-16 by [ADR 0016](0016-persistent-routing-provenance.md); visible-history
handoff and privacy/authority contracts remain binding.
Implementation: Implemented; deterministic invariants and final full checks passed
Decision evidence: Maintainer selected “Approve explicit visible-history handoff”
after the context/options/recommendation/consequences question in this thread.

## Context and options

Private Responses continuation currently replays completed provider output verbatim.
Its cross-model compatibility is not a runtime guarantee. Pinning a model for a
whole run preserves continuity but cannot meet Phase 7's heterogeneous-run goal.
The accepted alternative is an explicit visible-history handoff at stage boundaries.
No model classifier, worker, external harness or universal provider abstraction is needed.

## Contract

- Deterministic configured stages select direct inference models. A stage changes
  only between completed model/tool exchanges, never inside a tool batch. Initially
  use explicit call-number boundaries; selection is configuration, not a claimed
  semantic understanding of task type.
- On a model change, prepare a separate view of user messages, visible assistant
  text, complete typed tool requests and corresponding terminal results. Exclude
  all provider-private reasoning/continuation fields from that view. No tool is
  executed again. Keep original session messages/ledger/private continuation intact.
- When multiple stages are configured, use visible-history replay for the entire
  run, including the first request. Returning to an earlier model still uses visible
  history, preventing opaque continuation from other stages from entering its
  request. Routing disabled or effort-only configuration preserves legacy replay.
- Context selection and exact adapter preflight measure the same replay view and
  requested effort that will be dispatched. Preserve whole exchanges, original
  task/latest-exchange protection and existing budgets/compaction labels.
- Record requested and effective request model/effort, provider, route reason,
  stage/class and replay mode per model call. “Effective” means validated dispatch
  configuration; server-reported identity/effort remains separate and unavailable
  unless exposed. Omitted effort is provider default, not an invented medium.
- Unsupported explicit effort fails closed before authentication/dispatch; do not
  silently downgrade, retry on another model or spend extra inference calls.
- Existing permissions, cancellation, private idle snapshot schema and metadata
  trace privacy remain binding. No permission grant follows from a route or history.

## Consequences

Hidden reasoning continuity is lost for routed runs. Visible history may still
contain sensitive task data; it goes to the configured existing provider under
the same user authorization. This is direct model inference, not delegation to
Codex or another coding-agent harness. No harness adapter is authorized/needed.

Legacy sessions need no migration. Per-run routing configuration is not persisted
in their schema. The accepted refinement below prohibits routed resumption.
A response without tools still terminates the run; no extra stage runs for show.

## Upstream evidence checked 2026-10-15

[Reasoning guide](https://developers.openai.com/api/docs/guides/reasoning) documents
model-dependent effort and family-dependent reasoning reuse. This contract chooses
a stricter explicit portable history view rather than relying on opaque compatibility.
[Sol](https://developers.openai.com/api/docs/models/gpt-5.6-sol) and
[Terra](https://developers.openai.com/api/docs/models/gpt-5.6-terra) document supported
efforts; provider availability/authentication remains a separate runtime outcome.
No API price is applied to subscription usage.

## Compatibility refinement — explicitly accepted 2026-10-15

Maintainer chose “Approve one-shot routing; routed history inspection-only” after
the concrete mixed-private-history persistence problem was presented. This narrows
the initial draft's contemplated routed-session follow-up before final acceptance:

- Multi-stage routing is accepted only by fresh `runtime::run`, never
  `run_in_session`, including an empty session supplied to that continuation API.
- Mark its returned Session inspection-only in memory; private messages/ledger
  remain inspectable without conversion or deletion. Do not serialize that marker
  or change the existing snapshot version/fields. Reject whole-Session serialization,
  snapshot persistence and both routed/native continuation explicitly.
- The existing store must retain its prior snapshot after a rejected routed save.
  A routed result cannot be round-tripped through normal serialization to silently
  lose the guard. Existing single-model/effort-only sessions remain serializable
  and resumable under the established private continuation contract.
- Resumable routing is deferred pending a separately accepted source-model
  provenance and versioned persistence contract. No recovery guarantee is inferred.

This is bounded one-shot model routing, not a migration of legacy sessions. Trusted
library callers still have raw public messages for inspection; manually reconstructing
sessions outside the validated APIs does not establish safe replay provenance.

Any fresh run configured with multiple stages is inspection-only, even if it stops
before a later stage is used. This conservative guard avoids outcome-dependent
storage eligibility. Legacy single-model continuation behavior is preserved; its
lack of persistent source-model provenance is not a new guarantee that manually
changing a legacy session's model can safely replay arbitrary private state.
