# CU-01 — Screenshot observation

Integration successor update, 2026-10-21: [accepted ADR 0024](../adr/0024-foreground-computer-use-integration.md)
now authorizes real foreground observation/action/runtime and separate ephemeral
model disclosure. [Integration milestone](foreground-computer-use.md) records the
implemented code and mock evidence; live trials remain separately gated. Earlier
scope/publication statements below are historical to this initial slice.

Date: 2026-10-21. Authorization: requested new branch/thread, parent-created
`feat/screenshot-observation` from fetched main, same preserved checkout.
Scope: concrete design, public unauthenticated API audit and synthetic fixture/mock
work; publish conventional commits and an unmerged PR against main. Previous
computer-use exclusions are superseded only for this slice. No new thread or
branch is created by this implementation task.

## Contract and exit

[ADR 0022](../adr/0022-screenshot-observation.md) defines separate exact-call
observe authority, one ephemeral frame, explicit privacy/cleanup, finite
byte/pixel/time/count limits and zero provider image disclosure/context. It is
Accepted after the maintainer explicitly chose “Accept ADR 0022 with its gates”;
this is not image persistence, runtime integration or disclosure permission. [Audit/review](../reviews/screenshot-observation-2026-10-21.md) pins
public OpenAPI and SDK evidence, including the missing direct screenshot route.

Design/fixture exit: independently exercise denial/binding/single use, bounds,
safe restricted synthetic PNG decode, replacement/expiry/cancellation cleanup,
staleness and unavailable image-token accounting without a real machine or model.
Full bounded screenshot/status runtime observation remains gated on runtime integration under accepted
authority, supported transport and production decoder, then separately scoped live
authorization. Fixtures cannot establish live evidence or complete that exit.

No click/type/scroll/key/open or action loop, provisioning, lifecycle, ports,
credentials/secret transfer, remote workers, Host/mobile networking, Phase 9,
daemon, PTY or desktop client implementation. Private v2 history/continuation,
original results, complete exchanges, context admission and ADRs 0008/0009/0021
are preserved. Rust owns the harness; Meister primary, Astrix specialized.

Stop after this slice and its PR. Next plan item CU-02 remains future and requires
action, coordinate/freshness and provider-image contracts; do not start it. CU-01
runtime and RE-02 live gates remain pending independently.


## Validation and disposition

Accepted contract committed as `1579a889` before the fixture implementation commit.
Sixteen CU-01 Python synthetic tests plus two existing comparison tests pass;
**308 Rust passed, 0 failed, 1 ignored**, fmt/strict all-target Clippy/whitespace
passed. [Review/limits](../reviews/screenshot-observation-2026-10-21.md),
[source evidence](../evaluations/screenshot-observation-2026-10-21.json).
Only design/fixture exit is satisfied. No runtime tool, transport, production PNG
decoder, image session persistence or model image input was installed. Full CU-01
runtime/live acceptance remains pending; no CU-02 work began.

Publication: contract `1579a889` and fixtures `11971bf8` are published in
[PR #24](https://github.com/404khai/astrid/pull/24) against main, **open and unmerged**.
No PR was merged by this task. Only the design/fixture slice is delivered;
runtime/live gates remain pending and CU-02 has not begun.
