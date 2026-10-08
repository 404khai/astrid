# ADR 0008: Optional observability and settings

Date: 2026-10-08
Status: Accepted
Implementation: Implemented
Related: [Phase 4](../plans/phase-4.md), P4-01; preserves ADR 0002 lifecycle and ADRs 0006/0007 context enforcement
Decision evidence: Maintainer instruction on 2026-10-08: "Implement the observability and its related adrs 8 & 9".

## Context

The maintainer proposed a settings command for turning observability on/off.
Existing events drive authoritative execution transitions and client permissions;
they cannot safely disappear when optional instrumentation is disabled. Persistent
recording would introduce data retention absent from current ephemeral runs.

## Options

1. Default off, opt-in optional telemetry and metadata traces; core events remain.
2. Default on with the same boundaries; easier immediate inspection but new
   retention occurs without an explicit enable action.
3. Disable the whole event stream when off; conflicts with existing lifecycle,
   inspection and permission-client contracts and is not recommended.

## Decision

Use option 1. Add `astrid settings` and `astrid settings set observability on|off`.
Explicit `run --observability on|off` overrides the user setting, then built-in off.
Freeze effective options at submission, expose their source, and reject malformed
settings explicitly. Runtime callers supply options directly; the core does not
read terminal/user configuration. No repository setting can enable recording.

Off disables added optional telemetry collection and trace writes. It preserves
required lifecycle events, streamed text, context admission/accounting, permissions,
workspace evidence and cancellation. Off affects subsequent runs and does not
delete existing traces. Display preferences remain client-owned.

## Consequences and compatibility

Current no-persistence behavior remains the default. This is a new runtime option
and CLI configuration contract, not a change to execution event semantics. It
adds no dynamic run reconfiguration, generic settings framework or cleanup command.
On does not promise full transcript storage; ADR 0009 controls trace content.
Numeric bounds/config path choices are reversible local details. Settings are
atomically replaced in private user storage, separate from credentials.

## Accepted decision question

Accept default-off optional observability with a persistent user setting and
explicit per-run override, while preserving all mandatory runtime events?

## Validation

Isolated config/precedence tests; malformed config fails before dispatch; off
retains context and permission behavior; edits during execution affect only later
runs; a headless caller needs no user config or terminal. No unmeasured performance
claim follows from off/on.

## Resolution

Accepted on 2026-10-08 by the maintainer's explicit implementation instruction.
Implementation and validation are recorded in [the Phase 4 review](../reviews/phase-4.md).
Cost of changing later: Medium (CLI/config defaults
and runtime callers depend on the meaning of off).
