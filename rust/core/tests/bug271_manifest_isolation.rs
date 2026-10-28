//! Feature: spec/features/unreadable-session-isolation.feature

use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use std::{fs, path::Path};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn configure(path: &Path) -> Result<(), String> {
    codelet_common::set_data_directory(path.to_path_buf())?;
    reset_stores_for_tests();
    Ok(())
}

fn manifest_path(root: &Path, id: Uuid) -> std::path::PathBuf {
    root.join("sessions").join(format!("{id}.json"))
}

#[test]
#[serial_test::serial]
fn empty_historical_manifests_do_not_prevent_new_sessions() -> TestResult {
    // @step Given a healthy session and an empty historical manifest
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let healthy = create_session("healthy", directory.path())?;
    let damaged = manifest_path(directory.path(), Uuid::new_v4());
    fs::write(&damaged, [])?;
    reset_stores_for_tests();
    // @step When a fresh store creates a session with an explicit provider
    let created = create_session_with_provider("new", directory.path(), "openai:test")?;
    // @step Then the new session is durable and the empty manifest is reported without changing its bytes
    assert_ne!(created.id, healthy.id);
    assert_eq!(load_session(created.id)?.provider, "openai:test");
    assert_eq!(load_session(healthy.id)?.name, "healthy");
    assert!(fs::read(damaged)?.is_empty());
    Ok(())
}

#[test]
#[serial_test::serial]
fn unreadable_sessions_do_not_block_another_project() -> TestResult {
    // @step Given a healthy project session and malformed history for another project
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let healthy = create_session("healthy", directory.path())?;
    let foreign = create_session("foreign", &directory.path().join("other"))?;
    let damaged = manifest_path(directory.path(), foreign.id);
    fs::write(&damaged, "{broken")?;
    reset_stores_for_tests();
    // @step When the user lists and resumes healthy history and starts another session
    let listed = list_sessions_for_project(directory.path())?;
    let restored = resume_last_session(directory.path())?;
    let created = create_session("new", directory.path())?;
    // @step Then healthy sessions remain available and the malformed file remains unchanged
    assert_eq!(listed.len(), 1);
    assert_eq!(restored.id, healthy.id);
    assert_ne!(created.id, foreign.id);
    assert_eq!(fs::read_to_string(damaged)?, "{broken");
    Ok(())
}

#[test]
#[serial_test::serial]
fn damaged_target_sessions_fail_explicitly() -> TestResult {
    // @step Given truncated and incompatible historical manifests
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    fs::create_dir_all(directory.path().join("sessions"))?;
    for contents in ["{", "{}", " \n\t", "{\"id\":false}"] {
        let id = Uuid::new_v4();
        let path = manifest_path(directory.path(), id);
        fs::write(&path, contents)?;
        // @step When the user loads or updates each damaged session
        let mut store = SessionStore::new()?;
        let load_error = store.load(id).err().ok_or("damaged target must fail")?;
        let update_error = store
            .rename(id, "replacement")
            .err()
            .ok_or("update must fail")?;
        // @step Then each operation reports its path and cause without changing the damaged bytes
        assert!(load_error.contains(&id.to_string()));
        assert!(load_error.contains("manifest"));
        assert!(load_error.contains(&path.display().to_string()));
        assert!(update_error.contains("manifest"));
        assert!(update_error.contains(&path.display().to_string()));
        assert_eq!(fs::read_to_string(path)?, contents);
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn supported_legacy_manifests_remain_readable_without_rewriting() -> TestResult {
    // @step Given a valid older manifest missing defaultable fields
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let session = create_session("legacy", directory.path())?;
    let path = manifest_path(directory.path(), session.id);
    let mut value = serde_json::to_value(&session)?;
    let object = value.as_object_mut().ok_or("manifest is not an object")?;
    for field in [
        "revision",
        "provider",
        "token_usage",
        "provider_state",
        "work_unit_focus",
    ] {
        object.remove(field);
    }
    let original = serde_json::to_vec(&value)?;
    fs::write(&path, &original)?;
    // @step When a fresh store reads the older session
    let store = SessionStore::new()?;
    let restored = store.load(session.id)?;
    // @step Then supported defaults are applied without rewriting the manifest
    assert_eq!(restored.revision, 0);
    assert_eq!(restored.provider, "");
    assert_eq!(restored.token_usage.current_context_tokens, 0);
    assert_eq!(fs::read(path)?, original);
    Ok(())
}

#[test]
#[serial_test::serial]
fn unknown_references_prevent_destructive_cleanup() -> TestResult {
    // @step Given an unreadable manifest and existing original messages
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let mut session = create_session("original", directory.path())?;
    append_message(&mut session, "user", "must survive")?;
    let path = directory.path().join("messages/messages.jsonl");
    let original = fs::read(&path)?;
    fs::write(manifest_path(directory.path(), session.id), "{broken")?;
    reset_stores_for_tests();
    // @step When orphan cleanup inspects committed references
    let result = cleanup_orphaned_messages();
    // @step Then cleanup fails conservatively and all original bytes remain intact
    assert!(result.is_err(), "unknown references must prevent cleanup");
    assert_eq!(fs::read(path)?, original);
    Ok(())
}

#[test]
#[serial_test::serial]
fn store_wide_failures_remain_errors() -> TestResult {
    // @step Given the sessions path is not a directory
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    fs::write(directory.path().join("sessions"), "not a directory")?;
    // @step When a fresh store initializes
    let result = SessionStore::new();
    // @step Then initialization fails explicitly rather than reporting an empty usable store
    assert!(result.is_err());
    Ok(())
}

#[test]
#[serial_test::serial]
fn repair_is_discovered_on_the_next_cold_load() -> TestResult {
    // @step Given a healthy session and a damaged historical manifest across repeated starts
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let healthy = create_session("healthy", directory.path())?;
    let repaired = create_session("repairable", directory.path())?;
    let healthy_path = manifest_path(directory.path(), healthy.id);
    let healthy_bytes = fs::read(&healthy_path)?;
    let path = manifest_path(directory.path(), repaired.id);
    let backup = fs::read(&path)?;
    fs::write(&path, "{broken")?;
    for _attempt in 0..2 {
        let store = SessionStore::new()?;
        assert_eq!(store.list_all().len(), 1);
        assert_eq!(store.load(healthy.id)?.name, "healthy");
    }
    // @step When the damaged manifest is replaced with its valid backed up contents
    fs::write(path, backup)?;
    // @step Then the next cold load discovers both sessions without rewriting healthy history
    let store = SessionStore::new()?;
    assert_eq!(store.list_all().len(), 2);
    assert_eq!(store.load(repaired.id)?.name, "repairable");
    assert_eq!(fs::read(healthy_path)?, healthy_bytes);
    Ok(())
}
