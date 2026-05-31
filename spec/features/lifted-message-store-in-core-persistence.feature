@done
@refactor
@session-management
@persistence
@rust
@napi
@rpc
@RPC-032
Feature: Lift MessageStore + message_index into codelet-core::persistence::messages
  """
  [0] MessageStore::new() preserves BUG-122 lazy-init: on a data dir with an existing messages.idx whose recorded data_file_size matches the actual messages.jsonl size, the constructor returns without scanning the whole JSONL file (verified by the existing lazy_init_tests::test_message_store_uses_existing_index_without_scan path)
  [1] codelet-core/Cargo.toml gains three new dependencies that are currently only in codelet-napi: `lru` (LRU cache for the deserialized-message store), `sha2` (already in workspace.dependencies — wire as sha2.workspace = true), and `hex` (used by compute_hash to render the SHA-256 to a 64-char string). lru and hex are added as direct version deps since they are not yet in workspace.dependencies.
  [2] MessageStore::new() in the lifted location calls codelet_common::get_data_dir() directly to compute the messages dir, then std::fs::create_dir_all(&messages_dir) to ensure it exists. This removes the dependency on the NAPI-local `super::{ensure_directories, get_data_dir}` helpers, which stay in napi/src/persistence/mod.rs (they also create sessions/ and blobs/ dirs that codelet-core cannot know about until RPC-033/RPC-034).
  [3] Layout: codelet/core/src/persistence/messages.rs declares `mod index;` referring to `codelet/core/src/persistence/messages/index.rs` (former message_index.rs). The submodule is pub(super) for its public functions (load_index, save_index, scan_jsonl_range, read_message_at, IndexEntry) so MessageStore in messages.rs reaches them while keeping the public surface of codelet_core::persistence::messages limited to the high-level types.
  [4] Tests for compute_hash (single test in storage.rs lines 611-620) move to messages.rs alongside the function. The 48 persistence tests in tests.rs and 9 in lazy_init_tests.rs stay in napi (they exercise the full pipeline through the NAPI mod.rs free functions and global singletons, which are not lifted in this card). They continue to pass via the re-export shim.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. MessageStore lives in codelet-core::persistence::messages (no NAPI dependency)
  #   2. MessageStore, compute_hash, StoredMessage, MessageSource, MessageRef and the binary index helpers (IndexEntry, load_index, save_index, scan_jsonl_range, read_message_at) all live in codelet/core/src/persistence/messages.rs (the index helpers as a nested `mod index` submodule)
  #   3. codelet/napi/src/persistence/storage.rs no longer contains MessageStore or compute_hash; it retains only SessionStore (which is lifted in RPC-033). The MessageStore + compute_hash entry points are re-exported via `pub use codelet_core::persistence::messages::{MessageStore, compute_hash};` so every existing `crate::persistence::*` import path inside codelet-napi continues to compile unchanged.
  #   4. codelet/napi/src/persistence/message_index.rs is deleted (the `mod message_index;` declaration is removed from persistence/mod.rs) — the binary index helpers are reached exclusively through codelet_core::persistence::messages
  #   5. codelet/napi/src/persistence/types.rs no longer defines StoredMessage, MessageSource, or MessageRef; it re-exports those three types from codelet_core::persistence::messages so SessionManifest (which still lives in NAPI for RPC-033) keeps the same field type Vec<MessageRef>
  #   6. On-disk format is byte-identical: messages.jsonl serialization of StoredMessage and the messages.idx binary index format (MIDX magic, version 1, 28-byte entries) are unchanged after the lift
  #   7. MessageStore::get_referenced_ids — which previously took &[SessionManifest] from NAPI — is removed from the lifted struct (it had no NAPI-free callers). Its sole call site in codelet-napi (cleanup_orphaned_messages in persistence/mod.rs) inlines the equivalent HashSet<Uuid> build from the in-memory sessions cache before calling cleanup_orphans. This keeps codelet-core::persistence::messages free of any SessionManifest dependency, so RPC-033 can lift SessionStore cleanly.
  #
  # EXAMPLES:
  #   1. compute_hash(b"hello") returns the same 64-char hex SHA-256 from codelet_core::persistence::messages::compute_hash as it did from codelet_napi::persistence::compute_hash
  #   2. Round-trip: a MessageStore created in a temp data dir, used to store a (role, content, metadata) tuple, can be dropped and reopened — the loaded MessageStore reads the messages.idx file (no full JSONL scan) and MessageStore::get(id) returns a StoredMessage byte-identical to the one that was stored
  #   3. codelet-rpc-embedded (which depends on codelet-core but is forbidden from depending on codelet-napi by rpc_006_source_shape.rs) can `use codelet_core::persistence::messages::{MessageStore, StoredMessage}` and link successfully — proving the forbidden `rpc → napi` arrow is not re-introduced
  #   4. An existing NAPI caller in session_search_handler.rs continues to compile with `use crate::persistence::{StoredMessage, get_session_messages_full}` after the lift, because storage.rs re-exports MessageStore + compute_hash from codelet_core and types.rs re-exports StoredMessage from codelet_core
  #   5. cargo build -p codelet-core, cargo build -p codelet-napi, cargo test -p codelet-napi persistence::tests (48 tests), cargo test -p codelet-napi persistence::lazy_init_tests (9 tests), and cargo test -p codelet-napi --test session_persistence_test (23 tests) all pass after the lift
  #
  # ========================================
  Background: User Story
    As a rust developer working on the codelet workspace
    I want to import MessageStore + compute_hash + StoredMessage + MessageSource + MessageRef + binary message index from codelet-core::persistence::messages instead of codelet-napi
    So that non-NAPI crates (rpc-embedded, rpc-server, future codelet-sessions) can call into the on-disk message store without re-introducing a forbidden rpc → napi dependency

  Scenario: compute_hash produces the same SHA-256 hex from codelet-core as from codelet-napi
    Given codelet_core::persistence::messages::compute_hash is invoked with the byte string "hello"
    When the returned String is compared to codelet_napi::persistence::compute_hash("hello")
    Then both values are the same 64-character lowercase hex SHA-256
    And the value matches a SHA-256 of "hello" computed independently

  Scenario: MessageStore round-trips a stored message through messages.jsonl and messages.idx
    Given a fresh data directory is configured via codelet_common::set_data_directory
    And a MessageStore is constructed from codelet_core::persistence::messages
    When a message with role "user" and content "hello world" plus an empty metadata map is stored via MessageStore::store
    And the MessageStore is dropped
    And a second MessageStore is constructed against the same data directory
    Then the second MessageStore reads the existing messages.idx without rescanning messages.jsonl
    And MessageStore::get(id) on the second store returns a StoredMessage whose role, content, content_hash, and token_count equal the originally stored values

  Scenario: codelet-core consumers can import MessageStore without depending on codelet-napi
    Given codelet_core::persistence::messages exports MessageStore, compute_hash, StoredMessage, MessageSource, and MessageRef
    When a downstream crate that does not depend on codelet-napi (codelet-rpc-embedded) writes `use codelet_core::persistence::messages::{MessageStore, StoredMessage, compute_hash}`
    Then the build succeeds with no transitive dependency on codelet-napi
    And the dependency-rule test rpc_006_source_shape.rs continues to pass

  Scenario: messages.jsonl and messages.idx wire format stays byte-identical after the lift
    Given a pre-existing messages.jsonl plus messages.idx pair produced by the pre-lift NAPI MessageStore
    When the post-lift codelet_core::persistence::messages::MessageStore is opened against the same data directory
    Then the loaded index map has the same UUID→IndexEntry entries
    And the recorded data_file_size in the .idx header equals the messages.jsonl length
    And reading any message via MessageStore::get returns the same StoredMessage JSON as the original NAPI store would have returned
