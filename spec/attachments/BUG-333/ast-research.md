# BUG-333 AST research

Queries run with `fspec astgrep` to find every caller of the ownership release, every cache eviction and every cache reader.

## fspec astgrep --pattern 'codelet_core::persistence::release_session_ownership($$$A)' --lang rust --path rust/sessions/src
rust/sessions/src/session_manager/manager_impl_6.rs:75:13:codelet_core::persistence::release_session_ownership(uuid)


## fspec astgrep --pattern 'store.evict($$$A)' --lang rust --path rust/core/src/persistence
rust/core/src/persistence/manifest_delete.rs:22:13:store.evict(session_id)
rust/core/src/persistence/manifest_delete.rs:57:9:store.evict(session_id)


## fspec astgrep --pattern 'self.cache.values()' --lang rust --path rust/core/src/persistence
rust/core/src/persistence/manifest_store.rs:123:9:self.cache
rust/core/src/persistence/manifest_store.rs:131:9:self.cache.values()


## fspec astgrep --pattern 'self.cache.remove($$$A)' --lang rust --path rust/core/src/persistence
rust/core/src/persistence/manifest_delete.rs:8:32:self.cache.remove(&session_id)


## fspec astgrep --pattern 'self.leases.remove($$$A)' --lang rust --path rust/core/src/persistence
rust/core/src/persistence/manifest_delete.rs:7:9:self.leases.remove(&session_id)


## Findings

- `release_session_ownership` has exactly one caller: `destroy_session` (manager_impl_6.rs:75).
- `SessionStore::evict` has two callers: `release_session_ownership` (manifest_delete.rs:22) and
  `delete_current_session` (manifest_delete.rs:57). Only the delete path should drop the cached manifest.
- The cache is the only source for `list_for_project` (manifest_store.rs:123) and `list_all`
  (manifest_store.rs:131); `resume_last` reads `last_session` + `cache` (manifest_store.rs:108-119).
- Writes do not depend on the cache: `commit`/`update` re-acquire the lease through `writer_lease`
  (manifest_commit.rs:55-90) and `update` re-reads the manifest from disk (manifest_commit.rs:113).
