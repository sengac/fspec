@done
@session-management
@rpc
@napi
@rust
@persistence
@refactor
@RPC-033
Feature: Lift SessionStore manifest + load_session + append_message_with_metadata into codelet-core::persistence::manifest
  """
  Layout: create rust/core/src/persistence/manifest.rs as a NEW module holding SessionStore + SessionManifest + the lazy_static singletons (MESSAGE_STORE, SESSION_STORE) + every session-level free function from napi/src/persistence/mod.rs. Update rust/core/src/persistence/mod.rs to add `pub mod manifest;` and the flat re-export `pub use manifest::*;`. The existing rust/core/src/persistence/sessions.rs (RPC-026) is collapsed into manifest.rs (its single delete_session helper becomes a method on the lifted SessionStore facade).
  codelet-core/Cargo.toml gains lazy_static = workspace (already in codelet-napi at version 1.4 — wire as a direct dep here since it is not yet in workspace.dependencies). No other new dependencies — tracing, chrono, serde, serde_json, uuid, codelet-common are already present from RPC-031/RPC-032.
  NAPI shim shrinks from ~780 lines to ~50 lines: rust/napi/src/persistence/storage.rs is DELETED (SessionStore moved to core), rust/napi/src/persistence/types.rs is DELETED (SessionManifest + TokenUsage + MergeRecord + PastedContent + ForkPoint + CompactionState moved to core), rust/napi/src/persistence/mod.rs becomes a thin facade that (a) declares the remaining NAPI-only modules (blob, blob_processing, napi_bindings, tests, lazy_init_tests), (b) keeps the BLOB_STORE lazy_static singleton and store_blob/get_blob/blob_exists/ensure_directories until RPC-034, (c) wraps set_data_directory with the credentials/graph resets while delegating persistence resets to a new codelet_core::persistence::reset_stores_for_tests helper, and (d) re-exports everything else via `pub use codelet_core::persistence::*;`.
  RPC-026 unwinding: the existing rust/core/src/persistence/sessions.rs `delete_session(uuid)` becomes a thin alias for the lifted SessionStore::delete (it stays callable for backward compat with rpc026_cross_transport_parity test, but the canonical delete path is now the SessionStore method). The defensive double-delete in napi::persistence::mod::delete_session is removed — the single call into the lifted facade does the right thing.
  Test coverage strategy: matches RPC-031/RPC-032 — write a `rust/core/tests/session_store_lifted_test.rs` integration test that consumes the lifted types via `codelet_core::persistence::{...}` (proving the public surface is reachable without codelet-napi), plus a `rust/napi/tests/session_store_lift_shim_test.rs` that asserts type identity between `codelet_napi::persistence::SessionManifest` and `codelet_core::persistence::SessionManifest` (compile-time proof the shim re-exports rather than duplicates). The existing 80+ persistence tests in codelet-napi continue to run unchanged.
  Singleton ownership transition: MESSAGE_STORE and SESSION_STORE lazy_static globals move to rust/core/src/persistence/manifest.rs. lazy_init_tests.rs in NAPI currently reaches into MESSAGE_STORE / SESSION_STORE / BLOB_STORE directly via `super::*`; after the lift it switches to test-only accessor functions `codelet_core::persistence::is_message_store_initialized_for_tests()` and `codelet_core::persistence::is_session_store_initialized_for_tests()` (matching the pattern history.rs already uses: `codelet_core::persistence::history::is_initialized_for_tests()`). BLOB_STORE stays in napi until RPC-034 so its initialization-check helper remains in napi's lazy_init_tests.rs.
  Clippy compliance: codelet-core enforces workspace lints (redundant_closure_for_method_calls, needless_collect, etc.) that codelet-napi does not. The lifted code must pass `cargo clippy -p codelet-core --lib --no-deps` cleanly. Expect to need similar fixes to RPC-032 (replace `|v| v.as_u64()` with method references where applicable; drop intermediate Vec collects).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionStore (struct + impl) plus SessionManifest TokenUsage MergeRecord PastedContent ForkPoint CompactionState SessionLineage all live in rust/core/src/persistence/manifest.rs with identical serde derives, rename_all, and field order — no #[napi] attributes added
  #   2. All session-level free functions (create_session, create_session_with_provider, save_session, load_session, resume_last_session, fork_session, merge_messages, cherry_pick, switch_session, delete_session, rename_session, list_sessions, list_sessions_for_project, list_all_sessions, append_message, append_message_with_metadata, update_message_metadata, cleanup_orphaned_messages, get_message, get_session_messages, get_session_messages_full, update_session_tokens, set_session_tokens, set_compaction_state, clear_compaction_state, get_session_lineage) live in codelet-core::persistence (manifest module) and the MESSAGE_STORE + SESSION_STORE lazy_static singletons move with them
  #   3. rust/napi/src/persistence/storage.rs and rust/napi/src/persistence/types.rs are deleted (or reduced to 1-line shims) — every existing crate::persistence::{SessionStore, SessionManifest, TokenUsage, MergeRecord, PastedContent, ForkPoint, CompactionState, SessionLineage} import inside codelet-napi continues to compile via the flat re-export `pub use codelet_core::persistence::*;` in napi/src/persistence/mod.rs
  #   4. On-disk format is byte-identical: session.json files written by the lifted SessionStore use the same serde-pretty layout and field order as the pre-lift NAPI store, and messages.jsonl remains unchanged (already covered by RPC-032's MessageStore lift)
  #   5. SessionStore::new() in the lifted location calls codelet_common::get_data_dir() directly to compute {data_dir}/sessions/ and creates it with std::fs::create_dir_all — it does NOT depend on the NAPI-local ensure_directories helper (matching the RPC-032 pattern for MessageStore)
  #   6. The RPC-026 inverted-dependency (where NAPI's delete_session double-deletes via codelet_core::persistence::delete_session as a defensive idempotent call) is removed — the canonical delete logic now lives in the lifted facade and the existing codelet_core::persistence::sessions::delete_session helper is either consumed by the lifted SessionStore or deprecated to a re-export
  #   7. set_data_directory stays in codelet-napi because it resets NAPI-only stores (credentials, graph, blob) — but it delegates the persistence-store resets to a new codelet-core helper that clears the MESSAGE_STORE and SESSION_STORE singletons; codelet_common::set_data_directory remains the single source of truth for the directory itself
  #   8. BLOB_STORE singleton and store_blob/get_blob/blob_exists stay in codelet-napi until RPC-034 — the lifted SessionStore code does not reach for the blob store; cleanup_orphaned_messages and the synthetic compaction-summary path in get_session_messages remain blob-free
  #   9. list_sessions changes from pub(crate) to pub in the lifted location (no other crate-internal reason to keep it private once it lives in core); the existing NAPI binding `persistence_list_sessions` continues to call it through the re-export shim
  #   10. Test-only accessors for the lifted MESSAGE_STORE and SESSION_STORE singletons (is_message_store_initialized_for_tests, is_session_store_initialized_for_tests) are exposed by codelet-core so that codelet-napi's existing lazy_init_tests.rs can continue to assert per-store lazy initialization without re-introducing a direct reference to the moved lazy_static globals
  #
  # EXAMPLES:
  #   1. Round-trip: a SessionManifest created in a temp data dir via codelet_core::persistence::create_session, mutated through append_message_with_metadata + update_session_tokens, can be dropped and reopened — the loaded SessionStore reads {data_dir}/sessions/{uuid}.json and SessionStore::get(id) returns a SessionManifest byte-identical to the one that was saved
  #   2. codelet-rpc-embedded (which depends on codelet-core but is forbidden from depending on codelet-napi by rpc_006_source_shape.rs) can `use codelet_core::persistence::{SessionStore, SessionManifest, load_session, append_message_with_metadata, fork_session, merge_messages, cherry_pick, update_session_tokens, set_compaction_state}` and link successfully — proving the forbidden `rpc → napi` arrow is not re-introduced
  #   3. An existing NAPI caller in session_manager.rs continues to compile with `use crate::persistence::{load_session, append_message_with_metadata, update_session_tokens, get_session_messages_full, update_message_metadata}` after the lift, because napi/src/persistence/mod.rs flat-re-exports everything from codelet_core::persistence via `pub use codelet_core::persistence::*;`
  #   4. Fork: a SessionManifest with 5 messages forked at index 2 via codelet_core::persistence::fork_session produces a new SessionManifest with 3 messages (indices 0, 1, 2) all marked with MessageSource::Forked, a ForkPoint pointing at the source session, and the new session.json is persisted on disk
  #   5. Compaction round-trip: a SessionManifest with 10 messages compacted at index 5 via set_compaction_state stores a CompactionState{summary, compacted_before_index: 5, compacted_at} on the manifest; a subsequent get_session_messages returns 1 synthetic summary message (with metadata._compactionSummary == true) + 5 post-boundary messages (indices 5..10); clear_compaction_state followed by get_session_messages returns all 10 original messages
  #   6. All 80+ persistence tests continue to pass after the lift: `cargo test -p codelet-napi persistence::tests` (48 tests), `cargo test -p codelet-napi persistence::lazy_init_tests` (9 tests, including the BUG-122 lazy-init invariants for MESSAGE_STORE and SESSION_STORE), `cargo test -p codelet-napi --test session_persistence_test` (23 tests), and `cargo test -p codelet-napi --test subordinate_session_persistence_test` (4 tests) all pass via the re-export shim
  #   7. Cross-transport delete: deleting session s-2 via codelet_core::persistence::delete_session removes {data_dir}/sessions/s-2.json on disk and is observable by both EmbeddedFspecBackend and WebSocketFspecBackend — the existing rpc026_cross_transport_parity test continues to pass without the RPC-026 inverted-dependency double-delete trick
  #
  # ========================================
  Background: User Story
    As a fspec backend engineer
    I want to lift SessionStore SessionManifest and all session-level free functions out of codelet-napi into codelet-core::persistence::manifest
    So that codelet-rpc-embedded codelet-rpc-server and the upcoming codelet-sessions crate can manage session manifests without re-introducing a forbidden rpc to napi arrow while the on-disk session.json and messages.jsonl wire format remains byte-identical

  Scenario: SessionManifest round-trips through {data_dir}/sessions/{uuid}.json from codelet-core
    Given a fresh data directory is configured via codelet_common::set_data_directory
    And a session is created via codelet_core::persistence::create_session with name "Round Trip" and a project path
    When append_message_with_metadata is called with role "user" and content "hello" to record an in-context user message
    And update_session_tokens is called with input 120 output 60 cache_read 0 cache_create 0
    And the SESSION_STORE singleton is reset so the next load_session reads from disk
    Then load_session returns a SessionManifest whose id name project provider messages and token_usage equal the values that were saved
    And the on-disk file at {data_dir}/sessions/{uuid}.json contains a JSON object with current_context_tokens equal to 120

  Scenario: codelet-core consumers can construct and persist sessions without depending on codelet-napi
    Given codelet_core::persistence exports SessionStore SessionManifest load_session append_message_with_metadata fork_session merge_messages cherry_pick update_session_tokens and set_compaction_state
    When a downstream crate that does not depend on codelet-napi writes `use codelet_core::persistence::{SessionStore, SessionManifest, load_session, append_message_with_metadata, fork_session, merge_messages, cherry_pick, update_session_tokens, set_compaction_state}`
    Then the build succeeds with no transitive dependency on codelet-napi
    And the dependency-rule test rpc_006_source_shape.rs continues to pass

  Scenario: fork_session preserves provider lineage and persists the new manifest
    Given a fresh data directory and a parent session with 5 appended messages and provider "claude"
    When fork_session is called on the parent session with at_index 2 and name "Forked"
    Then a new SessionManifest is returned with 3 messages whose source is MessageSource::Forked
    And the new SessionManifest has a ForkPoint whose source_session_id equals the parent id and fork_after_index equals 2
    And the new manifest is persisted at {data_dir}/sessions/{new_uuid}.json with the same "claude" provider

  Scenario: Compaction round-trip returns the synthetic summary plus post-boundary messages
    Given a session manifest with 10 appended messages
    When set_compaction_state is called with summary "ten messages summarised" and compacted_before_index 5
    Then get_session_messages returns 6 entries — one synthetic message with metadata._compactionSummary equal to true followed by the 5 messages at indices 5 through 9
    And clear_compaction_state followed by get_session_messages returns all 10 original messages in order

  Scenario: delete_session removes the on-disk manifest through the lifted facade
    Given three sessions s-1 s-2 and s-3 are persisted as {data_dir}/sessions/{uuid}.json files
    When codelet_core::persistence::delete_session is called with s-2 followed by a fresh SessionStore::new
    Then {data_dir}/sessions/{s-2}.json no longer exists on disk
    And SessionStore::list_all returns only s-1 and s-3
    And calling delete_session again with s-2 is idempotent and returns Ok
