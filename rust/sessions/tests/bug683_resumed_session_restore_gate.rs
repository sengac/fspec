//! Feature: spec/features/resumed-sessions-refuse-input-until-restored.feature
//!
//! BUG-683: `resume_session` put a session in the live map before its saved
//! history and provider state were restored. Input sent during a slow restore
//! ran a turn on an empty generation-0 Codex history, whose checkpoint the
//! store then refused as a rollback below the saved generation 1, and the
//! session stayed blocked. These tests stop the restore half-way through:
//! `create_session_from_manifest` has run (the first step of
//! `resume_session`) and the restore has not, which is exactly the state the
//! user's input arrived in. The failure-message scenario is in
//! rust/cli/tests/bug683_provider_state_failure_message.rs.

#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_cli::session::Session;
use codelet_core::persistence::{
    get_session_message_envelopes, input_receipts, load_session, reset_stores_for_tests,
};
use codelet_core::SessionManagerHandle;
use codelet_providers::codex::history::CodexHistory;
use codelet_providers::codex::session::CodexSession;
use codelet_rpc_types::{SessionId, SessionStatus, StreamChunk};
use codelet_sessions::background_session::{BackgroundSession, IncomingMessage, PromptInput};
use codelet_sessions::session_manager::SessionManagerHooks;
use codelet_sessions::SessionManager;
use codelet_tools::McpInjection;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::mpsc;

type TestResult = support::TestResult;

/// The agent loop's input channel, held so the test can see what was queued.
#[derive(Default)]
struct HeldLoop(Mutex<Option<(mpsc::Receiver<PromptInput>, mpsc::Receiver<McpInjection>)>>);

impl HeldLoop {
    fn take_queued(&self) -> Option<PromptInput> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
            .and_then(|(input, _)| input.try_recv().ok())
    }
}

impl SessionManagerHooks for HeldLoop {
    fn spawn_agent_loop(
        &self,
        _: Arc<BackgroundSession>,
        input: mpsc::Receiver<PromptInput>,
        mcp: mpsc::Receiver<McpInjection>,
    ) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some((input, mcp));
    }

    fn spawn_scheduler(&self, _: String, _: tokio::runtime::Handle) {}
    fn ensure_scheduler_running_for_loop(&self, _: String, _: tokio::runtime::Handle) {}
    fn spawn_footer_poller(&self, _: String, _: String, _: Option<String>) {}
    fn stop_footer_poller(&self, _: &str) {}
    fn cleanup_session_loops(&self, _: uuid::Uuid) {}
}

fn history(inner: &Session) -> Result<CodexHistory, Box<dyn std::error::Error>> {
    Ok(inner
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("the session has no Codex history")?
        .history
        .clone())
}

fn user(text: &str) -> Value {
    json!({"type":"message","role":"user","content":[{"type":"input_text","text":text}]})
}

fn answer(id: &str, text: &str) -> Value {
    json!({"type":"message","role":"assistant","id":id,"phase":"final_answer",
        "content":[{"type":"output_text","text":text,"annotations":[]}]})
}

fn replayed(text: &str) -> Value {
    json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":text,"annotations":[]}]})
}

fn request(input: Vec<Value>) -> Value {
    json!({"model":"test","input":input})
}

/// The saved Codex checkpoint of the latest generation, as replay delivers it.
fn saved_checkpoint(session: uuid::Uuid) -> Result<Value, Box<dyn std::error::Error>> {
    let mut latest = Value::Null;
    for envelope in get_session_message_envelopes(session)? {
        let envelope: Value = serde_json::from_str(&envelope)?;
        if envelope["codexContinuation"].is_object() {
            latest = envelope["codexContinuation"].clone();
        }
    }
    Ok(latest)
}

/// A Codex session that answered once at generation 0, moved to generation 1
/// and answered again there, as the user's session did around its compaction.
async fn saved_at_generation_one(
) -> Result<(support::TestDirectory, SessionId), Box<dyn std::error::Error>> {
    let (directory, manager, session_id, _) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let mut inner = background.inner.lock().await;
    inner.provider_session.ensure_provider("codex");
    inner.bind_provider_persistence(background.id)?;
    let codex = history(&inner)?;
    codex.commit(&request(vec![user("first")]), &[answer("a1", "one")])?;
    codex.transition_generation(1)?;
    codex.commit(&request(vec![user("second")]), &[answer("a2", "two")])?;
    drop(inner);
    let saved = saved_checkpoint(background.id)?;
    assert_eq!(saved["generation"], 1);
    assert_eq!(saved["items"].as_array().map(Vec::len), Some(2));
    Ok((directory, session_id))
}

/// A restarted daemon has created the session from its manifest, the first
/// step of `resume_session`, and has not restored it yet.
async fn resumed_before_restore(
    session_id: &SessionId,
) -> Result<(Arc<SessionManager>, Arc<HeldLoop>, Arc<BackgroundSession>), Box<dyn std::error::Error>>
{
    reset_stores_for_tests();
    let manager = SessionManager::new();
    let held = Arc::new(HeldLoop::default());
    manager.set_hooks(held.clone());
    let manifest = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
    manager
        .create_session_from_manifest(&manifest, &manifest.provider)
        .await?;
    let background = manager.get_session(&session_id.value)?;
    background
        .inner
        .lock()
        .await
        .provider_session
        .ensure_provider("codex");
    assert!(background.requires_model_restore());
    assert_eq!(history(&*background.inner.lock().await)?.generation()?, 0);
    Ok((manager, held, background))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn input_sent_before_a_resumed_session_finishes_restoring_is_refused_and_changes_nothing(
) -> TestResult {
    // @step Given a Codex session whose saved provider state is at generation 1
    let (_directory, session_id) = saved_at_generation_one().await?;
    // @step And the daemon restarts and resumes the session, and the restore has not finished
    let (manager, held, background) = resumed_before_restore(&session_id).await?;
    let before = serde_json::to_value(load_session(background.id)?)?;
    let mut chunks = manager.chunks_rx();
    // @step When the user sends input, a supervisor message arrives and a manual compaction is requested
    let typed = manager.send_input(&session_id, "fix the two tests".into());
    let supervised = background.receive_incoming_message(IncomingMessage::new(
        "supervisor".into(),
        "takeover supervisor".into(),
        "one more for BUG-634".into(),
    )?);
    let compaction = manager.compact_session(&session_id);
    let refusals = [typed.err(), supervised.err(), compaction.err()];
    // @step Then each is refused with a message saying the session is still restoring its saved history
    for refusal in &refusals {
        let error = refusal
            .as_deref()
            .ok_or("input was accepted while restoring")?;
        assert!(
            error.contains("is still restoring its saved history"),
            "unexpected refusal: {error}"
        );
    }
    // @step And the message says to send the input once the resume completes or to resume the session again
    for error in refusals.iter().flatten() {
        assert!(
            error.contains("once the resume completes")
                && error.contains("resume the session again"),
            "the refusal does not say what to do: {error}"
        );
    }
    // @step And nothing is echoed or queued and the session stays idle
    while let Ok((id, chunk)) = chunks.try_recv() {
        assert!(
            id != session_id
                || !matches!(
                    chunk,
                    StreamChunk::UserInput { .. } | StreamChunk::IncomingMessage { .. }
                ),
            "refused input was echoed: {chunk:?}"
        );
    }
    assert!(held.take_queued().is_none(), "refused input was queued");
    assert_eq!(background.pending_incoming_message_count(), 0);
    assert!(input_receipts::list(background.id)?.is_empty());
    assert_eq!(background.get_status(), SessionStatus::Idle);
    // @step And the session's messages and saved provider state are unchanged
    assert_eq!(serde_json::to_value(load_session(background.id)?)?, before);
    assert_eq!(saved_checkpoint(background.id)?["generation"], 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_resumed_codex_session_continues_at_its_saved_generation_once_its_restore_completes(
) -> TestResult {
    // @step Given a Codex session whose saved provider state is at generation 1
    let (_directory, session_id) = saved_at_generation_one().await?;
    let saved = saved_checkpoint(uuid::Uuid::parse_str(&session_id.value)?)?;
    // @step And the daemon restarts and resumes the session, and the restore has not finished
    let (manager, held, background) = resumed_before_restore(&session_id).await?;
    // @step When the resume completes
    manager.resume_session(&session_id)?;
    assert!(!background.requires_model_restore());
    let codex = history(&*background.inner.lock().await)?;
    // @step Then the session's Codex history is at generation 1 with the saved checkpoint
    assert_eq!(codex.generation()?, 1);
    assert_eq!(serde_json::to_value(codex.snapshot()?)?, saved);
    // @step And the next turn's checkpoint is saved at generation 1 without a rollback
    let mut next = request(vec![user("second"), replayed("two"), user("third")]);
    codex.prepare(&mut next)?;
    codex.commit(&next, &[answer("a3", "three")])?;
    let committed = saved_checkpoint(background.id)?;
    assert_eq!(committed["generation"], 1);
    assert_eq!(committed["items"].as_array().map(Vec::len), Some(4));
    // @step And input sent to the session is accepted
    manager.send_input(&session_id, "fix the two tests".into())?;
    let queued = held.take_queued().ok_or("accepted input was not queued")?;
    assert_eq!(queued.input, "fix the two tests");
    Ok(())
}
