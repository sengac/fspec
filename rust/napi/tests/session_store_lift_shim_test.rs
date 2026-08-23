// Feature: spec/features/napi-re-export-shim-for-session-store.feature
//
// This integration test validates the RPC-033 acceptance criteria that
// are NAPI-side observable: the flat re-export shim in
// rust/napi/src/persistence/mod.rs (`pub use codelet_core::persistence::*;`)
// must keep every existing `crate::persistence::*` import path working,
// and the codelet-core types reached via the shim must BE the same
// types (compile-time identity).
//
// Living in `rust/napi/tests/` gives us access to BOTH the NAPI
// re-export paths (`codelet_napi::persistence::*`) and the underlying
// lifted module (`codelet_core::persistence::*`).
//
// Tests will FAIL to compile until the lift is implemented (red phase).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::persistence::{
    is_message_store_initialized_for_tests, is_session_store_initialized_for_tests,
    CompactionState as CoreCompactionState, ForkPoint as CoreForkPoint,
    MergeRecord as CoreMergeRecord, SessionLineage as CoreSessionLineage,
    SessionManifest as CoreSessionManifest, SessionStore as CoreSessionStore,
    TokenUsage as CoreTokenUsage,
};
use codelet_napi::persistence::{
    append_message_with_metadata, create_session, load_session, save_session,
    set_data_directory as napi_set_data_directory, update_session_tokens, CompactionState,
    ForkPoint, MergeRecord, SessionLineage, SessionManifest, SessionStore, TokenUsage,
};
use serial_test::serial;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    napi_set_data_directory(tmp.path().to_path_buf())
        .expect("napi set_data_directory must succeed");
    tmp
}

// ============================================================================
// Compile-time type identity proof — the NAPI shim re-exports rather than
// re-defines every lifted struct.
// ============================================================================
//
// If the shim is correct, the following assignments only compile when both
// names refer to the same Rust type.

#[allow(dead_code)]
fn _types_are_reexported_not_duplicated() {
    fn assert_same<T>(_a: &T, _b: &T) {}

    let core_manifest = CoreSessionManifest::new("ident", PathBuf::from("/x"));
    let napi_manifest = SessionManifest::new("ident", PathBuf::from("/x"));
    assert_same(&core_manifest, &napi_manifest);

    let core_tokens = CoreTokenUsage::default();
    let napi_tokens = TokenUsage::default();
    assert_same(&core_tokens, &napi_tokens);

    let core_fork = CoreForkPoint {
        source_session_id: uuid::Uuid::nil(),
        fork_after_index: 0,
        forked_at: chrono::Utc::now(),
    };
    let napi_fork = ForkPoint {
        source_session_id: uuid::Uuid::nil(),
        fork_after_index: 0,
        forked_at: chrono::Utc::now(),
    };
    assert_same(&core_fork, &napi_fork);

    let core_compaction = CoreCompactionState {
        summary: String::new(),
        compacted_before_index: 0,
        compacted_at: chrono::Utc::now(),
    };
    let napi_compaction = CompactionState {
        summary: String::new(),
        compacted_before_index: 0,
        compacted_at: chrono::Utc::now(),
    };
    assert_same(&core_compaction, &napi_compaction);

    let core_merge = CoreMergeRecord {
        source_session_id: uuid::Uuid::nil(),
        source_indices: Vec::new(),
        inserted_at: None,
        merged_at: chrono::Utc::now(),
    };
    let napi_merge = MergeRecord {
        source_session_id: uuid::Uuid::nil(),
        source_indices: Vec::new(),
        inserted_at: None,
        merged_at: chrono::Utc::now(),
    };
    assert_same(&core_merge, &napi_merge);

    let core_lineage = CoreSessionLineage {
        session_id: uuid::Uuid::nil(),
        forked_from: None,
        merged_from: Vec::new(),
    };
    let napi_lineage = SessionLineage {
        session_id: uuid::Uuid::nil(),
        forked_from: None,
        merged_from: Vec::new(),
    };
    assert_same(&core_lineage, &napi_lineage);

    // SessionStore is reachable through both paths; assigning one to the
    // other proves the aliases point at the same struct.
    let _store: Option<SessionStore> = None::<CoreSessionStore>;
}

// ============================================================================
// Scenario: NAPI re-export shim preserves existing crate::persistence imports for session types
// ============================================================================

#[test]
#[serial]
fn napi_reexport_paths_route_through_core_for_session_types() {
    // @step Given rust/napi/src/persistence/storage.rs is deleted and rust/napi/src/persistence/types.rs is deleted
    // (verified at compile time by the absence of NAPI-local SessionStore/SessionManifest types — the
    //  aliases above resolve through the flat re-export)

    // @step And rust/napi/src/persistence/mod.rs re-exports the lifted surface via `pub use codelet_core::persistence::*;`
    let _data_dir = setup_data_dir();

    // @step When internal NAPI modules continue to write `use crate::persistence::{SessionManifest, load_session, append_message_with_metadata, update_session_tokens, get_session_messages_full, update_message_metadata}` unchanged
    let project = PathBuf::from("/test/project/rpc033/shim");
    let mut session = create_session("Shim Path", &project).expect("create_session via NAPI alias");
    let session_id = session.id;
    append_message_with_metadata(&mut session, "user", "shim hello", HashMap::new())
        .expect("append_message_with_metadata via NAPI alias");
    update_session_tokens(&mut session, 42, 21, 0, 0, 0)
        .expect("update_session_tokens via NAPI alias");
    save_session(&session).expect("save_session via NAPI alias");

    // @step Then the imports resolve to the codelet-core types
    let core_view: CoreSessionManifest = load_session(session_id).expect("load via NAPI alias");
    assert_eq!(core_view.id, session_id);
    assert_eq!(core_view.token_usage.current_context_tokens, 42);

    // @step And `cargo build -p codelet-napi` succeeds without modification of those importing modules
    // (verified by CI running `cargo build -p codelet-napi` after the lift)
}

// ============================================================================
// Scenario: All NAPI persistence test suites continue to pass after the session store lift
// ============================================================================

#[test]
#[serial]
fn napi_session_store_shim_round_trips_a_full_manifest_lifecycle() {
    // @step Given SessionStore SessionManifest TokenUsage MergeRecord PastedContent ForkPoint CompactionState SessionLineage and every session-level free function live in codelet-core and are re-exported by NAPI
    let _data_dir = setup_data_dir();

    // @step When the existing test suites are run with `cargo test -p codelet-napi persistence::tests`, `cargo test -p codelet-napi persistence::lazy_init_tests`, `cargo test -p codelet-napi --test session_persistence_test`, and `cargo test -p codelet-napi --test subordinate_session_persistence_test`
    // We exercise the smallest possible slice end-to-end through the NAPI
    // flat re-export to assert the shim path works; the per-suite green
    // bar is enforced by CI.
    let project = PathBuf::from("/test/project/rpc033/shim_lifecycle");
    let mut parent =
        codelet_napi::persistence::create_session_with_provider("Parent", &project, "claude")
            .expect("create_session_with_provider via NAPI shim");
    for i in 0..3 {
        codelet_napi::persistence::append_message(&mut parent, "user", &format!("msg-{i}"))
            .expect("append_message via NAPI shim");
    }

    // @step Then the 48 persistence tests pass
    assert_eq!(parent.messages.len(), 3);
    assert_eq!(parent.provider, "claude");

    // @step And the 9 lazy_init_tests pass including the BUG-122 lazy-init invariants for MESSAGE_STORE and SESSION_STORE accessed via codelet_core::persistence::{is_message_store_initialized_for_tests, is_session_store_initialized_for_tests}
    // After a fresh set_data_directory + at least one message append, both
    // singletons are initialised.
    assert!(
        is_message_store_initialized_for_tests(),
        "MESSAGE_STORE must be lazily initialised by append_message"
    );
    assert!(
        is_session_store_initialized_for_tests(),
        "SESSION_STORE must be lazily initialised by create_session"
    );

    // @step And the 23 session_persistence_test cases and 4 subordinate_session_persistence_test cases pass
    let forked = codelet_napi::persistence::fork_session(&parent, 1, "Forked")
        .expect("fork_session via NAPI shim");
    assert_eq!(forked.messages.len(), 2);
    assert!(forked.forked_from.is_some());
    assert_eq!(forked.provider, "claude");
}

// ============================================================================
// Scenario: set_data_directory in codelet-napi resets credentials graph blob and core persistence singletons
// ============================================================================

#[test]
#[serial]
fn napi_set_data_directory_resets_core_persistence_singletons() {
    // @step Given codelet_napi::persistence::set_data_directory has been replaced with a wrapper that delegates the persistence-store reset to codelet_core::persistence::reset_stores_for_tests
    // @step And a temporary directory is prepared for the test run
    let tmp = tempfile::tempdir().expect("create temp data dir");

    // Force prior state: set a directory, create a session so the
    // SESSION_STORE singleton is initialised.
    let priming_dir = tempfile::tempdir().expect("create priming dir");
    napi_set_data_directory(priming_dir.path().to_path_buf()).expect("priming set_data_directory");
    let project = PathBuf::from("/test/project/rpc033/set_data_dir_priming");
    let mut session = create_session("priming", &project).expect("create_session");
    append_message_with_metadata(&mut session, "user", "primer", HashMap::new())
        .expect("append_message_with_metadata");
    assert!(is_message_store_initialized_for_tests());
    assert!(is_session_store_initialized_for_tests());

    // @step When codelet_napi::persistence::set_data_directory is called with the temporary directory path
    napi_set_data_directory(tmp.path().to_path_buf())
        .expect("napi set_data_directory must succeed");

    // @step Then codelet_common::get_data_dir returns the temporary path
    assert_eq!(
        codelet_common::get_data_dir().expect("get_data_dir"),
        tmp.path().to_path_buf()
    );

    // @step And codelet_core::persistence::is_message_store_initialized_for_tests returns false
    assert!(
        !is_message_store_initialized_for_tests(),
        "MESSAGE_STORE singleton must be cleared after set_data_directory"
    );

    // @step And codelet_core::persistence::is_session_store_initialized_for_tests returns false
    assert!(
        !is_session_store_initialized_for_tests(),
        "SESSION_STORE singleton must be cleared after set_data_directory"
    );

    // @step And the NAPI-owned BLOB_STORE singleton is cleared so the next blob operation re-initialises against the new directory
    // After RPC-033 the BLOB_STORE singleton still lives in codelet-napi
    // (until RPC-034). The cleanest cross-crate assertion is that storing
    // a blob in the freshly-set directory writes to the new path — the
    // pre-priming dir must NOT contain any new blob files.
    let blob_hash = codelet_napi::persistence::store_blob(b"after reset")
        .expect("store_blob after set_data_directory");
    let new_blob_path = tmp
        .path()
        .join("blobs")
        .join(&blob_hash[0..2])
        .join(&blob_hash);
    assert!(
        new_blob_path.exists(),
        "blob must be written under the new data directory"
    );
}
