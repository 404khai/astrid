# Foreground computer-use integration

Date: 2026-10-21. The maintainer requested actual CU integration, then explicitly
accepted [ADR 0024](../adr/0024-foreground-computer-use-integration.md) option 1.
Acceptance was committed as `390bddec` before dependent code. Branch
`feat/foreground-computer-use` is stacked on open PR #25 (`4d10f91f`), same preserved
checkout, no worktree. Neither PR is authorized to merge.

## Implemented

- Invocation-only `--computer-use --instabox-machine <exact-ID>` on a fresh
  foreground `run`; no resume, routing, notes, worker or interactive installation.
- Separate observation, action and per-frame named-provider image approval.
  Observation covers only exact-target running-state preflight plus reviewed
  status/capture; local shell presets convey none of these grants.
- Fixed documented exec/status HTTP payloads, capped responses and no retry,
  redirect, lifecycle or desktop startup. Typed inputs map to fixed xdotool plans;
  typing goes through stdin. Missing tools fail without installation/fallback.
- Piped PNG24 capture, bounded CRC/framing and maintained PNG decoding in a
  killable/reaped helper, validated native geometry, stripped metadata and real
  controller receipt installation. No guest/host screenshot file.
- One canonical frame, <=10 seconds from capture admission. Native coordinates,
  unchanged 5-second action freshness, two screenshots/actions and finite limits.
  Stale/denied/uncertain operations stop without refresh, retries or continuation.
- One original-detail image in only the next explicitly approved supported OpenAI
  request. Real request-byte admission plus separate documented image estimate;
  existing budgets do not grow. Image-only actual usage/cost remains unavailable.
- Request-local nonserializable image input; no image Message, persisted image,
  private image continuation or replay. Original outputs/provenance retained in
  existing permanent visible-history mode. Numeric transient context admission,
  metadata-only persistent traces and observability-off semantics preserved.

## Validation and remaining gate

[Review/evidence](../reviews/foreground-computer-use-2026-10-21.md) records tests
actually executed. Mocked HTTP and synthetic images demonstrate the application
path; they do not establish provider account access, installed guest tools, real
geometry, latency, UI effect or live success. RE-02 live validation is independently
pending. No authenticated API, machine or inference operation was performed.

The remaining operational step is a separately authorized exact existing machine,
scratch task, capture/disclosure/action limits and selected supported provider/model.
See the [usage and trial checklist](../guides/foreground-computer-use.md). A
practical freshness/approval UX beyond experimental five seconds requires separate
review. Unsupported capture tools/large PNGs fail closed. No mobile, Host, worker,
daemon, PTY, desktop client, image persistence or successor implementation is part
of this slice. Stop after its reviewable PR.
