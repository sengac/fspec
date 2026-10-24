# Session-owned transcript admission and recovery retirement contract

## Bounded core ownership

No tombstone, new retry store, schema, or agent-loop production change is proposed.
The existing committed session manifest is the authoritative lifecycle boundary.
The existing `persistence-recovery/<session>.lock` is the stable recovery lease.

`JournalStore::configured(session).append(record)` will acquire the same core
session writer lease/gate as manifest updates/deletion, read the committed
manifest under that lease, and append before releasing it. It will not use a
check-then-append existence test or the global SessionStore cache. Missing or
unreadable manifests fail before journal directory creation. Existing low-level
`JournalStore::new(root, session)` remains the explicitly unbound storage primitive
used by isolated storage tests and already-owned core operations; there is no
fallback to it after an ownership failure.

Core will expose `persistence::session_lifecycle::RecoveryLease`:

- `acquire(data_root, session_id) -> io::Result<RecoveryLease>` acquires the existing
  stable sidecar nonblockingly, with private directory/file permissions.
- `configured(session_id) -> Result<RecoveryLease, ManifestCommitError>` resolves
  the configured root once and acquires the same lease.
- `directory() -> &Path` identifies the existing per-session recovery lane.
- `session_exists() -> Result<bool, ManifestCommitError>` reads and validates the
  committed manifest while the caller holds this recovery lease. Only NotFound
  means deleted; malformed/unreadable state is an error and retains originals.
- `retire() -> io::Result<()>` removes the durable recovery lane and synchronizes
  its parent. It does not access agent-loop OWNERS or take the session writer.

## Lock ordering and deletion

Order: agent-loop OWNERS (when applicable) -> recovery sidecar -> global
SessionStore mutex (facade only) -> session writer -> transcript writer.
Configured transcript append itself takes only session writer -> transcript
writer, so retry holding a recovery lease does not acquire another recovery lock.
Core deletion never takes the agent-loop OWNERS mutex.

The canonical deletion facade acquires the recovery lease BEFORE touching the
global SessionStore mutex. Direct SessionStore deletion acquires it before its
writer transaction. A helper accepts the already-held lease so the facade does
not double-lock. Busy recovery ownership returns an explicit retryable error
without waiting under the session mutex or deleting data.

Deletion preflights private-lane node types before changing committed state.
Manifest removal is the commit point, synchronized before journal/recovery lane
retirement. This ordering is necessary: removing pending originals while a live
manifest survives a later error would silently lose retryable content. After the
commit point, cleanup failure is explicitly reported as deletion-promoted, the
cache is evicted, private residual bytes remain for retry, and session-owned
append cannot resurrect a transcript. Repeating deletion finishes cleanup for an
already absent manifest. No parallel deletion marker is needed.

## Required BUG-246 owner interface change

Core cannot mutate the backwards-dependent `persistence_retry::OWNERS` registry.
Please replace `persistence_retry_io::lock`'s duplicated File lease with this core
RecoveryLease, preserving the same pathname/order. Under OWNERS + RecoveryLease,
before re-promoting retained/admitted memory, `retry` must inspect session_exists:

- live: retain the existing admission/retry behavior;
- absent: retire the durable lane and remove this session's OWNERS entry without
  republishing its admitted/retained records;
- error: keep exact originals and return a visible retryable error.

Likewise enqueue/submit/retain must distinguish explicit deleted-session retirement
from storage failure, so submit does not re-retain a record after enqueue discovers
deletion. Do not clear another session's owner or turn malformed evidence into
absence. These are interface requests, not authorization for this worker to edit
the other worker's agent-loop production files.

The new cross-crate regression target is
`rust/agent-loop/tests/bug250_recovery_retirement.rs`; it exercises the public
PersistenceSpool, canonical delete facade, retained memory, durable pending records,
and concurrent session ownership. Any remaining memory-retirement RED is reported
to the parent/BUG-246 owner, not waived by core-only GREEN.
