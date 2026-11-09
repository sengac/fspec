//! Feature: spec/features/a-panic-in-one-sessions-message-append-never-stops-other-sessions-messages.feature
//!
//! BUG-625: the daemon-wide message store lock was held across every
//! message read, write and index rebuild, so a panic inside one session's
//! message append poisoned it and every other session's message
//! persistence failed with "poisoned lock" until the daemon restarted.
//! The panic is injected deterministically at the message.sync durability
//! boundary of session A's append, on a named worker thread; no timing is
//! involved.

use codelet_core::persistence::conversation_commit::{
    admit_stored_message, append_message_idempotent, store_original_idempotent,
};
use codelet_core::persistence::messages::durability::panic_at;
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const INJECTED: &str = "injected panic at message.sync";

fn configure(path: &std::path::Path) -> Result<(), String> {
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(path.to_path_buf())?;
    reset_stores_for_tests();
    Ok(())
}

/// Append a message to `session` on a worker thread with a panic raised
/// inside the message store, and return the messages the panic hook saw.
fn panic_inside_append(
    session: &SessionManifest,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    {
        let seen = seen.clone();
        std::panic::set_hook(Box::new(move |info| {
            let message = info
                .payload()
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "<non-string panic payload>".into());
            if let Ok(mut seen) = seen.lock() {
                seen.push(message);
            }
        }));
    }
    let mut stale = session.clone();
    let worker = std::thread::Builder::new()
        .name("bug625-worker".into())
        .spawn(move || {
            panic_at("message.sync", || {
                append_message(&mut stale, "user", "never committed")
            })
        })?;
    let joined = worker.join();
    let _ = std::panic::take_hook();
    if let Ok(result) = joined {
        return Err(format!("the append returned instead of panicking: {result:?}").into());
    }
    let seen = seen
        .lock()
        .map_err(|_| "panic hook record poisoned")?
        .clone();
    Ok(seen)
}

fn poisoned(session_id: Uuid) -> String {
    format!(
        "the manifest writer lock of session {session_id} is poisoned: an earlier update of \
         this session panicked; close and reopen the session to recover"
    )
}

/// Two sessions with one committed message each, and A's message id.
fn two_sessions(
    path: &std::path::Path,
) -> Result<(SessionManifest, SessionManifest, Uuid), Box<dyn std::error::Error>> {
    let mut a = create_session("session A", path)?;
    let mut b = create_session("session B", path)?;
    let first = append_message(&mut a, "user", "A before the panic")?;
    append_message(&mut b, "user", "B before the panic")?;
    Ok((a, b, first))
}

#[test]
#[serial_test::serial]
fn another_session_still_appends_after_one_sessions_message_append_panics() -> TestResult {
    // @step Given two open sessions A and B in one daemon
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let (a, mut b, first) = two_sessions(directory.path())?;

    // @step When session A's message append panics inside the message store
    let seen = panic_inside_append(&a)?;

    // @step Then the panic reaches the process panic hook with the message "injected panic at message.sync"
    assert_eq!(seen, vec![INJECTED.to_string()]);

    // @step And session B's next message append commits and B's manifest references the new message
    let id = append_message(&mut b, "assistant", "B after the panic")?;
    let durable = load_session(b.id)?;
    assert_eq!(durable.messages.len(), 2);
    assert_eq!(durable.messages[1].message_id, id);
    let stored = get_message(id)?.ok_or("B's new message is not in the store")?;
    assert_eq!(stored.content, "B after the panic");

    // @step And session A's committed manifest keeps its messages from before the panic
    let durable = load_session(a.id)?;
    assert_eq!(durable.revision, a.revision);
    let ids: Vec<Uuid> = durable.messages.iter().map(|m| m.message_id).collect();
    assert_eq!(ids, vec![first]);
    Ok(())
}

#[test]
#[serial_test::serial]
fn the_panicked_session_reports_its_poisoned_writer_until_it_is_reopened() -> TestResult {
    // @step Given session A's message append panicked inside the message store
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let (a, _b, _first) = two_sessions(directory.path())?;
    assert_eq!(panic_inside_append(&a)?, vec![INJECTED.to_string()]);

    // @step When session A appends a message again
    let mut stale = load_session(a.id)?;
    let again = append_message(&mut stale, "user", "after the panic");

    // @step Then the append fails with the poisoned writer error naming session A
    assert_eq!(again.err().as_deref(), Some(poisoned(a.id).as_str()));

    // @step When session A is closed, releasing its writer ownership
    release_session_ownership(a.id);

    // @step Then session A's next message append commits
    let mut reopened = load_session(a.id)?;
    let id = append_message(&mut reopened, "user", "reopened")?;
    assert_eq!(reopened.revision, a.revision + 1);
    assert_eq!(
        load_session(a.id)?.messages.last().map(|m| m.message_id),
        Some(id)
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn every_message_store_operation_still_works_after_one_sessions_append_panics() -> TestResult {
    // @step Given session A's message append panicked inside the message store
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let (a, b, _first) = two_sessions(directory.path())?;
    assert_eq!(panic_inside_append(&a)?, vec![INJECTED.to_string()]);

    // @step When session B appends a message idempotently
    let idempotent = Uuid::new_v4();
    append_message_idempotent(b.id, idempotent, "assistant", "idempotent", HashMap::new())?;
    append_message_idempotent(b.id, idempotent, "assistant", "idempotent", HashMap::new())?;

    // @step And a retained original is stored and admitted to session B
    let original = Uuid::new_v4();
    store_original_idempotent(original, "user", "retained original", HashMap::new())?;
    admit_stored_message(b.id, original)?;

    // @step And the metadata of B's message is updated
    update_message_metadata(
        idempotent,
        HashMap::from([("bug625".to_string(), serde_json::json!("updated"))]),
    )?;

    // @step Then session B's messages read back with the updated metadata
    let messages = get_session_messages(&load_session(b.id)?)?;
    let contents: Vec<&str> = messages.iter().map(|m| m.content.as_str()).collect();
    assert_eq!(
        contents,
        vec!["B before the panic", "idempotent", "retained original"]
    );
    assert_eq!(
        messages[1].metadata.get("bug625"),
        Some(&serde_json::json!("updated"))
    );

    // @step And orphaned message cleanup runs without an error
    cleanup_orphaned_messages()?;
    Ok(())
}
