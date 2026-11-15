# ADR 0040 — Personal memory: facts the user asks Astrid to remember

Date: 2026-11-15
Status: **Accepted 2026-11-15 as the agent's call, pending the maintainer's review.**
The maintainer ranked "memory of you" first for making Astrid "more personal and
fluid. think jarvis, baymax" and asked to build it in parallel with the wake word.
The persisted-state and provider-disclosure choices below are the conservative
options. The maintainer should confirm or change them before memory grows.

## Context

Every ask starts from nothing. Astrid can't keep the user's name, preferences or
facts ("I post on X as 404khai", "keep replies short") from one ask to the next.
The ask surface (ADR 0033) and the desktop app (ADR 0037) both run asks through
`astrid presence-ask`, so memory belongs there and in the runtime, not in a client.

## Decision

- **Only what the user asks.** A fact is kept only when the user says so:
  "remember (that) ...", optionally after "hey/yo Astrid". Nothing is inferred
  or proposed by the model in this slice. "Remember to ..." (a reminder) and "do
  you remember ...?" (a question) are not memory requests and go to the model.
- **Recognised without a model call.** `memory::request` matches remember,
  forget ("forget (that|about) ..." removes facts containing all those words),
  "forget everything", and "what do you remember (about me)?". The runtime
  answers directly ("Got it. I'll remember that your name is Khai.").
- **Stored on this Mac only.** `~/Library/Application Support/Astrid/memory.json`
  (`$ASTRID_MEMORY_FILE` overrides it), mode 0600, written atomically: a private
  temp file, then a rename. Version 1 JSON: `{version, next, facts: [{id, text, at}]}`.
  At most 100 facts of at most 300 characters; past 100 the oldest goes. An
  unreadable file is an error, never silently replaced.
- **Shown to the model as context.** Every presence ask's task starts with the
  facts, newest first, at most 6,000 characters, labelled as context about the
  user and never instructions. This discloses them to the ask's model provider
  (and to operator's goal only through the ask model's hand-off). The memory
  file itself is never sent anywhere else, synced or traced.
- **Visible and editable.** `astrid memory list | add <text> | forget <id> | clear`,
  plus the spoken forms above. The file is plain JSON the user can read.

## Alternatives considered

- **Model-proposed memories** ("Want me to remember that?"). This is more fluid,
  but it lets the model decide what gets kept. It is left for the next slice,
  behind a confirm.
- **Retrieval (embeddings or search) instead of sending everything.** That isn't
  needed at 100 short facts, and it would add a subsystem before there's
  evidence for one.
- **Syncing to the phone (ADR 0039).** This is a cross-device trust decision and
  stays out of this slice.

## Consequences

- Astrid can use the user's name and preferences in every ask, typed or spoken.
- Each ask carries up to 6 KB more context.
- Operator sessions and the desktop app get memory through the same ask path.
  Mail tools are unaffected.
