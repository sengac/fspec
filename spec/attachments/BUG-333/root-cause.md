# BUG-333 root cause: destroy_session drops the session from list_sessions and resume

## Symptom

`cargo test --profile ci-test -p codelet-sessions --test rpc422_session_persistence` fails:

```
test destroy_session_removes_from_memory_preserves_manifest ... FAILED
panicked at sessions/tests/rpc422_session_persistence.rs:199:5:
session should still appear in list_sessions via persisted merge
```

The other 11 tests of the target pass. In the product this means: close a session in the TUI and it
disappears from `/resume` in the same daemon until the daemon restarts, although its manifest is
still on disk.

## Evidence

- `rust/sessions/src/session_manager/manager_impl_6.rs:75` — BUG-288 added
  `codelet_core::persistence::release_session_ownership(uuid)` to `destroy_session`.
- `rust/core/src/persistence/manifest_delete.rs:19-25` — `release_session_ownership` calls
  `SessionStore::evict`, and `evict` (lines 6-13) removes the lease **and** `cache[id]` **and** the
  project's `last_session` pointer.
- `rust/core/src/persistence/manifest_store.rs:122-132` — `list_for_project` / `list_all` read only
  `self.cache`; the cache is filled from disk once, in `SessionStore::new → load_all` (lines 21-55).
- `rust/core/src/persistence/manifest_state.rs:57-79` — `list_sessions_for_project` /
  `list_all_sessions` serve `SessionManager::list_sessions`
  (`rust/sessions/src/session_manager/manager_impl_2.rs:34-52`) from that cache.
- `rust/core/src/persistence/manifest_store.rs:108-119` — `resume_last` needs `last_session` + cache.

So after BUG-288 the destroyed session is no longer in the cache, and the persisted merge of
`list_sessions` cannot see it. The test is right (feature `session-persistence-integration`,
scenario "Session destruction removes from memory but preserves manifest" requires it); the product
is wrong.

## Why the eviction was not needed

BUG-288's rule is only that the **writer lease** (the OS lock on `sessions/<id>.lock`) is released.
The cached manifest holds no lock and is not used for writing: `commit`/`update`
(`manifest_commit.rs:92-121`) re-acquire the lease through `writer_lease` (lines 55-90) and
`update` re-reads the committed manifest from disk. Keeping the cached manifest therefore cannot
block another owner and cannot make a later write stale.

## Fix

`release_session_ownership` drops only the lease: a new `SessionStore::release_lease(id)` removes
`leases[id]`; `evict` (lease + cache + last-session) stays for the delete path
(`delete_current_session`, manifest_delete.rs:57), which is the operation that removes a session.

## Alternatives rejected

- **Change the rpc422 test to expect the session to vanish** — contradicts the accepted scenario
  and the `/resume` behaviour (a closed session must stay resumable).
- **Make `list_*` re-scan the sessions directory on every call** — a larger behaviour change of the
  store (cross-process cache coherence) that this bug does not need; out of scope.
- **Re-insert the manifest into the cache after eviction** — reads it back from disk only to undo
  an eviction that should not happen.

## Wire / API changes

None on the wire. `release_session_ownership` keeps its signature; its doc now says it releases
only the writer lease.
