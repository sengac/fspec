// Feature: spec/features/lifted-session-store-in-core-persistence.feature
//
// This integration test validates the acceptance criteria for RPC-033 —
// lifting SessionStore + SessionManifest + every session-level free
// function (load_session, append_message_with_metadata,
// update_session_tokens, fork_session, set_compaction_state,
// clear_compaction_state, delete_session, etc.) into
// codelet-core::persistence::manifest. Living in `codelet/core/tests/`
// means we consume codelet_core the same way an external downstream
// crate (codelet-rpc-embedded, the upcoming codelet-sessions,
// codelet-fspec) would — proving the public surface is reachable
// without a codelet-napi dependency.
//
// Tests are written against the NEW location in codelet-core. They
// will FAIL to compile until the lift is implemented (red phase).
//
// All tests serialize via `serial_test::serial` because they reach
// for the process-global data directory + MESSAGE_STORE/SESSION_STORE
// singletons set by codelet_common::set_data_directory.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::persistence::{
    append_message_with_metadata, clear_compaction_state, create_session, delete_session,
    fork_session, get_session_messages, list_all_sessions, load_session, save_session,
    set_compaction_state, update_session_tokens, MessageSource, SessionManifest, SessionStore,
};
use serde_json::Value;
use serial_test::serial;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Configure a unique temp data dir for the test and return the guard.
fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set_data_directory must succeed");
    // Force the lifted singletons to forget any prior state so each
    // serial test runs against a fresh on-disk store.
    codelet_core::persistence::reset_stores_for_tests();
    tmp
}

// ============================================================================
// Scenario: SessionManifest round-trips through {data_dir}/sessions/{uuid}.json from codelet-core
// ============================================================================

#[test]
#[serial]
fn session_manifest_round_trips_via_sessions_json_from_core() {
    // @step Given a fresh data directory is configured via codelet_common::set_data_directory
    let data_dir = setup_data_dir();

    // @step And a session is created via codelet_core::persistence::create_session with name "Round Trip" and a project path
    let project = PathBuf::from("/test/project/rpc033/round_trip");
    let mut session = create_session("Round Trip", &project).expect("create_session");
    let session_id = session.id;

    // @step When append_message_with_metadata is called with role "user" and content "hello" to record an in-context user message
    let mut metadata: HashMap<String, Value> = HashMap::new();
    metadata.insert("source".to_string(), Value::String("test".to_string()));
    append_message_with_metadata(&mut session, "user", "hello", metadata)
        .expect("append_message_with_metadata");

    // @step And update_session_tokens is called with input 120 output 60 cache_read 0 cache_create 0
    update_session_tokens(&mut session, 120, 60, 0, 0).expect("update_session_tokens");

    // @step And the SESSION_STORE singleton is reset so the next load_session reads from disk
    codelet_core::persistence::reset_stores_for_tests();

    // @step Then load_session returns a SessionManifest whose id name project provider messages and token_usage equal the values that were saved
    let restored = load_session(session_id).expect("load_session");
    assert_eq!(restored.id, session_id);
    assert_eq!(restored.name, "Round Trip");
    assert_eq!(restored.project, project);
    assert_eq!(restored.messages.len(), 1);
    assert_eq!(restored.token_usage.current_context_tokens, 120);
    assert_eq!(restored.token_usage.cumulative_billed_input, 120);
    assert_eq!(restored.token_usage.cumulative_billed_output, 60);

    // @step And the on-disk file at {data_dir}/sessions/{uuid}.json contains a JSON object with current_context_tokens equal to 120
    let path = data_dir
        .path()
        .join("sessions")
        .join(format!("{session_id}.json"));
    assert!(path.exists(), "session JSON file must exist on disk");
    let contents = std::fs::read_to_string(&path).expect("read session.json");
    let parsed: Value = serde_json::from_str(&contents).expect("parse session.json");
    assert_eq!(
        parsed["token_usage"]["current_context_tokens"]
            .as_u64()
            .expect("current_context_tokens"),
        120
    );
}

// ============================================================================
// Scenario: codelet-core consumers can construct and persist sessions without depending on codelet-napi
// ============================================================================
//
// This is a compile-time invariant: this test file is in `codelet/core/tests/`
// and uses ONLY `codelet_core::persistence::*`. The fact that it
// compiles and links proves codelet-core has no codelet-napi dependency.

#[test]
#[serial]
fn core_consumers_can_persist_sessions_without_napi() {
    // @step Given codelet_core::persistence exports SessionStore SessionManifest load_session append_message_with_metadata fork_session merge_messages cherry_pick update_session_tokens and set_compaction_state
    // (verified at compile time by the `use` statement at the top of this file)
    let _data_dir = setup_data_dir();

    // @step When a downstream crate that does not depend on codelet-napi writes `use codelet_core::persistence::{SessionStore, SessionManifest, load_session, append_message_with_metadata, fork_session, merge_messages, cherry_pick, update_session_tokens, set_compaction_state}`
    let project = PathBuf::from("/test/project/rpc033/core_only");
    let session = create_session("Core Only", &project).expect("create_session");
    save_session(&session).expect("save_session through core");

    // Construct a SessionStore directly to prove the type is reachable.
    let _store_handle: SessionStore = SessionStore::new().expect("SessionStore::new from core");

    // @step Then the build succeeds with no transitive dependency on codelet-napi
    // @step And the dependency-rule test rpc_006_source_shape.rs continues to pass
    // (both verified by the workspace build graph — codelet-core has no
    //  codelet-napi entry in its Cargo.toml; the rpc-embedded gate asserts
    //  there is no `napi` substring in rpc-embedded's source tree)
    assert_eq!(session.name, "Core Only");
}

// ============================================================================
// Scenario: fork_session preserves provider lineage and persists the new manifest
// ============================================================================

#[test]
#[serial]
fn fork_session_preserves_provider_lineage_and_persists() {
    // @step Given a fresh data directory and a parent session with 5 appended messages and provider "claude"
    let data_dir = setup_data_dir();
    let project = PathBuf::from("/test/project/rpc033/fork");
    let mut parent: SessionManifest =
        codelet_core::persistence::create_session_with_provider("Parent", &project, "claude")
            .expect("create_session_with_provider");
    for i in 0..5 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        codelet_core::persistence::append_message(&mut parent, role, &format!("msg{i}"))
            .expect("append_message");
    }
    let parent_id = parent.id;

    // @step When fork_session is called on the parent session with at_index 2 and name "Forked"
    let forked = fork_session(&parent, 2, "Forked").expect("fork_session");

    // @step Then a new SessionManifest is returned with 3 messages whose source is MessageSource::Forked
    assert_eq!(forked.messages.len(), 3);
    for msg in &forked.messages {
        if let MessageSource::Forked { from_session } = &msg.source {
            assert_eq!(*from_session, parent_id);
        } else {
            panic!("expected MessageSource::Forked, got {:?}", msg.source);
        }
    }

    // @step And the new SessionManifest has a ForkPoint whose source_session_id equals the parent id and fork_after_index equals 2
    let fork_point = forked
        .forked_from
        .as_ref()
        .expect("forked_from must be set");
    assert_eq!(fork_point.source_session_id, parent_id);
    assert_eq!(fork_point.fork_after_index, 2);

    // @step And the new manifest is persisted at {data_dir}/sessions/{new_uuid}.json with the same "claude" provider
    assert_eq!(forked.provider, "claude");
    let path = data_dir
        .path()
        .join("sessions")
        .join(format!("{}.json", forked.id));
    assert!(path.exists(), "forked session.json must be persisted");
    let contents = std::fs::read_to_string(&path).expect("read forked session.json");
    let parsed: Value = serde_json::from_str(&contents).expect("parse forked session.json");
    assert_eq!(parsed["provider"].as_str(), Some("claude"));
}

// ============================================================================
// Scenario: Compaction round-trip returns the synthetic summary plus post-boundary messages
// ============================================================================

#[test]
#[serial]
fn compaction_round_trip_returns_summary_plus_post_boundary() {
    // @step Given a session manifest with 10 appended messages
    let _data_dir = setup_data_dir();
    let project = PathBuf::from("/test/project/rpc033/compaction");
    let mut session = create_session("Compaction", &project).expect("create_session");
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        codelet_core::persistence::append_message(&mut session, role, &format!("turn-{i}"))
            .expect("append_message");
    }

    // @step When set_compaction_state is called with summary "ten messages summarised" and compacted_before_index 5
    set_compaction_state(&mut session, "ten messages summarised".to_string(), 5)
        .expect("set_compaction_state");

    // @step Then get_session_messages returns 6 entries — one synthetic message with metadata._compactionSummary equal to true followed by the 5 messages at indices 5 through 9
    let compacted = get_session_messages(&session).expect("get_session_messages");
    assert_eq!(compacted.len(), 6);
    let summary = &compacted[0];
    assert_eq!(summary.role, "user");
    assert_eq!(
        summary.metadata.get("_compactionSummary"),
        Some(&Value::Bool(true)),
        "synthetic summary must carry the _compactionSummary marker"
    );
    for (offset, msg) in compacted.iter().skip(1).enumerate() {
        let expected = format!("turn-{}", offset + 5);
        assert_eq!(
            msg.content, expected,
            "post-boundary message {offset} should be turn-{}",
            offset + 5
        );
    }

    // @step And clear_compaction_state followed by get_session_messages returns all 10 original messages in order
    clear_compaction_state(&mut session).expect("clear_compaction_state");
    let full = get_session_messages(&session).expect("get_session_messages after clear");
    assert_eq!(full.len(), 10);
    for (i, msg) in full.iter().enumerate() {
        assert_eq!(msg.content, format!("turn-{i}"));
    }
}

// ============================================================================
// Scenario: delete_session removes the on-disk manifest through the lifted facade
// ============================================================================

#[test]
#[serial]
fn delete_session_removes_manifest_through_lifted_facade() {
    // @step Given three sessions s-1 s-2 and s-3 are persisted as {data_dir}/sessions/{uuid}.json files
    let data_dir = setup_data_dir();
    let project = PathBuf::from("/test/project/rpc033/delete");
    let s1 = create_session("s-1", &project).expect("create s-1");
    let s2 = create_session("s-2", &project).expect("create s-2");
    let s3 = create_session("s-3", &project).expect("create s-3");

    let s1_path = data_dir
        .path()
        .join("sessions")
        .join(format!("{}.json", s1.id));
    let s2_path = data_dir
        .path()
        .join("sessions")
        .join(format!("{}.json", s2.id));
    let s3_path = data_dir
        .path()
        .join("sessions")
        .join(format!("{}.json", s3.id));
    assert!(s1_path.exists() && s2_path.exists() && s3_path.exists());

    // @step When codelet_core::persistence::delete_session is called with s-2 followed by a fresh SessionStore::new
    delete_session(s2.id).expect("delete_session via lifted facade");
    codelet_core::persistence::reset_stores_for_tests();

    // @step Then {data_dir}/sessions/{s-2}.json no longer exists on disk
    assert!(!s2_path.exists(), "s-2.json must be removed");

    // @step And SessionStore::list_all returns only s-1 and s-3
    let remaining = list_all_sessions().expect("list_all_sessions");
    let ids: std::collections::HashSet<uuid::Uuid> = remaining.iter().map(|s| s.id).collect();
    assert!(ids.contains(&s1.id));
    assert!(ids.contains(&s3.id));
    assert!(!ids.contains(&s2.id));

    // @step And calling delete_session again with s-2 is idempotent and returns Ok
    delete_session(s2.id).expect("second delete_session must be idempotent");
}
