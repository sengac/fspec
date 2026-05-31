@done
@blob
@refactor
@persistence
@rust
@napi
@rpc
@session-management
@RPC-034
Feature: NAPI Re-Export Shim For Blob Store
  """
  The NAPI persistence module retains its existing public surface (codelet_napi::persistence::*) after RPC-034 lifts BlobStore + should_use_blob_storage + blob_processing helpers. codelet/napi/src/persistence/blob.rs and codelet/napi/src/persistence/blob_processing.rs are deleted outright. codelet/napi/src/persistence/mod.rs further shrinks (losing the blob+blob_processing module declarations, the BLOB_STORE lazy_static singleton, and the init_blob_store/store_blob/get_blob/blob_exists facade) and becomes a ~30-line thin facade that keeps only set_data_directory (delegating persistence resets to codelet_core::persistence::reset_stores_for_tests which is widened to cover BLOB_STORE), get_data_dir, the History wrappers, and the pub use codelet_core::persistence::*; flat re-export. All internal NAPI modules (session_manager.rs, session_search_handler.rs, persistence/napi_bindings.rs, persistence/tests.rs, persistence/lazy_init_tests.rs) continue to use crate::persistence::{BlobStore, store_blob, get_blob, blob_exists, BLOB_REF_PREFIX, is_blob_reference, extract_blob_hash, make_blob_reference, process_envelope_for_blob_storage, rehydrate_envelope_blobs, should_use_blob_storage} paths unchanged. Lift precedent: matches RPC-025 (history.rs), RPC-026 (sessions.rs delete_session), RPC-031 (message_envelope.rs), RPC-032 (message store), and RPC-033 (session store).
  """

  Background: User Story
    As a fspec backend engineer maintaining the NAPI surface
    I want to expose codelet_core::persistence::blob and codelet_core::persistence::blob_processing types (BlobStore, store_blob, get_blob, blob_exists, should_use_blob_storage, BLOB_REF_PREFIX, is_blob_reference, extract_blob_hash, make_blob_reference, process_envelope_for_blob_storage, rehydrate_envelope_blobs, is_blob_store_initialized_for_tests) through thin re-export shims at codelet_napi::persistence
    So that every existing crate::persistence::* import in codelet-napi continues to compile, the on-disk blob layout and BLOB_REF_PREFIX wire format remain byte-identical after the lift, and lazy-initialization invariants for the moved BLOB_STORE singleton are observable from NAPI tests via codelet_core::persistence test accessors

  Scenario: NAPI re-export shim preserves existing crate::persistence imports for blob types
    Given codelet/napi/src/persistence/blob.rs is deleted and codelet/napi/src/persistence/blob_processing.rs is deleted
    When internal NAPI modules continue to write `use crate::persistence::{BlobStore, store_blob, get_blob, blob_exists, BLOB_REF_PREFIX, is_blob_reference, extract_blob_hash, make_blob_reference, process_envelope_for_blob_storage, rehydrate_envelope_blobs, should_use_blob_storage}` unchanged
    Then the imports resolve to the codelet-core types
    And codelet/napi/src/persistence/mod.rs re-exports the lifted surface via `pub use codelet_core::persistence::*;`
    And `cargo build -p codelet-napi` succeeds without modification of those importing modules

  Scenario: All NAPI persistence test suites continue to pass after the blob store lift
    Given BlobStore, should_use_blob_storage, the BLOB_STORE singleton, the store_blob/get_blob/blob_exists facade, BLOB_REF_PREFIX, is_blob_reference, extract_blob_hash, make_blob_reference, process_envelope_for_blob_storage, and rehydrate_envelope_blobs live in codelet-core and are re-exported by NAPI
    When the existing test suites are run with `cargo test -p codelet-napi persistence::tests` and `cargo test -p codelet-napi persistence::lazy_init_tests`
    Then the 48 persistence tests pass — including test_blob_reference_format, test_blob_storage_dedup, the tool_result/image/document/thinking/tool_use blob round-trip tests, and the multi-content envelope dedup test
    And the 9 lazy_init_tests pass — including the BUG-122 lazy-init invariant for BlobStore accessed via codelet_core::persistence::is_blob_store_initialized_for_tests

  Scenario: set_data_directory in codelet-napi resets the lifted BLOB_STORE alongside MESSAGE_STORE SESSION_STORE history credentials and graph
    Given codelet_core::persistence::reset_stores_for_tests has been widened to also clear the BLOB_STORE singleton and codelet_napi::persistence::set_data_directory delegates the persistence-store reset to it
    When codelet_napi::persistence::set_data_directory is called with a temporary directory path
    Then codelet_common::get_data_dir returns the temporary path
    And codelet_core::persistence::is_blob_store_initialized_for_tests returns false
    And codelet_core::persistence::is_message_store_initialized_for_tests and codelet_core::persistence::is_session_store_initialized_for_tests also return false

  Scenario: BlobStore is initialized lazily and only by blob operations
    Given codelet_napi::persistence::set_data_directory has just been called with a fresh temporary directory
    And codelet_core::persistence::is_blob_store_initialized_for_tests returns false
    When codelet_core::persistence::store_blob is called once with a 20000 byte buffer
    Then codelet_core::persistence::is_blob_store_initialized_for_tests returns true
    And codelet_core::persistence::is_message_store_initialized_for_tests still returns false and codelet_core::persistence::is_session_store_initialized_for_tests still returns false
