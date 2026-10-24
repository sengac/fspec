//! Feature: spec/features/provider-reset-failure-atomicity.feature

#[path = "cmpct060/support.rs"]
mod support;

use codelet_core::SessionManagerHandle;
use codelet_providers::session::{ExtensionError, ProviderSessionExtension, StateWriter};
use std::{any::Any, sync::Arc};

struct RejectedReset;

impl ProviderSessionExtension for RejectedReset {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn restore(&self, _: &[String]) -> Result<(), ExtensionError> {
        Ok(())
    }

    fn set_writer(&self, _: StateWriter) -> Result<(), ExtensionError> {
        Ok(())
    }

    fn begin_turn(&self) {}

    fn reset(&self) -> Result<(), ExtensionError> {
        Err(ExtensionError::Persistence(
            "checkpoint reset rejected".into(),
        ))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_failed_provider_reset_cannot_clear_preserved_session_history(
) -> support::TestResult {
    // @step Given a session whose provider rejects resetting its replay checkpoint
    let (_directory, manager, session_id, _) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let before = {
        let mut inner = background.inner.lock().await;
        inner
            .messages
            .push(rig::message::Message::user("preserved original"));
        inner
            .provider_session
            .register("reject", Arc::new(RejectedReset));
        serde_json::to_value(&inner.messages)?
    };
    background.handle_output(codelet_rpc_types::StreamChunk::text(
        "preserved display".into(),
    ));
    let display_before = serde_json::to_value(background.get_buffered_output(100))?;
    // @step When the session history is cleared
    let result = manager.clear_history(&session_id);
    // @step Then the failure is returned without deleting history or announcing a cleared session
    assert!(
        result.is_err(),
        "provider reset failure must reach the caller"
    );
    assert_eq!(
        serde_json::to_value(&background.inner.lock().await.messages)?,
        before
    );
    assert_eq!(
        serde_json::to_value(background.get_buffered_output(100))?,
        display_before
    );
    let result = manager.clear_session_history(&session_id.value);
    assert!(
        result.is_err(),
        "inherent clear must propagate reset failure too"
    );
    assert_eq!(
        serde_json::to_value(&background.inner.lock().await.messages)?,
        before
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_failed_provider_reset_cannot_report_a_successful_model_switch(
) -> support::TestResult {
    // @step Given a session whose provider rejects resetting its replay checkpoint
    let (_directory, manager, session_id, _) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let previous = {
        let mut inner = background.inner.lock().await;
        inner
            .provider_session
            .register("reject", Arc::new(RejectedReset));
        inner.current_model_id()
    };
    let previous_labels = serde_json::to_value(manager.get_session_model(&session_id))?;
    // @step When a different model is selected
    let result = manager.set_model(&session_id, "anthropic", "claude-opus-4-8");
    // @step Then the reset failure is returned without changing the selected model
    assert!(
        result.is_err(),
        "model switch must not hide provider reset failure"
    );
    assert_eq!(background.inner.lock().await.current_model_id(), previous);
    assert_eq!(
        serde_json::to_value(manager.get_session_model(&session_id))?,
        previous_labels
    );
    Ok(())
}
