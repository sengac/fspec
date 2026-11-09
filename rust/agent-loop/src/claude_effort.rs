//! MODEL-015: the agent loop plans each admitted turn's Claude effort.
//!
//! A level changed between turns on a Claude model that takes per-message
//! effort becomes an effort message before the user's new message
//! (`codelet_providers::claude_effort`). The conversation's effort log is
//! persisted on the session manifest whenever it changes, so a restored
//! session sends its effort messages at their positions.
//!
//! Feature: spec/features/claude-effort-can-change-between-turns.feature

use crate::thinking_config::ThinkingTarget;
use codelet_cli::session::Session;
use codelet_providers::claude_effort::EffortLog;
use serde_json::Value;
use uuid::Uuid;

/// Plans the admitted turn whose user message is `message` and persists the
/// effort log when it changed. A log that cannot be persisted refuses the
/// turn: an effort message must never be sent without being recorded.
pub async fn plan_turn_effort(
    session_id: Uuid,
    inner: &mut Session,
    target: &ThinkingTarget,
    config: Option<&Value>,
    message: Uuid,
    fresh_input: bool,
) -> Result<(), String> {
    let changed = inner.effort_log.plan_turn(
        target.takes_per_message_effort(),
        config,
        message,
        fresh_input,
    );
    if !changed {
        return Ok(());
    }
    let record = inner.effort_log.clone();
    tokio::task::spawn_blocking(move || persist_effort_log(session_id, record))
        .await
        .map_err(|error| error.to_string())?
}

/// Records the effort log on the session manifest.
pub fn persist_effort_log(session_id: Uuid, record: EffortLog) -> Result<(), String> {
    codelet_core::persistence::update_session_manifest(session_id, |manifest| {
        manifest.effort_log = Some(record);
        Ok(())
    })
    .map(drop)
    .map_err(|error| format!("the Claude effort log could not be saved: {error}"))
}
