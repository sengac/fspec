use super::fixture::{self, TestResult};
use codelet_core::persistence::*;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use std::path::Path;

fn checkpoint(inner: &codelet_cli::session::Session, row: &str) -> TestResult<serde_json::Value> {
    if row != "codex" {
        return Ok(serde_json::Value::Null);
    }
    let extension = inner
        .provider_session
        .extension::<codelet_providers::codex::session::CodexSession>("codex")
        .ok_or("Codex extension after repeated restore")?;
    Ok(serde_json::to_value(extension.history.snapshot()?)?)
}

pub async fn repeated(
    manager: &SessionManager,
    id: &SessionId,
    background: &BackgroundSession,
    root: &Path,
    row: &str,
) -> TestResult {
    let before = load_session(background.id)?;
    let bytes = fixture::manifest_bytes(root, background.id)?;
    let originals = serde_json::to_value(get_session_messages_full(&before)?)?;
    let envelopes = get_session_message_envelopes(background.id)?;
    manager
        .restore_session_messages(id, envelopes.clone())
        .map_err(|error| {
            format!(
                "{error}; installed replay/runtime count: {:?}; incoming envelopes: {}",
                background
                    .inner
                    .try_lock()
                    .map(|inner| (inner.restored_replay.clone(), inner.messages.len())),
                envelopes.len()
            )
        })?;
    let (messages, tokens, first_checkpoint) = {
        let inner = background.inner.lock().await;
        (
            inner.messages.clone(),
            serde_json::to_value(&inner.token_tracker)?,
            checkpoint(&inner, row)?,
        )
    };
    assert_eq!(
        tokens["cumulative_billed_input"],
        before.token_usage.cumulative_billed_input
    );
    assert_eq!(
        tokens["cumulative_billed_output"],
        before.token_usage.cumulative_billed_output
    );
    assert_eq!(
        tokens["reasoning_tokens"],
        before.token_usage.reasoning_tokens
    );
    manager.restore_session_messages(id, envelopes)?;
    {
        let inner = background.inner.lock().await;
        assert_eq!(
            inner.messages, messages,
            "repeated restore duplicated or reordered suffix"
        );
        assert_eq!(
            serde_json::to_value(&inner.token_tracker)?,
            tokens,
            "repeated restore changed usage or billed twice"
        );
        assert_eq!(
            checkpoint(&inner, row)?,
            first_checkpoint,
            "repeated restore changed same-generation continuation"
        );
        inner.ensure_compaction_inference_ready()?;
    }
    assert_eq!(
        serde_json::to_value(load_session(background.id)?)?,
        serde_json::to_value(&before)?,
        "repeated restore changed durable state or billed usage"
    );
    assert_eq!(fixture::manifest_bytes(root, background.id)?, bytes);
    assert_eq!(
        serde_json::to_value(get_session_messages_full(&load_session(background.id)?)?)?,
        originals
    );
    Ok(())
}
