# BUG-256: Resume retains an obsolete transcript generation

## Review context

- Reviewed commit: `144653b0dc12ca096e835e86b620b109b29019bf` (`more fixes`).
- Review priority: P2 — an affected open tab stops displaying subsequent content.
- Evidence: static inspection of paging, lifecycle, and resume state; no interactive reproduction was executed during review.

## Problem and impact

A transcript cursor is bound to one server generation. Page failures leave the
assembler cursor, partial payload, and pinned snapshot intact. Once a cursor
exists, `/resume` also retains that state. If the server generation has changed,
the client repeatedly requests pages from the obsolete generation and cannot
recover using the action recommended by its own error notice.

An ordinarily delivered session-clear event already resets client state. The
problem concerns missed lifecycle events and archive-seeding races, not every
normal `/clear` operation.

## Source evidence

Locations refer to the reviewed commit:

- `rust/fspec-tui/src/app/dispatch_transcript.rs:193` reports page errors without resetting generation-bound hydration state.
- `rust/fspec-tui/src/app/dispatch_transcript.rs:242` tells the user to retry with `/resume` or reopen the tab.
- `rust/fspec-tui/src/app/dispatch_transcript.rs:272` changes suspension based on cursor presence without reinitializing an existing assembler.
- `rust/fspec-tui/src/app/dispatch_transcript.rs:144` resumes paging after success using the retained state.
- `rust/sessions/src/transcript_journal.rs:60` changes the generation when installing an archive.
- `rust/sessions/src/transcript_journal.rs:178` rejects a cursor whose generation differs from the journal.

## Reproduction to automate

1. Hydrate a TUI tab far enough to install a transcript cursor.
2. Disconnect its WebSocket so it cannot receive lifecycle notifications.
3. Clear the same session from another client, replacing the server generation.
4. Reconnect the original client and select the session through `/resume`.
5. Submit new content and observe repeated generation failures instead of new transcript content.

Also cover a first page accepted before concurrent archive installation changes
the generation. Closing and reopening the tab currently provides a workaround.

## Suggested repair direction

Keep generation mismatch distinguishable from transient transport failures.
On the appropriate recovery path, reset the assembler, snapshot boundary,
partial payload, and in-flight request state together, then reconcile rendered
content with the new generation. Do not indiscriminately append a new snapshot
to old scrollback, which would duplicate history. Reject stale responses from
the old hydration attempt.

## Acceptance and validation guidance

- `/resume` recovers from a missed clear without reopening the tab.
- Archive installation during hydration does not permanently wedge delivery.
- New-generation content appears exactly once in order.
- Old-generation partial bytes and late responses cannot contaminate recovery.
- Ordinary reconnects with an unchanged generation retain deduplication.

Related specifications: `spec/features/lossless-transcript-delivery.feature`,
`spec/features/lossless-transcript-tui-hydration.feature`, and
`spec/features/slash-command-resume.feature`.
