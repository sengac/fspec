#[path = "cmpct060/support.rs"]
mod support;

use std::sync::{atomic::Ordering, Arc, Mutex, PoisonError};
use std::time::Duration;

use codelet_core::persistence::{append_message, load_session};
use codelet_core::SessionManagerHandle;
use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use codelet_sessions::session_manager::SessionManagerHooks;
use codelet_tools::McpInjection;
use rig::message::Message;
use tokio::sync::mpsc;

struct Channels {
    input: mpsc::Receiver<PromptInput>,
    mcp: mpsc::Receiver<McpInjection>,
}

#[derive(Default)]
struct DeferredLoop(Mutex<Option<Channels>>);

impl SessionManagerHooks for DeferredLoop {
    fn spawn_agent_loop(
        &self,
        _: Arc<BackgroundSession>,
        input: mpsc::Receiver<PromptInput>,
        mcp: mpsc::Receiver<McpInjection>,
    ) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(Channels { input, mcp });
    }

    fn spawn_scheduler(&self, _: String, _: tokio::runtime::Handle) {}
    fn ensure_scheduler_running_for_loop(&self, _: String, _: tokio::runtime::Handle) {}
    fn spawn_footer_poller(&self, _: String, _: String, _: Option<String>) {}
    fn stop_footer_poller(&self, _: &str) {}
    fn cleanup_session_loops(&self, _: uuid::Uuid) {}
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cancellation_while_manual_compaction_waits_for_preparation_prevents_its_continuation(
) -> support::TestResult {
    // @step Given a populated session whose manual compaction is waiting to prepare its working view
    let (_directory, manager, session_id, _) = support::manager().await?;
    let manager = Arc::new(manager);
    let background = manager.get_session(&session_id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "preserved unfinished task")?;
    append_message(&mut manifest, "assistant", "preserved work in progress")?;
    let durable = serde_json::to_value(&manifest)?;
    let mut inner = background.inner.lock().await;
    inner.messages.extend([
        Message::user("preserved unfinished task"),
        Message::assistant("preserved work in progress"),
    ]);
    inner.token_tracker.input_tokens = 12_345;
    let original_messages = serde_json::to_value(&inner.messages)?;
    let (entered, ready) = tokio::sync::oneshot::channel();
    let compact_manager = manager.clone();
    let compact_id = session_id.clone();
    let operation = tokio::spawn(async move {
        let _ = entered.send(());
        compact_manager.compact_session(&compact_id)
    });
    tokio::time::timeout(Duration::from_secs(3), ready).await??;
    // @step When the session is interrupted before preparation can proceed
    background.interrupt();
    drop(inner);
    let outcome = tokio::time::timeout(Duration::from_secs(3), operation).await??;
    // @step Then manual compaction returns a cancelled outcome without clearing the interrupt
    assert!(
        outcome
            .as_ref()
            .is_err_and(|error| error.contains("cancel")),
        "manual compaction must return cancellation: {outcome:?}"
    );
    assert!(background.is_interrupted.load(Ordering::Acquire));
    // @step And no internal continuation input or compaction success is published
    assert!(!background
        .get_buffered_output(1000)
        .iter()
        .any(|chunk| matches!(chunk,
            StreamChunk::UserInput { text, .. } if text == "Continue"
        ) || matches!(chunk, StreamChunk::CompactionComplete { .. })));
    // @step And the original conversation and durable session remain unchanged
    assert_eq!(serde_json::to_value(load_session(background.id)?)?, durable);
    let inner = background.inner.lock().await;
    assert_eq!(serde_json::to_value(&inner.messages)?, original_messages);
    assert_eq!(inner.token_tracker.input_tokens, 12_345);
    assert!(!inner.has_compaction_working_view());
    // @step And no compaction activity remains owned by the cancelled operation
    assert!(!background.compaction_in_progress.load(Ordering::Acquire));
    assert!(background.get_compaction_progress().is_none());
    assert_ne!(background.get_status(), SessionStatus::Compacting);
    assert!(inner.ensure_compaction_inference_ready().is_ok());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cancellation_after_manual_continuation_is_queued_prevents_agent_execution(
) -> support::TestResult {
    // @step Given manual compaction has queued its continuation but the agent has not consumed it
    let (directory, manager, _, _) = support::manager().await?;
    let deferred = Arc::new(DeferredLoop::default());
    manager.set_hooks(deferred.clone());
    let session_id = codelet_rpc_types::SessionId::new(
        manager
            .create_session(
                "anthropic/claude-sonnet-4",
                &directory.path().to_string_lossy(),
            )
            .await?,
    );
    let background = manager.get_session(&session_id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "preserved unfinished task")?;
    append_message(&mut manifest, "assistant", "preserved work in progress")?;
    let durable = serde_json::to_value(&manifest)?;
    {
        let mut inner = background.inner.lock().await;
        inner.messages.extend([
            Message::user("preserved unfinished task"),
            Message::assistant("preserved work in progress"),
        ]);
        inner
            .provider_manager_mut()
            .set_facade_override(Some("bug214-no-network".into()));
    }
    manager.compact_session(&session_id)?;
    let mut channels = deferred
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
        .ok_or("deferred agent channels")?;
    assert_eq!(
        channels.input.len(),
        1,
        "manual continuation must really be queued"
    );
    channels.input.close();
    channels.mcp.close();
    // @step When the session is interrupted before the queued continuation is consumed
    background.interrupt();
    tokio::time::timeout(
        Duration::from_secs(5),
        codelet_agent_loop::agent_loop::agent_loop(
            background.clone(),
            channels.input,
            channels.mcp,
        ),
    )
    .await?;
    // @step Then the agent discards the cancelled continuation without clearing the interrupt
    assert!(
        background.is_interrupted.load(Ordering::Acquire),
        "agent turn entry erased cancellation of queued manual continuation"
    );
    // @step And no continuation message is persisted or inference attempted
    assert!(!background
        .get_buffered_output(1000)
        .iter()
        .any(|chunk| matches!(
            chunk,
            StreamChunk::Error { .. } | StreamChunk::CompactionComplete { .. }
        )));
    // @step And the durable conversation remains unchanged and compaction activity is cleared
    assert_eq!(serde_json::to_value(load_session(background.id)?)?, durable);
    assert!(!background.compaction_in_progress.load(Ordering::Acquire));
    assert!(background.get_compaction_progress().is_none());
    assert_ne!(background.get_status(), SessionStatus::Compacting);
    let inner = background.inner.lock().await;
    assert!(!inner.has_compaction_working_view());
    assert!(inner.ensure_compaction_inference_ready().is_ok());
    Ok(())
}
