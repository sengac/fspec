// Feature: spec/features/napi-re-export-shim-for-blob-store.feature
//
// This integration test validates the RPC-034 acceptance criteria that
// are NAPI-side observable: the flat re-export shim in
// codelet/napi/src/persistence/mod.rs (`pub use codelet_core::persistence::*;`)
// must keep every existing `crate::persistence::*` import path working,
// the codelet-core types reached via the shim must BE the same types
// (compile-time identity), and the set_data_directory wrapper must clear
// the lifted BLOB_STORE singleton alongside MESSAGE_STORE + SESSION_STORE
// + history.
//
// Living in `codelet/napi/tests/` gives us access to BOTH the NAPI
// re-export paths (`codelet_napi::persistence::*`) and the underlying
// lifted module (`codelet_core::persistence::*`).
//
// Tests will FAIL to compile until the lift is implemented (red phase).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Utc;
use codelet_core::persistence::{
    is_blob_store_initialized_for_tests, is_message_store_initialized_for_tests,
    is_session_store_initialized_for_tests, BlobStore as CoreBlobStore,
};
use codelet_napi::persistence::{
    blob_exists, extract_blob_hash, get_blob, is_blob_reference, make_blob_reference,
    process_envelope_for_blob_storage, rehydrate_envelope_blobs,
    set_data_directory as napi_set_data_directory, should_use_blob_storage, store_blob,
    AssistantContent, AssistantMessage, BlobStore as NapiBlobStore, MessageEnvelope,
    MessagePayload, UserContent, UserMessage, BLOB_REF_PREFIX,
};
use serial_test::serial;
use tempfile::TempDir;
use uuid::Uuid;

fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    napi_set_data_directory(tmp.path().to_path_buf())
        .expect("napi set_data_directory must succeed");
    tmp
}

// ============================================================================
// Compile-time type identity proof — the NAPI shim re-exports rather than
// re-defines BlobStore. If the two paths refer to the same type, the
// assignment below compiles.
// ============================================================================

#[allow(dead_code)]
fn _blob_store_type_is_reexported_not_duplicated() {
    fn assert_same<T>(_a: &Option<T>, _b: &Option<T>) {}
    let core: Option<CoreBlobStore> = None;
    let napi: Option<NapiBlobStore> = None;
    assert_same(&core, &napi);
}

// ============================================================================
// Scenario: NAPI re-export shim preserves existing crate::persistence imports for blob types
// ============================================================================

#[test]
#[serial]
fn napi_reexport_paths_route_through_core_for_blob_types() {
    // @step Given codelet/napi/src/persistence/blob.rs is deleted and codelet/napi/src/persistence/blob_processing.rs is deleted
    // (verified at compile time by the absence of NAPI-local BlobStore / blob_processing modules — the
    //  aliases above resolve through the flat re-export `pub use codelet_core::persistence::*;`)
    let _data_dir = setup_data_dir();

    // @step When internal NAPI modules continue to write `use crate::persistence::{BlobStore, store_blob, get_blob, blob_exists, BLOB_REF_PREFIX, is_blob_reference, extract_blob_hash, make_blob_reference, process_envelope_for_blob_storage, rehydrate_envelope_blobs, should_use_blob_storage}` unchanged
    let payload = b"napi-shim re-export sanity payload".repeat(400);
    assert!(should_use_blob_storage(&payload));
    let hash = store_blob(&payload).expect("store_blob via NAPI shim");
    assert!(blob_exists(&hash).expect("blob_exists via NAPI shim"));
    let fetched = get_blob(&hash).expect("get_blob via NAPI shim");
    assert_eq!(fetched, payload);

    let reference = make_blob_reference(&hash);
    assert!(is_blob_reference(&reference));
    assert_eq!(extract_blob_hash(&reference), Some(hash.as_str()));

    // @step Then the imports resolve to the codelet-core types
    // (compile-time identity is asserted by _blob_store_type_is_reexported_not_duplicated above;
    //  here we additionally prove that BLOB_REF_PREFIX is the same constant)
    assert_eq!(BLOB_REF_PREFIX, "blob:sha256:");

    // @step And codelet/napi/src/persistence/mod.rs re-exports the lifted surface via `pub use codelet_core::persistence::*;`
    // (verified by the `use codelet_napi::persistence::*` block at the top)

    // @step And `cargo build -p codelet-napi` succeeds without modification of those importing modules
    // (verified by CI running `cargo build -p codelet-napi` after the lift)
}

// ============================================================================
// Scenario: All NAPI persistence test suites continue to pass after the blob store lift
// ============================================================================

#[test]
#[serial]
fn napi_blob_shim_round_trips_an_envelope_with_blob_extraction() {
    // @step Given BlobStore, should_use_blob_storage, the BLOB_STORE singleton, the store_blob/get_blob/blob_exists facade, BLOB_REF_PREFIX, is_blob_reference, extract_blob_hash, make_blob_reference, process_envelope_for_blob_storage, and rehydrate_envelope_blobs live in codelet-core and are re-exported by NAPI
    let _data_dir = setup_data_dir();

    // @step When the existing test suites are run with `cargo test -p codelet-napi persistence::tests` and `cargo test -p codelet-napi persistence::lazy_init_tests`
    // We exercise the smallest possible slice end-to-end through the NAPI
    // flat re-export to assert the shim path works; the per-suite green
    // bar is enforced by CI.
    let large_content = "y".repeat(15_000);
    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_napi_shim_rpc034".to_string(),
                content: large_content.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    let (processed, blob_refs) =
        process_envelope_for_blob_storage(&envelope).expect("shim process_envelope");

    // @step Then the 48 persistence tests pass — including test_blob_reference_format, test_blob_storage_dedup, the tool_result/image/document/thinking/tool_use blob round-trip tests, and the multi-content envelope dedup test
    assert_eq!(blob_refs.len(), 1);
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert!(content.starts_with(BLOB_REF_PREFIX));
            }
            _ => panic!("expected ToolResult"),
        },
        _ => panic!("expected User payload"),
    }

    let rehydrated_json =
        rehydrate_envelope_blobs(&serde_json::to_string(&processed).expect("serialize"))
            .expect("rehydrate via shim");
    let rehydrated: MessageEnvelope =
        serde_json::from_str(&rehydrated_json).expect("parse rehydrated");
    match &rehydrated.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert_eq!(*content, large_content);
            }
            _ => panic!("expected ToolResult"),
        },
        _ => panic!("expected User payload"),
    }

    // Exercise an Assistant content variant to prove AssistantContent +
    // AssistantMessage are also reachable through the shim re-export.
    let _ = AssistantContent::Text {
        text: "via shim".to_string(),
    };
    let _ = AssistantMessage {
        role: "assistant".to_string(),
        id: None,
        model: None,
        content: Vec::new(),
        stop_reason: None,
        usage: None,
    };

    // @step And the 9 lazy_init_tests pass — including the BUG-122 lazy-init invariant for BlobStore accessed via codelet_core::persistence::is_blob_store_initialized_for_tests
    assert!(
        is_blob_store_initialized_for_tests(),
        "BLOB_STORE must be lazily initialised after process_envelope_for_blob_storage stored a blob"
    );
}

// ============================================================================
// Scenario: set_data_directory in codelet-napi resets the lifted BLOB_STORE alongside MESSAGE_STORE SESSION_STORE history credentials and graph
// ============================================================================

#[test]
#[serial]
fn napi_set_data_directory_resets_lifted_blob_store_alongside_message_and_session_stores() {
    // @step Given codelet_core::persistence::reset_stores_for_tests has been widened to also clear the BLOB_STORE singleton and codelet_napi::persistence::set_data_directory delegates the persistence-store reset to it
    // Force prior state in a priming dir so the BLOB_STORE singleton is
    // initialised before we switch dirs.
    let priming = tempfile::tempdir().expect("create priming dir");
    napi_set_data_directory(priming.path().to_path_buf()).expect("priming set_data_directory");

    let payload = b"priming the lazy BLOB_STORE singleton before the swap".repeat(300);
    let _ = store_blob(&payload).expect("priming store_blob");
    assert!(
        is_blob_store_initialized_for_tests(),
        "priming store_blob must initialise BLOB_STORE"
    );

    // @step When codelet_napi::persistence::set_data_directory is called with a temporary directory path
    let target = tempfile::tempdir().expect("create target dir");
    napi_set_data_directory(target.path().to_path_buf())
        .expect("napi set_data_directory must succeed");

    // @step Then codelet_common::get_data_dir returns the temporary path
    assert_eq!(
        codelet_common::get_data_dir().expect("get_data_dir"),
        target.path().to_path_buf()
    );

    // @step And codelet_core::persistence::is_blob_store_initialized_for_tests returns false
    assert!(
        !is_blob_store_initialized_for_tests(),
        "BLOB_STORE singleton must be cleared after set_data_directory"
    );

    // @step And codelet_core::persistence::is_message_store_initialized_for_tests and codelet_core::persistence::is_session_store_initialized_for_tests also return false
    assert!(!is_message_store_initialized_for_tests());
    assert!(!is_session_store_initialized_for_tests());

    // Sanity: writing a blob now lands under the NEW data dir, not the
    // priming dir (proves the lifted BlobStore re-initialised against the
    // freshly-set data directory).
    let post_hash = store_blob(b"after the swap").expect("store_blob after swap");
    let new_blob_path = target
        .path()
        .join("blobs")
        .join(&post_hash[0..2])
        .join(&post_hash);
    assert!(
        new_blob_path.exists(),
        "post-swap blob must land under the new data directory"
    );
}

// ============================================================================
// Scenario: BlobStore is initialized lazily and only by blob operations
// ============================================================================

#[test]
#[serial]
fn blob_store_is_initialized_lazily_and_only_by_blob_operations() {
    // @step Given codelet_napi::persistence::set_data_directory has just been called with a fresh temporary directory
    let _data_dir = setup_data_dir();

    // @step And codelet_core::persistence::is_blob_store_initialized_for_tests returns false
    assert!(
        !is_blob_store_initialized_for_tests(),
        "fresh set_data_directory must leave BLOB_STORE uninitialised"
    );
    // Sibling singletons must also be uninitialised at this point.
    assert!(!is_message_store_initialized_for_tests());
    assert!(!is_session_store_initialized_for_tests());

    // @step When codelet_core::persistence::store_blob is called once with a 20000 byte buffer
    let payload = vec![0u8; 20_000];
    let _hash = codelet_core::persistence::store_blob(&payload)
        .expect("store_blob must succeed against the fresh data dir");

    // @step Then codelet_core::persistence::is_blob_store_initialized_for_tests returns true
    assert!(
        is_blob_store_initialized_for_tests(),
        "store_blob must lazily initialise BLOB_STORE"
    );

    // @step And codelet_core::persistence::is_message_store_initialized_for_tests still returns false and codelet_core::persistence::is_session_store_initialized_for_tests still returns false
    assert!(
        !is_message_store_initialized_for_tests(),
        "store_blob must NOT initialise MESSAGE_STORE (BUG-122 per-store laziness)"
    );
    assert!(
        !is_session_store_initialized_for_tests(),
        "store_blob must NOT initialise SESSION_STORE (BUG-122 per-store laziness)"
    );
}
