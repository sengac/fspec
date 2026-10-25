# BUG-255: direct implementation review

Reviewer: this same verified codex/gpt-6.1-sol session, without subagents,
additional sessions or model substitution.

## Result

No outstanding BUG-255 implementation defect found. The repair is connected to
the actual recovery append path, not just a standalone journal benchmark.
The executed command/result ledger is in `acdd-validation.md`; no whole-workspace
all-green claim is made.

## Admission and integrity

- `JournalStore` retains a private mutex-protected admission index. Identity
  digests, request/sequence coordinates and tool states grow linearly with
  records; warm admission does not retain or replay large old result payloads.
- Cold reconstruction uses the existing complete path/ordinal/identity/version
  validation plus the shared lifecycle validator. Explicit reads still validate
  the full immutable history and invalidate the writer's warm state.
- Directory and tail stamps include length and modification time, plus Unix
  device/inode/change-time identity. Changed stamps rebuild validation; warm
  corrupt tails and missing interior records are rejected, and another writer's
  valid append is reconstructed before continuing. This follows the specified
  private immutable-history trust boundary, not arbitrary in-place historical
  mutation detection on every delta. Cold and explicit reads check all records.
- The existing filesystem writer lease remains authoritative. Ownership checks
  still wrap configured appends in the existing committed-session transaction;
  that transaction now uses the retained writer instead of constructing another.
- Admission state is taken out of the cache while writing and installed only
  after the existing record, directory and root synchronization boundaries.
  Failed acknowledgement discards warm state; retry reconstructs and verifies
  the exact original durable bytes without duplicating identity or order.
- Projection and incremental admission use the same lifecycle transition checks.
  Missing requests, duplicate starts, cross-attempt results, invalid anchors and
  record identity conflicts retain their rejection behavior.

## Recovery ownership and connection

- `PersistenceSpool` gets an `Arc<Mutex<RecoveryOwner>>` from the registry, then
  releases the registry lock before session lookup, leases, reads or writes.
  All filesystem waits occur under the affected session's lane, not that global
  registry. The filesystem-stall acceptance case proves independent progress.
- The lane retains its configured journal across successful retries. The
  recovery-scaling control repeatedly opens spools and admits 256 actual
  transcript records, so recreating a cold writer for each event cannot pass.
- Idle registry entries can be pruned after 128 lanes without filesystem work.
  Active handles, poisoned/busy lanes and lanes with admitted/retained originals
  are not pruned. The acceptance helper exercises registry pressure, checks that
  an active writer remains warm and rejects 129 deleted-owner admissions.
- Deleted-session retirement clears retained originals and the cached writer.
  Unreadable manifests retain unacknowledged originals rather than inventing
  ownership. Existing recovery, capacity, promotion and deletion tests pass.
- `archive::commit` remains the archive publication entry point; transcript
  events reuse the retained writer, while original-message handling is unchanged.
  BUG-258's output/journal identity, terminal settlement and live diagnostic
  routing are not edited and all seven acceptance cases pass on this code.

## Scope and hygiene

All touched/new Rust files remain below 300 lines. No lint suppression, unsafe
code, panic/unwrap shortcut, fabricated durable event or background execution
engine is introduced. Existing error types, locking order, disk layout and
durable-before-publication behavior are retained. Library check/Clippy and
BUG-255 test Clippy pass without warnings in their recorded outputs.

The only resumed held transition was the single human-authorized normal native
retry; it succeeded. Coverage bookkeeping was repaired through native unlink/link
commands, not a gate bypass or manual JSON edit. The existing original red/hold
record is preserved. BUG-253/256/257/258 remain done, BUG-254 stays specifying;
uncommitted work is preserved, with no commit, push, restart or policy change.

Native finalization accepted **validating → done**, with all evidence and caveats
in the completion reason. The feature carries `@done`, not `@wip`.
