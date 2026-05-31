@done
@RPC-032
@rpc
@napi
@rust
@persistence
@session-management
@refactor
Feature: NAPI Re-Export Shim For Message Store
  """
  The NAPI persistence module retains its existing public surface (codelet_napi::persistence::*) after RPC-032's MessageStore + message_index lift. The shim in codelet/napi/src/persistence/storage.rs becomes `pub use codelet_core::persistence::messages::{MessageStore, compute_hash};` (SessionStore + the SessionStore-only helpers stay in storage.rs until RPC-033). codelet/napi/src/persistence/types.rs additionally re-exports StoredMessage, MessageSource, and MessageRef from codelet-core. codelet/napi/src/persistence/message_index.rs is deleted outright (the `mod message_index;` declaration is removed from persistence/mod.rs). All internal NAPI modules (session_manager.rs, session_search_handler.rs, persistence/mod.rs, persistence/napi_bindings.rs, persistence/tests.rs, persistence/lazy_init_tests.rs) continue to use `crate::persistence::{MessageStore, StoredMessage, MessageSource, MessageRef, compute_hash}` paths unchanged. Lift precedent: matches RPC-025 (history.rs), RPC-026 (sessions.rs delete_session), and RPC-031 (message_envelope.rs).
  """

  Background: User Story
    As a fspec backend engineer maintaining the NAPI surface
    I want to expose codelet_core::persistence::messages types (MessageStore, compute_hash, StoredMessage, MessageSource, MessageRef) through thin re-export shims at codelet_napi::persistence
    So that every existing crate::persistence::* import in codelet-napi continues to compile and the on-disk JSONL/idx wire format remains byte-identical after the lift

  Scenario: NAPI re-export shim preserves existing crate::persistence imports
    Given codelet/napi/src/persistence/storage.rs re-exports MessageStore and compute_hash from codelet_core::persistence::messages
    And codelet/napi/src/persistence/types.rs re-exports StoredMessage, MessageSource, and MessageRef from codelet_core::persistence::messages
    When internal NAPI modules continue to write `use crate::persistence::{MessageStore, StoredMessage, MessageSource, MessageRef, compute_hash}` unchanged
    Then the imports resolve to the codelet-core types
    And `cargo build -p codelet-napi` succeeds without modification of those importing modules

  Scenario: All NAPI persistence test suites continue to pass after the lift
    Given MessageStore, compute_hash, StoredMessage, MessageSource, MessageRef, and the binary index helpers live in codelet-core and are re-exported by NAPI
    When the existing test suites are run with `cargo test -p codelet-napi persistence::tests`, `cargo test -p codelet-napi persistence::lazy_init_tests`, and `cargo test -p codelet-napi --test session_persistence_test`
    Then the 48 persistence tests pass
    And the 9 lazy_init_tests pass (including the BUG-122 lazy-init coverage that asserts MessageStore::new() does not rescan the JSONL when messages.idx is current)
    And the 23 session_persistence_test cases pass
