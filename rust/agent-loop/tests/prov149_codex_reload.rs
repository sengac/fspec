use codelet_core::persistence::{get_session_message_envelopes, reset_stores_for_tests};
use codelet_core::SessionManagerHandle;
use codelet_providers::codex::history::CodexHistory;
use codelet_providers::codex::session::CodexSession;

fn history(
    session: &codelet_cli::session::Session,
) -> Result<&CodexHistory, Box<dyn std::error::Error>> {
    Ok(&session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("history missing")?
        .history)
}
use codelet_sessions::SessionManager;
use serde_json::json;
use serial_test::serial;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn setup() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::fs::create_dir_all(directory.path().join("cache"))?;
    std::fs::write(
        directory.path().join("cache/models.json"),
        include_str!("fixtures/prov101_models.json"),
    )?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    Ok(directory)
}

fn seed(history: &CodexHistory) -> TestResult {
    history.commit(&json!({"model":"test","input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]}]}), &[
        json!({"type":"reasoning","id":"r","summary":[],"encrypted_content":"synthetic-secret"}),
        json!({"type":"message","role":"assistant","id":"m","phase":"final_answer","content":[{"type":"output_text","text":"Hi","annotations":[]}]}),
    ])?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn backend_checkpoint_survives_session_reload_and_new_user_turn() -> TestResult {
    // @step Given a completed Codex turn persisted by the backend
    let _directory = setup()?;
    let manager = SessionManager::new();
    manager.set_default_model("anthropic/claude-opus-4-5");
    let session_id = SessionManagerHandle::create_session(&manager, None);
    let session = manager.get_session(&session_id.value)?;
    {
        let mut inner = session.inner.lock().await;
        inner.provider_session.ensure_provider("codex");
        inner.bind_provider_persistence(session.id)?;
        seed(history(&inner)?)?;
    }
    reset_stores_for_tests();
    let envelopes = get_session_message_envelopes(session.id)?;
    assert!(envelopes
        .iter()
        .any(|entry| entry.contains("codexContinuation")));
    let restored_manager = SessionManager::new();
    let manifest = codelet_core::persistence::load_session(session.id)?;
    restored_manager
        .create_session_from_manifest(&manifest, "anthropic/claude-opus-4-5")
        .await?;
    restored_manager
        .get_session(&session_id.value)?
        .inner
        .lock()
        .await
        .provider_session
        .ensure_provider("codex");
    // @step When its session envelopes are loaded into a fresh live session
    restored_manager.restore_session_messages(&session_id, envelopes)?;
    let fresh = restored_manager.get_session(&session_id.value)?;
    let inner = fresh.inner.lock().await;
    let history = history(&inner)?;
    // @step And a new user turn is sampled using a newly created provider
    let mut request = json!({"model":"test","input":[
        {"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]},
        {"type":"message","role":"assistant","content":[{"type":"output_text","text":"Hi","annotations":[]}]},
        {"type":"message","role":"user","content":[{"type":"input_text","text":"next"}]}
    ]});
    history.prepare(&mut request)?;
    // @step Then the request replays the saved typed items before the new user input
    assert_eq!(request["input"][1]["encrypted_content"], "synthetic-secret");
    assert_eq!(request["input"][2]["phase"], "final_answer");
    assert_eq!(request["input"][3]["content"][0]["text"], "next");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn legacy_and_non_codex_sessions_keep_ordinary_history_behavior() -> TestResult {
    // @step Given legacy envelopes without Codex state and a non-Codex session
    // @step When their histories are saved restored and sampled
    // @step Then no Codex state is added and ordinary history is unchanged
    let _directory = setup()?;
    let manager = SessionManager::new();
    manager.set_default_model("anthropic/claude-opus-4-5");
    let session_id = SessionManagerHandle::create_session(&manager, None);
    let session = manager.get_session(&session_id.value)?;
    let mut manifest = codelet_core::persistence::load_session(session.id)?;
    codelet_core::persistence::append_message(&mut manifest, "user", "legacy")?;
    {
        let mut inner = session.inner.lock().await;
        inner.provider_session.ensure_provider("codex");
        seed(history(&inner)?)?;
    }
    let envelopes = get_session_message_envelopes(session.id)?;
    assert!(!envelopes
        .iter()
        .any(|entry| entry.contains("codexContinuation")));
    manager.restore_session_messages(&session_id, envelopes)?;
    let inner = session.inner.lock().await;
    assert!(history(&inner)?.snapshot()?.is_none());
    assert!(inner.preserve_thinking_enabled);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn conversation_replacement_invalidates_typed_continuation() -> TestResult {
    // @step Given a live Codex session with saved opaque reasoning
    // @step When its history is cleared compacted or switched to another model
    // @step Then subsequent requests and reloads do not replay stale typed items
    let _directory = setup()?;
    let manager = SessionManager::new();
    manager.set_default_model("anthropic/claude-opus-4-5");
    let session_id = SessionManagerHandle::create_session(&manager, None);
    let session = manager.get_session(&session_id.value)?;
    {
        let mut inner = session.inner.lock().await;
        inner.provider_session.ensure_provider("codex");
        inner.bind_provider_persistence(session.id)?;
        seed(history(&inner)?)?;
        codelet_cli::interactive_helpers::reset_session_to_reminders(&mut inner)?;
        assert!(history(&inner)?.snapshot()?.is_none());
        seed(history(&inner)?)?;
    }
    tokio::task::block_in_place(|| session.clear_history())?;
    let envelopes = get_session_message_envelopes(session.id)?;
    let last: serde_json::Value =
        serde_json::from_str(envelopes.last().ok_or("checkpoint envelope missing")?)?;
    assert!(last["codexContinuation"].is_null());
    Ok(())
}
