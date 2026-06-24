// Feature: spec/features/lifted-blob-store-in-core-persistence.feature
//
// This integration test validates the acceptance criteria for RPC-034 —
// lifting BlobStore + the BLOB_STORE singleton + the
// store_blob/get_blob/blob_exists free-function facade + should_use_blob_storage
// + the envelope blob helpers (BLOB_REF_PREFIX, is_blob_reference,
// extract_blob_hash, make_blob_reference, process_envelope_for_blob_storage,
// rehydrate_envelope_blobs) into codelet-core::persistence::blob and
// codelet-core::persistence::blob_processing. Living in
// `codelet/core/tests/` means we consume codelet_core the same way an
// external downstream crate (codelet-rpc-embedded, the upcoming
// codelet-sessions, codelet-fspec) would — proving the public surface
// is reachable without a codelet-napi dependency.
//
// Tests are written against the NEW location in codelet-core. They
// will FAIL to compile until the lift is implemented (red phase).
//
// All tests serialize via `serial_test::serial` because they reach for
// the process-global data directory + BLOB_STORE/MESSAGE_STORE/SESSION_STORE
// singletons set by codelet_common::set_data_directory.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Utc;
use codelet_core::persistence::{
    blob_exists, extract_blob_hash, get_blob, is_blob_reference, make_blob_reference,
    process_envelope_for_blob_storage, rehydrate_envelope_blobs, should_use_blob_storage,
    store_blob, AssistantContent, AssistantMessage, BlobStore, DocumentSource, ImageSource,
    MessageEnvelope, MessagePayload, UserContent, UserMessage, BLOB_REF_PREFIX,
};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

/// Configure a unique temp data dir for the test, reset every persistence
/// singleton (MESSAGE_STORE + SESSION_STORE + history + the freshly-lifted
/// BLOB_STORE), and return the guard.
fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set_data_directory must succeed");
    // After RPC-034 this widened helper also resets the lifted BLOB_STORE
    // singleton so the next blob operation re-initialises against the
    // new directory.
    codelet_core::persistence::reset_stores_for_tests();
    tmp
}

// ============================================================================
// Scenario: Envelope tool_result round-trips through the lifted blob storage in codelet-core
// ============================================================================

#[test]
#[serial]
fn tool_result_round_trips_through_lifted_blob_storage_in_codelet_core() {
    // @step Given a fresh data directory is configured via codelet_common::set_data_directory
    let _data_dir = setup_data_dir();

    // @step And a user MessageEnvelope is constructed with one UserContent::ToolResult whose content is a 15KB string of ASCII bytes
    let large_content = "x".repeat(15_000);
    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_rpc034_round_trip".to_string(),
                content: large_content.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // @step When codelet_core::persistence::process_envelope_for_blob_storage is called on the envelope
    let (processed, blob_refs) =
        process_envelope_for_blob_storage(&envelope).expect("process_envelope_for_blob_storage");

    // @step Then the processed envelope's UserContent::ToolResult content is exactly the string "blob:sha256:" followed by a 64 hex character SHA-256 digest
    assert_eq!(blob_refs.len(), 1, "exactly one blob ref must be produced");
    let (_key, hash) = &blob_refs[0];
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert!(
                    content.starts_with(BLOB_REF_PREFIX),
                    "tool_result content must be replaced by a blob reference"
                );
                assert_eq!(
                    content,
                    &format!("{BLOB_REF_PREFIX}{hash}"),
                    "the blob reference must be the canonical blob:sha256:<hash> form"
                );
                assert_eq!(
                    hash.len(),
                    64,
                    "hash must be a 64 hex character SHA-256 digest"
                );
                assert!(
                    hash.chars().all(|c| c.is_ascii_hexdigit()),
                    "hash must be lowercase ASCII hex"
                );
            }
            _ => panic!("expected UserContent::ToolResult"),
        },
        _ => panic!("expected MessagePayload::User"),
    }

    // @step And codelet_core::persistence::get_blob called with that hash returns the original 15KB byte sequence
    let blob_data = get_blob(hash).expect("get_blob via codelet-core");
    assert_eq!(blob_data.len(), 15_000);
    assert_eq!(String::from_utf8_lossy(&blob_data), large_content);

    // @step And codelet_core::persistence::rehydrate_envelope_blobs called on the processed envelope JSON returns an envelope JSON whose tool_result content equals the original 15KB string
    let processed_json = serde_json::to_string(&processed).expect("serialize processed envelope");
    let rehydrated_json = rehydrate_envelope_blobs(&processed_json).expect("rehydrate");
    let rehydrated: MessageEnvelope =
        serde_json::from_str(&rehydrated_json).expect("parse rehydrated envelope");
    match &rehydrated.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert_eq!(
                    *content, large_content,
                    "rehydrated tool_result content must equal the original 15KB string"
                );
            }
            _ => panic!("expected UserContent::ToolResult"),
        },
        _ => panic!("expected MessagePayload::User"),
    }
}

// ============================================================================
// Scenario: Identical blobs deduplicate to a single on-disk file
// ============================================================================

#[test]
#[serial]
fn identical_blobs_deduplicate_to_a_single_on_disk_file() {
    // @step Given a fresh data directory is configured via codelet_common::set_data_directory
    let data_dir = setup_data_dir();

    // @step And a 5MB buffer of deterministic bytes is prepared
    let buffer: Vec<u8> = (0..(5 * 1024 * 1024)).map(|i| (i % 251) as u8).collect();
    assert_eq!(buffer.len(), 5 * 1024 * 1024);

    // @step When codelet_core::persistence::store_blob is called twice with that same buffer
    let hash_a = store_blob(&buffer).expect("first store_blob");
    let hash_b = store_blob(&buffer).expect("second store_blob");

    // @step Then both calls return the identical SHA-256 hash string
    assert_eq!(
        hash_a, hash_b,
        "identical content must hash to the same value"
    );
    assert_eq!(hash_a.len(), 64);

    // @step And exactly one file exists at {data_dir}/blobs/{first2hex}/{full_hash} and no .tmp file is left behind
    let blobs_dir = data_dir.path().join("blobs");
    let subdir = blobs_dir.join(&hash_a[0..2]);
    let blob_path = subdir.join(&hash_a);
    assert!(
        blob_path.exists(),
        "blob file must exist at the canonical path"
    );

    let entries: Vec<_> = fs::read_dir(&subdir)
        .expect("read blob subdir")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "exactly one file must live in the subdir (no .tmp leftover)"
    );
    let only = entries[0]
        .file_name()
        .into_string()
        .expect("blob file name is utf-8");
    assert_eq!(only, hash_a, "the only file must be the canonical blob");
    assert!(
        !only.ends_with(".tmp"),
        "no .tmp leftover may remain after the second store"
    );
}

// ============================================================================
// Scenario: codelet-core consumers can hash-store and rehydrate envelope blobs without depending on codelet-napi
// ============================================================================
//
// This is a compile-time invariant: this test file is in `codelet/core/tests/`
// and uses ONLY `codelet_core::persistence::*`. The fact that it compiles
// and links proves codelet-core has no codelet-napi dependency.

#[test]
#[serial]
fn core_consumers_can_hash_store_and_rehydrate_blobs_without_napi() {
    // @step Given codelet_core::persistence exports BlobStore store_blob get_blob blob_exists is_blob_reference extract_blob_hash make_blob_reference process_envelope_for_blob_storage rehydrate_envelope_blobs should_use_blob_storage and BLOB_REF_PREFIX
    // (verified at compile time by the `use` statement at the top of this file)
    let _data_dir = setup_data_dir();

    // @step When a downstream crate that does not depend on codelet-napi writes `use codelet_core::persistence::{BlobStore, store_blob, get_blob, blob_exists, is_blob_reference, extract_blob_hash, make_blob_reference, process_envelope_for_blob_storage, rehydrate_envelope_blobs, should_use_blob_storage, BLOB_REF_PREFIX}`
    let payload = b"core-only blob payload that is large enough to be blobified".repeat(300);
    assert!(should_use_blob_storage(&payload));
    let hash = store_blob(&payload).expect("store_blob from core-only consumer");
    assert!(blob_exists(&hash).expect("blob_exists from core-only consumer"));
    let reference = make_blob_reference(&hash);
    assert!(is_blob_reference(&reference));
    assert_eq!(extract_blob_hash(&reference), Some(hash.as_str()));

    // Construct a BlobStore directly to prove the type is reachable.
    let _handle: BlobStore = BlobStore::new().expect("BlobStore::new from core");

    // @step Then the build succeeds with no transitive dependency on codelet-napi
    // @step And the dependency-rule test rpc_006_source_shape.rs continues to pass
    // (both verified by the workspace build graph — codelet-core has no
    //  codelet-napi entry in its Cargo.toml; the rpc-embedded gate asserts
    //  there is no `napi` substring in rpc-embedded's source tree)
    let round_trip = get_blob(&hash).expect("get_blob round trip");
    assert_eq!(round_trip, payload);
}

// ============================================================================
// Scenario: Pre-lift blob files at {data_dir}/blobs/{first2}/{hash} remain resolvable
// ============================================================================

#[test]
#[serial]
fn pre_lift_blob_files_remain_resolvable_through_codelet_core() {
    // @step Given a fresh data directory is configured via codelet_common::set_data_directory
    let data_dir = setup_data_dir();

    // @step And a blob file is manually written at {data_dir}/blobs/{first2hex}/{full_64_hex} containing a known byte sequence, where {full_64_hex} is the SHA-256 of those bytes and {first2hex} is its first two hex characters
    use sha2::{Digest, Sha256};
    let pre_lift_bytes = b"bytes written by the pre-lift NAPI BlobStore implementation".to_vec();
    let mut hasher = Sha256::new();
    hasher.update(&pre_lift_bytes);
    let digest = hasher.finalize();
    let hash = hex::encode(digest);
    let first2 = &hash[0..2];
    let blobs_dir = data_dir.path().join("blobs");
    fs::create_dir_all(blobs_dir.join(first2)).expect("create blob subdir");
    let pre_lift_path = blobs_dir.join(first2).join(&hash);
    fs::write(&pre_lift_path, &pre_lift_bytes).expect("write pre-lift blob");

    // @step When codelet_core::persistence::get_blob is called with that hash string
    let resolved = get_blob(&hash).expect("get_blob must read pre-lift on-disk blob");

    // @step Then the returned bytes equal the bytes that were manually written
    assert_eq!(resolved, pre_lift_bytes);

    // @step And codelet_core::persistence::blob_exists with the same hash returns true
    assert!(
        blob_exists(&hash).expect("blob_exists must observe pre-lift blob"),
        "pre-lift blob must be observable via blob_exists"
    );
}

// ============================================================================
// Scenario: BLOB_REF_PREFIX wire-format value is preserved exactly
// ============================================================================

#[test]
#[serial]
fn blob_ref_prefix_wire_format_value_is_preserved_exactly() {
    // @step Given codelet_core::persistence::BLOB_REF_PREFIX is imported
    // (verified at compile time by the `use` statement at the top of this file)

    // @step When the constant value is inspected and combined with a 64 hex character hash via make_blob_reference
    let hash = "a".repeat(64);
    let reference = make_blob_reference(&hash);

    // @step Then BLOB_REF_PREFIX equals the string "blob:sha256:" exactly
    assert_eq!(
        BLOB_REF_PREFIX, "blob:sha256:",
        "wire-format prefix MUST remain the on-wire literal"
    );

    // @step And is_blob_reference returns true for the combined string and extract_blob_hash returns Some with the original 64 hex characters
    assert!(is_blob_reference(&reference));
    assert_eq!(extract_blob_hash(&reference), Some(hash.as_str()));

    // Pre-lift blob:md5:<hash> wire format must remain rejected
    let md5_form = format!("blob:md5:{hash}");
    assert!(!is_blob_reference(&md5_form));
    assert_eq!(extract_blob_hash(&md5_form), None);
}

// ============================================================================
// Scenario: should_use_blob_storage preserves the 10KB threshold from the pre-lift implementation
// ============================================================================

#[test]
#[serial]
fn should_use_blob_storage_preserves_10kb_threshold_from_pre_lift() {
    // @step Given codelet_core::persistence::should_use_blob_storage is imported
    // (verified at compile time by the `use` statement at the top of this file)

    // @step When should_use_blob_storage is called with a 100 byte slice and again with a 20000 byte slice
    let small = vec![0u8; 100];
    let large = vec![0u8; 20_000];

    // @step Then the 100 byte call returns false and the 20000 byte call returns true
    assert!(
        !should_use_blob_storage(&small),
        "100 byte payload must stay inline"
    );
    assert!(
        should_use_blob_storage(&large),
        "20000 byte payload must be eligible for blob storage"
    );

    // The exact 10240 (10 * 1024) byte boundary is preserved: 10240 is NOT
    // greater than the threshold (uses `>`, not `>=`), so it stays inline.
    let exactly_10kb = vec![0u8; 10 * 1024];
    assert!(
        !should_use_blob_storage(&exactly_10kb),
        "exactly-10KB payload must stay inline (preserves the `>` boundary)"
    );
    let just_over = vec![0u8; 10 * 1024 + 1];
    assert!(
        should_use_blob_storage(&just_over),
        "one byte over the threshold must be eligible for blob storage"
    );
}

// ============================================================================
// Extra: prove AssistantContent + DocumentSource + ImageSource variants
// are reachable via the same codelet_core re-export (Phase 1.4 requires the
// process_envelope_for_blob_storage matcher to handle every variant).
// ============================================================================

#[test]
#[serial]
fn assistant_thinking_and_image_blob_storage_through_core() {
    let _data_dir = setup_data_dir();

    let large_thinking = "z".repeat(20_000);
    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_thinking_rpc034".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![AssistantContent::Thinking {
                thinking: large_thinking.clone(),
                signature: Some("sig_rpc034".to_string()),
            }],
            stop_reason: None,
            usage: None,
        }),
        request_id: None,
    };

    let (processed, blob_refs) =
        process_envelope_for_blob_storage(&envelope).expect("assistant thinking blob");
    assert_eq!(blob_refs.len(), 1);

    match &processed.message {
        MessagePayload::Assistant(msg) => match &msg.content[0] {
            AssistantContent::Thinking { thinking, .. } => {
                assert!(thinking.starts_with(BLOB_REF_PREFIX));
            }
            _ => panic!("expected Thinking content"),
        },
        _ => panic!("expected Assistant message"),
    }

    let rehydrated_json =
        rehydrate_envelope_blobs(&serde_json::to_string(&processed).expect("serialize"))
            .expect("rehydrate thinking blob");
    let rehydrated: MessageEnvelope = serde_json::from_str(&rehydrated_json).expect("parse");
    match &rehydrated.message {
        MessagePayload::Assistant(msg) => match &msg.content[0] {
            AssistantContent::Thinking { thinking, .. } => {
                assert_eq!(*thinking, large_thinking);
            }
            _ => panic!("expected Thinking content"),
        },
        _ => panic!("expected Assistant message"),
    }

    // Sanity check that DocumentSource + ImageSource symbols round-trip
    // through codelet_core (proving the blob_processing.rs imports are
    // satisfied by the lifted message_envelope types).
    let _image = ImageSource::Base64 {
        media_type: "image/png".to_string(),
        data: "small".to_string(),
    };
    let _document = DocumentSource::Base64 {
        media_type: "application/pdf".to_string(),
        data: "small".to_string(),
    };
}
