//! Feature: spec/features/durable-session-manifest-transactions.feature

use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use serde_json::json;
use std::fs;
use std::sync::{Arc, Barrier};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn configure(path: &std::path::Path) -> Result<(), String> {
    codelet_common::set_data_directory(path.to_path_buf())?;
    reset_stores_for_tests();
    Ok(())
}

#[test]
#[serial_test::serial]
fn missing_committed_message_references_block_replay() -> TestResult {
    // @step Given a committed session referencing an unavailable original message
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let mut session = create_session("missing original", directory.path())?;
    append_message(&mut session, "user", "required original")?;
    let mut compacted = create_session("missing covered original", directory.path())?;
    append_message(&mut compacted, "user", "required covered original")?;
    set_compaction_state(
        &mut compacted,
        "<dag-node depth=\"D1\" turns=\"0-0\" label=\"memory\">covered</dag-node>".into(),
        1,
    )?;
    fs::remove_file(directory.path().join("messages/messages.jsonl"))?;
    reset_stores_for_tests();
    // @step When a cold reader loads the session replay envelopes
    let result = get_session_message_envelopes(session.id);
    // @step Then replay fails explicitly instead of omitting the committed message
    assert!(
        result.is_err(),
        "missing committed reference must block replay: {result:?}"
    );
    let compacted_result = get_session_message_envelopes(compacted.id);
    assert!(
        compacted_result.is_err(),
        "a summary must not hide a missing committed original: {compacted_result:?}"
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn provider_session_creation_publishes_the_committed_revision() -> TestResult {
    // @step Given a fresh session store
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let mut store = SessionStore::new()?;
    // @step When a session is created with an explicit provider
    let session = store.create_with_provider("provider", directory.path(), "codex")?;
    // @step Then the returned cached and durable snapshots share the committed revision
    assert!(session.revision > 0);
    assert_eq!(
        store
            .get(session.id)
            .ok_or("missing cached session")?
            .revision,
        session.revision
    );
    assert_eq!(store.load(session.id)?.revision, session.revision);
    Ok(())
}

#[test]
#[serial_test::serial]
fn concurrent_field_writers_retain_all_committed_updates() -> TestResult {
    // @step Given token provider and message writers share one persisted session
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let session = create_session("concurrent", directory.path())?;
    let barrier = Arc::new(Barrier::new(3));
    let mut writers = Vec::new();
    // @step When barrier synchronized writers commit their owned fields
    for writer in 0..3 {
        let mut snapshot = session.clone();
        let barrier = barrier.clone();
        writers.push(std::thread::spawn(move || {
            barrier.wait();
            match writer {
                0 => update_session_tokens(&mut snapshot, 101, 17, 0, 0, 3),
                1 => append_message_with_metadata(
                    &mut snapshot,
                    "assistant",
                    "",
                    [("providerCheckpoint".into(), json!({"generation": 7}))]
                        .into_iter()
                        .collect(),
                )
                .map(|_| ()),
                _ => append_message(&mut snapshot, "user", "immutable original").map(|_| ()),
            }
        }));
    }
    for writer in writers {
        writer.join().map_err(|_| "writer thread failed")??;
    }
    // @step Then token usage provider metadata and every message reference survive cold open
    reset_stores_for_tests();
    let restored = load_session(session.id)?;
    assert_eq!(restored.token_usage.current_context_tokens, 101);
    assert_eq!(restored.messages.len(), 2);
    let messages = get_session_messages_full(&restored)?;
    assert!(messages
        .iter()
        .any(|message| message.content == "immutable original"));
    assert!(messages.iter().any(
        |message| message.metadata.get("providerCheckpoint") == Some(&json!({"generation": 7}))
    ));
    Ok(())
}

#[test]
#[serial_test::serial]
fn stale_whole_snapshots_cannot_overwrite_a_newer_commit() -> TestResult {
    // @step Given two snapshots of the same committed session
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let mut first = create_session("original", directory.path())?;
    let mut stale = first.clone();
    // @step When one snapshot commits and the other attempts a whole snapshot save
    first.provider = "new-provider".into();
    save_session(&first)?;
    stale.name = "stale overwrite".into();
    let result = save_session(&stale);
    // @step Then the stale save reports a revision conflict and preserves the newer commit
    assert!(result.is_err(), "stale whole snapshot must conflict");
    assert!(result
        .err()
        .is_some_and(|error| error.contains("revision conflict")));
    reset_stores_for_tests();
    let restored = load_session(first.id)?;
    assert_eq!(restored.provider, "new-provider");
    assert_eq!(restored.name, "original");
    Ok(())
}

#[test]
#[serial_test::serial]
fn failed_promotion_cannot_publish_a_changed_cache() -> TestResult {
    // @step Given a readable committed session and an unwritable promotion directory
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let mut store = SessionStore::new()?;
    let session = store.create("original", directory.path())?;
    let path = directory
        .path()
        .join("sessions")
        .join(format!("{}.json", session.id));
    let before = fs::read(&path)?;
    let sessions = path.parent().ok_or("sessions directory")?;
    let permissions = fs::metadata(sessions)?.permissions();
    let mut readonly = permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(sessions, readonly)?;
    assert_eq!(fs::read(&path)?, before);
    assert_eq!(
        SessionStore::new()?.load(session.id)?.revision,
        session.revision
    );
    // @step When a rename attempts to commit a new session name
    let result = store.rename(session.id, "uncommitted");
    // @step Then the write fails and both cached and durable state remain unchanged before permissions are restored
    assert!(result.is_err(), "staging must fail: {result:?}");
    assert_eq!(
        store.get(session.id).ok_or("missing cached session")?.name,
        "original"
    );
    assert_eq!(
        serde_json::to_value(store.get(session.id).ok_or("cached session")?)?,
        serde_json::to_value(&session)?
    );
    assert_eq!(fs::read(&path)?, before);
    assert_eq!(
        serde_json::to_value(SessionStore::new()?.load(session.id)?)?,
        serde_json::to_value(&session)?
    );
    fs::set_permissions(sessions, permissions)?;
    Ok(())
}

#[test]
#[serial_test::serial]
fn interrupted_staging_preserves_the_committed_revision() -> TestResult {
    // @step Given an unpromoted staging file beside a committed session
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let session = create_session("committed", directory.path())?;
    fs::write(
        directory
            .path()
            .join("sessions")
            .join(format!("{}.stage", session.id)),
        "{partial",
    )?;
    // @step When a fresh store opens the session
    let restored = SessionStore::new()?.load(session.id)?;
    // @step Then only the old committed revision is visible
    assert_eq!(restored.name, "committed");
    assert!(restored.messages.is_empty());
    Ok(())
}

#[test]
#[serial_test::serial]
fn competing_processes_cannot_mutate_an_owned_session() -> TestResult {
    if let Ok(path) = std::env::var("CMPCT064_CHILD_STORE") {
        configure(std::path::Path::new(&path))?;
        let mut store = SessionStore::new()?;
        let session = (*store.list_all().first().ok_or("missing session")?).clone();
        let result = store.save(&session);
        assert!(result.is_err(), "competing process must not save");
        assert!(result
            .err()
            .is_some_and(|error| error.contains("ownership")));
        return Ok(());
    }
    // @step Given a process owns the session writer lock
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let _session = create_session("owned", directory.path())?;
    // @step When another process attempts to save that session
    let child = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "competing_processes_cannot_mutate_an_owned_session",
            "--nocapture",
        ])
        .env("CMPCT064_CHILD_STORE", directory.path())
        .output()?;
    // @step Then the competing process receives an explicit ownership failure
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn unreadable_committed_manifests_are_not_absent_sessions() -> TestResult {
    // @step Given an incompatible committed manifest on disk
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let session = create_session("valid", directory.path())?;
    fs::write(
        directory
            .path()
            .join("sessions")
            .join(format!("{}.json", session.id)),
        "{broken",
    )?;
    // @step When a fresh store opens the sessions directory
    let result = SessionStore::new();
    // @step Then opening fails explicitly instead of silently omitting the session
    assert!(
        result.is_err(),
        "corruption must not become an absent session"
    );
    Ok(())
}
