# Message durability design and verification

## Ownership and scope

BUG-247 owns `rust/core/src/persistence/messages.rs`, its write/index/maintenance/
recovery helpers, new durability/framing helpers and dedicated integration tests.
Only narrow operation observation in `manifest_commit.rs` is needed to verify the
existing manifest barriers. No schema change or major refactor is proposed.
BUG-243 retains originals/session replay/provider checkpoint ownership. Agent-loop,
sessions, and CLI/provider sources are outside this implementation scope.

## Acknowledgement boundary

On Unix, with an already durable data root on a local filesystem honoring
`File::sync_all` and atomic same-directory rename, a successful message append
means JSONL data and the containing directory have been synchronized before its
ID is returned. The parent of the messages directory is also synchronized, so
creation of the messages directory is ordered before acknowledgement. The
manifest writer cannot obtain a successful message ID before those barriers.
The existing manifest protocol stages, syncs, renames, and syncs its directory.

For blob-backed content the order is blob file sync, blob prefix/blobs/data-root
directory sync, message append/file/directory sync, then manifest staging/file
sync/rename/directory sync. BUG-243's separate provider-state checkpoint references
an already committed manifest prefix under its session writer lease; its own
file-sync/rename/directory-sync protocol is not a new manifest transaction. This
card does not claim atomic conversation-plus-provider-checkpoint acknowledgement.
The owning checkpoint route must reject incompatible source revisions/generations;
the coordinator audits that cross-card integration separately.

The binary index is derived. Losing it must not lose acknowledged content. Index
replacement is itself synchronized, but a failure aborts the caller before any
new manifest reference. Durable orphan bytes may remain and are recoverable.
This does not claim that all metadata updates are atomic with message appends.

Cleanup writes a complete staged replacement and synchronizes it. It invalidates
and durably removes the derived index before replacing the data file: even equal
file sizes cannot make old offsets look valid after interrupted replacement.
After rename, errors explicitly report promotion; no rollback of the new data
file is attempted. The in-memory generation/cache is invalidated so retry reads
the authoritative replacement. Before rename the old JSONL remains authoritative.

The guarantee depends on filesystem/device compliance with synchronization. It
is not a claim of macOS hardware-cache `F_FULLFSYNC`, network-filesystem support,
or transactional survival under device failure. Unsupported directory-sync
semantics fail rather than acknowledge a durability guarantee.

## Stable identity prerequisite audit

The independent completion audit added stable-identity acceptance scenarios and
`rust/core/tests/bug247_stable_message_identity.rs` before implementing the missing
`MessageStore::store_with_id` API. The initial command was:

`cargo test --profile release-slim -j 2 -p codelet-core --test bug247_stable_message_identity`

`/tmp/bug247-stable-red.log` records E0599 at `conversation_commit.rs:20`, so this
is compilation RED, not a claimed behavioral failure. The primitive is now in
`messages/write.rs`: stable-sidecar exclusion, index reconciliation, strict
on-disk payload comparison, conflict rejection, and repeat message/file-directory
synchronization without another append. Role, content and metadata must match;
retries preserve the original stored timestamp. The conversation helper remains
owned by BUG-246 and associates the identity once under the session writer.

BUG-243 owner `44135c59-e9f1-4233-a3db-31d97e8ebc4e`, journal owner
`bc2c91d4-3bec-46f2-b35e-1a32328d3022`, and parent message attempts returned
`session_busy`, so this contract also records the shared-file handoff. No other
owner's core files were edited in this audit. Passing behavioral and quality
results, subsequent successful owner notifications, and exact commands are
recorded in `completion-audit.md`; API compilation alone is not the evidence.

## Test seams

Tests use shared-helper temporary directories and real MessageStore/SessionStore
operations. A thread-local, opt-in observer injects an I/O error immediately
before a named operation; events are recorded only after the real operation
succeeds. There is no global environment toggle or live-store mutation. Tests
verify barrier order, pre/post-promotion distinctions, index rebuild, retained
cleanup content, and concurrent writers. This is deterministic filesystem fault
testing, not a model of a disk controller dropping already acknowledged writes.

## Opt-in genuine crash procedure

1. Use an expendable VM or physical test host with an isolated data root on the
   filesystem under evaluation. Never use the user's `~/.fspec` or project store.
2. Run a workload appending uniquely numbered Unicode records and committing
   manifest references. Send success acknowledgements to a separate controller
   outside that machine, including exact record IDs/content hashes/revisions.
3. Randomly cut VM power or physical power from the controller. Do not substitute
   process exit, SIGKILL, normal reboot, or filesystem unmount for this step.
4. Cold boot, retain a forensic copy, remove only the derived index in the copy,
   and rebuild. Verify every externally acknowledged reference resolves to the
   exact content/hash. Failed calls may leave orphans or promoted manifests, but
   no success may reference missing bytes. Repeat during cleanup and recovery.
5. Record OS, filesystem/mount options, hypervisor cache settings, storage device,
   synchronization semantics, controller acknowledgements, and trial count.

These destructive trials have not been run here. Ordinary reopen integration
tests check consistency and recoverability, not power-loss persistence.
