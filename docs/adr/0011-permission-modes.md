# ADR 0011: Permission modes and client display

Date: 2026-10-08
Status: Accepted
Implementation: Implemented; deterministic validation passed
Related: ADR 0004 execution authority; ADR 0010 interactive sessions
Decision evidence: Maintainer explicitly requested switchable ask/auto/unbound
modes, full access for unbound, visible mode, and its logo palette on 2026-10-08.

## Context

Astrid already evaluates typed read/write/execute policies per run. The client
needs named modes without hiding effective authority or coupling runtime
execution to a terminal. Continuous conversation also needs a clear boundary
for applying mode changes.

## Decision

Use a typed `PermissionMode` that maps to the existing `PermissionPolicy`:

| Mode | Read | Write | Execute |
| --- | --- | --- | --- |
| ask | allow | ask | ask |
| auto (default) | allow | allow | ask |
| unbound | allow | allow | allow |

`auto` preserves ADR 0004's defaults. Read access remains allowed in `ask`;
mutations and broad shell authority require a decision. `unbound` selects all
existing capabilities as allow; it does not remove path validation, timeouts,
output limits, or cancellation. General shell execution retains broad account
authority; no network/destructive command classifier or sandbox is introduced.

`/mode` opens a selector, and `/mode <name>` switches directly. Commands are
available between runs. The client snapshots the selected policy into each new
RunConfig; runtime policy/lifecycle events continue to expose effective authority.
Session switching does not change mode. Mode is not persisted or repository
controlled. Starting another process defaults to auto.

One-shot `--mode` selects a preset. Explicit per-capability CLI flags override
that capability. The displayed mode is inferred from the effective policy;
unmatched combinations display `custom`, never a misleading preset name.

The CLI displays mode in metadata, input footer, and execution status. In unbound,
the logo's upper/middle/lower arch bands use #F94447/#EC1A1D/#F94447 and its eyes
use #F7C600. Color remains client behavior; NO_COLOR and redirected output retain
plain rendering.

## Alternatives considered

- Separate mode-specific executors: duplicates the existing permission boundary.
- Auto-approving supposedly safe shell strings: cannot enforce the advertised
  boundary and conflicts with ADR 0004.
- Removing workspace checks in unbound: unnecessary to grant the existing full
  tool authority and would discard native tool guarantees.
- Remembering modes across launches: silently restores account-level authority;
  excluded from this scoped request.

## Consequences and compatibility

Library callers can continue constructing custom PermissionPolicy values. CLI
defaults and explicit allow/ask/deny flags retain their effective behavior.
Switching a mode itself dispatches no tools or model calls. This extends client
configuration within Phase 3; it authorizes no phase transition or persistence.

## Validation

Preset tests execute real native writes and shell calls with denying approval:
ask blocks writes/shell, auto permits writes but blocks shell, unbound permits
both without asking. Traversal still fails under unbound. Header/palette tests
cover exact RGB values and disabled color. A PTY fixture exercises direct mode
selection during continued conversation and session switching. Existing tests
cover policy events, cancellation races, missing handlers, and deny precedence.

## Resolution

Accepted as implementation of the explicit maintainer request on 2026-10-08.
Local preset mapping preserves existing defaults and executor guarantees.
