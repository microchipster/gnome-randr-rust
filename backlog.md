# Backlog Workflow

This repo tracks implementation work in two places:

- `docs/unaddressed/`: active backlog items
- `docs/addressed/`: completed items and historical context

## The simple model

- `unaddressed` is the queue
- `addressed` is the record of what actually landed
- each note should describe one concrete task or one honest umbrella that is still being split apart

If a note is still too large to implement cleanly, do not keep stretching it. Split it into smaller follow-up notes and leave the larger note as routing context.

## How to work in `docs/unaddressed/`

- put active work in `docs/unaddressed/notes/`
- use numeric prefixes to suggest execution order
- keep notes implementation-oriented, not just research dumps
- prefer bounded slices that can be shipped and verified
- if a note becomes outdated, rewrite or reroute it instead of letting it drift

When you pick up a task:

1. Read the note and any linked follow-ups.
2. Check whether it is still true.
3. Implement the smallest honest slice.
4. Update any docs or routing notes that depend on it.

## How to move work to `docs/addressed/`

Move a note to `docs/addressed/notes/` when the work is actually done, not when it is partly done or merely investigated.

Keep the same filename when moving it.

Every addressed note should end with these sections:

- `## How This Was Addressed`
- `## How To Exercise And Test It`

Those sections should include:

- concrete code pointers
- a short explanation of the real fix path
- practical commands or manual steps to verify the behavior

## When to split a note

Split a note when:

- it contains multiple unrelated tasks
- one part shipped but other parts are still open
- the remaining work belongs to different owners or different subsystems
- the note is acting like an umbrella instead of a next task

In that case:

- move the shipped part to `addressed` if appropriate
- create smaller new notes in `unaddressed`
- update cross-references so the next developer can follow the trail

## Practical rules for contributors

- do not leave completed work in `unaddressed`
- do not move a note to `addressed` unless the behavior is really shipped
- do not hide unfinished work inside an addressed note; create a follow-up note instead
- prefer a truthful backlog over a tidy-looking backlog
- if you touch backlog routing, update nearby notes that point to the old path

## Good outcome

A new developer should be able to:

- open `docs/unaddressed/` and find the next real task
- open `docs/addressed/` and understand what already landed
- follow links between them without guessing what is still active
