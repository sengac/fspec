# BUG-247: Establish durable message-before-manifest ordering

## Priority and evidence

**P1 — source-confirmed system-crash durability gap; no power-loss reproduction is claimed.**
JSONL appends are not synchronized before session manifests are durably promoted.
Index publication flushes and renames without synchronizing its data/directory.
A power/system crash can therefore preserve acknowledged manifest references without
their message bytes. Ordinary process exit or SIGKILL does not necessarily lose completed writes.

## Code to inspect

- `rust/core/src/persistence/messages/write.rs:74`: append file; line 84 writes without data synchronization.
- `rust/core/src/persistence/messages/index/write.rs:43`: flush and rename, without a durability barrier.
- `rust/core/src/persistence/manifest_updates.rs:62`: couples stored-message creation and manifest reference update.
- `rust/core/src/persistence/manifest_commit.rs:181`: manifest promotion synchronizes file and parent directory.
- `rust/core/src/persistence/messages/maintenance.rs:153`: cleanup replacement also flushes/renames without synchronization.

## Required repair

1. Document precisely what a successful conversation commit acknowledges under supported filesystems.
   Define the durable order of message/blob data, references, checkpoint state and session manifest.
2. Ensure message data required by an acknowledged manifest survives before promoting that manifest.
   Include directory synchronization when creating/replacing files, not just flushing userspace buffers.
3. Decide whether the binary index is derived and rebuildable or a durability participant. A stale or
   absent derived index must recover from durable framed JSONL without discarding acknowledged messages.
4. Make cleanup/compaction replacement durable before deleting the last recoverable old state.
   Preserve cross-process writer exclusion and avoid destructive partially acknowledged replacements.
5. Propagate synchronization errors with correct pre-promotion versus post-promotion semantics;
   a late acknowledgement failure must not roll back a manifest already committed on disk.
6. Consider safe batching/group commit only with explicit acknowledgement boundaries and measured
   behavior. Do not improve throughput by silently weakening the stated durability guarantee.

## Regression scenarios and verification

- Verify the filesystem operation sequence: data write/sync precedes durable reference promotion.
- Inject failures at message sync, index write/rename, manifest stage sync, manifest rename, and directory sync.
- Close/reopen after each acknowledged or failed boundary; observe a complete old or committed state,
  never a successfully acknowledged manifest referencing unavailable message content.
- Replace JSONL during cleanup with failures before and after promotion; retain all referenced records.
- Run concurrent writers and rebuild a stale/missing index without incorrect rollback or duplicate references.
- Provide an opt-in crash/power-loss harness or documented manual procedure for environments able to test
  genuine crash persistence. Ordinary tempdir reopen tests do not prove power-loss behavior.

Use shared filesystem fixtures and a narrow injectable operation abstraction for deterministic ordering
tests where necessary. Keep platform guarantees explicit; unsupported semantics must return typed errors,
not claim durability. Do not run destructive crash experiments against `~/.fspec`.

## Boundaries and related work

BUG-246 owns retaining failed conversation content and retry ownership. BUG-248 owns torn-tail
framing recovery. This card owns durability ordering, including cleanup, without broad storage redesign.
Coordinate with BUG-243's checkpoint storage contract before changing persistence schemas.

## Completion evidence

- [ ] Commit ordering and durability acknowledgement are documented and asserted by fault tests.
- [ ] Cleanup, creation, replacement, and post-promotion error semantics are covered.
- [ ] Derived-index rebuild and concurrent-writer behavior remain correct.
- [ ] Scoped tests/check/clippy/format and scenario coverage pass; crash-test limits are honestly recorded.

Investigation: `docs/investigations/2026-10-23-compaction-and-resume.md`.
