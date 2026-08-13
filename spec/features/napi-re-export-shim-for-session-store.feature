@done
@session-management
@rpc
@napi
@rust
@persistence
@refactor
@RPC-033
@critical
Feature: NAPI Re-Export Shim For Session Store
  """
  The NAPI persistence module retains its existing public surface (codelet_napi::persistence::*) after RPC-033 lifts SessionStore + SessionManifest + every session-level free function. rust/napi/src/persistence/storage.rs and rust/napi/src/persistence/types.rs are deleted outright. rust/napi/src/persistence/mod.rs becomes a ~50-line thin facade that declares the still-NAPI-owned blob+blob_processing modules, keeps the BLOB_STORE lazy_static singleton (until RPC-034), wraps set_data_directory with the credentials+graph resets while delegating persistence resets to codelet_core::persistence::reset_stores_for_tests, and re-exports the lifted surface via `pub use codelet_core::persistence::*;`. All internal NAPI modules (session_manager.rs, session_search_handler.rs, agent_manager_handler.rs, test_support.rs, persistence/napi_bindings.rs, persistence/tests.rs, persistence/lazy_init_tests.rs) continue to use `crate::persistence::{SessionManifest, load_session, append_message_with_metadata, ...}` paths unchanged. Lift precedent: matches RPC-025 (history.rs), RPC-026 (sessions.rs delete_session), RPC-031 (message_envelope.rs), and RPC-032 (message store).
  """

  Background: User Story
    As a fspec backend engineer maintaining the NAPI surface
    I want to expose codelet_core::persistence::manifest types (SessionStore, SessionManifest, TokenUsage, MergeRecord, PastedContent, ForkPoint, CompactionState, SessionLineage, list_sessions, list_sessions_for_project, list_all_sessions, load_session, append_message_with_metadata, fork_session, merge_messages, cherry_pick, update_session_tokens, set_compaction_state, clear_compaction_state, delete_session, rename_session, get_session_messages, get_session_messages_full) through thin re-export shims at codelet_napi::persistence
    So that every existing crate::persistence::* import in codelet-napi continues to compile, the on-disk session.json wire format remains byte-identical after the lift, and lazy-initialization invariants for the moved MESSAGE_STORE and SESSION_STORE singletons are observable from NAPI tests via codelet_core::persistence test accessors

  Scenario: NAPI re-export shim preserves existing crate::persistence imports for session types
    Given rust/napi/src/persistence/storage.rs is deleted and rust/napi/src/persistence/types.rs is deleted
    When internal NAPI modules continue to write `use crate::persistence::{SessionManifest, load_session, append_message_with_metadata, update_session_tokens, get_session_messages_full, update_message_metadata}` unchanged
    Then the imports resolve to the codelet-core types
    And rust/napi/src/persistence/mod.rs re-exports the lifted surface via `pub use codelet_core::persistence::*;`
    And `cargo build -p codelet-napi` succeeds without modification of those importing modules

  Scenario: All NAPI persistence test suites continue to pass after the session store lift
    Given SessionStore SessionManifest TokenUsage MergeRecord PastedContent ForkPoint CompactionState SessionLineage and every session-level free function live in codelet-core and are re-exported by NAPI
    When the existing test suites are run with `cargo test -p codelet-napi persistence::tests`, `cargo test -p codelet-napi persistence::lazy_init_tests`, `cargo test -p codelet-napi --test session_persistence_test`, and `cargo test -p codelet-napi --test subordinate_session_persistence_test`
    Then the 48 persistence tests pass
    And the 9 lazy_init_tests pass including the BUG-122 lazy-init invariants for MESSAGE_STORE and SESSION_STORE accessed via codelet_core::persistence::{is_message_store_initialized_for_tests, is_session_store_initialized_for_tests}
    And the 23 session_persistence_test cases and 4 subordinate_session_persistence_test cases pass

  Scenario: set_data_directory in codelet-napi resets credentials graph blob and core persistence singletons
    Given codelet_napi::persistence::set_data_directory has been replaced with a wrapper that delegates the persistence-store reset to codelet_core::persistence::reset_stores_for_tests
    When codelet_napi::persistence::set_data_directory is called with the temporary directory path
    Then codelet_common::get_data_dir returns the temporary path
    And a temporary directory is prepared for the test run
    And codelet_core::persistence::is_message_store_initialized_for_tests returns false
    And codelet_core::persistence::is_session_store_initialized_for_tests returns false
    And the NAPI-owned BLOB_STORE singleton is cleared so the next blob operation re-initialises against the new directory
