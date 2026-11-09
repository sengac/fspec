//! MODEL-011: a session stores the thinking level exactly as chosen and its
//! turn sends that level unchanged.
//!
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature
//!
//! A real `SessionManager` creates a session on anthropic/claude-haiku-4-5
//! from a hermetic models.dev cache that advertises only a token budget with
//! a minimum of 1024 (its models.dev `reasoning_options`). Levels are chosen
//! through the `SessionManagerHandle`, as the TUI does, and the turn's
//! thinking parameters come from the agent loop's own resolver.

use std::sync::Arc;

use codelet_agent_loop::agent_loop::thinking::resolve_turn_thinking;
use codelet_agent_loop::thinking_config::ThinkingTarget;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionId, ThinkingLevel};
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::SessionManager;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use serde_json::json;

/// A models.dev cache holding only claude-haiku-4-5, budget-only.
const MODELS: &str = r#"{
  "anthropic": {
    "id": "anthropic",
    "name": "Anthropic",
    "env": ["ANTHROPIC_API_KEY"],
    "models": {
      "claude-haiku-4-5": {
        "id": "claude-haiku-4-5",
        "name": "Claude Haiku 4.5",
        "reasoning": true,
        "reasoning_options": [{"type": "budget_tokens", "min": 1024}],
        "tool_call": true,
        "attachment": true,
        "temperature": true,
        "limit": { "context": 200000, "output": 64000 }
      }
    }
  }
}
"#;

/// A session on anthropic/claude-haiku-4-5 created through the handle.
fn haiku_session() -> Result<(Arc<SessionManager>, Arc<BackgroundSession>, SessionId), TestError> {
    let data_dir = codelet_test_helpers::shared_temp_subdirectory()?;
    let cache = data_dir.join("cache");
    std::fs::create_dir_all(&cache)?;
    std::fs::write(cache.join("models.json"), MODELS)?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data_dir.clone())?;
    std::env::set_var("ANTHROPIC_API_KEY", "model011-fake-key");
    let manager = SessionManager::new();
    manager.set_default_project(data_dir)?;
    manager.set_default_model("anthropic/claude-haiku-4-5")?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let id = handle.create_session_checked(None)?;
    let session = manager.get_session(&id.value)?;
    Ok((manager, session, id))
}

/// What the session's next turn sends.
async fn turn_sends(session: &BackgroundSession) -> Result<Option<serde_json::Value>, TestError> {
    let target = {
        let inner = session.inner.lock().await;
        ThinkingTarget::for_provider_manager(inner.provider_manager())
    };
    Ok(resolve_turn_thinking(session, None, &target)?)
}

/// Scenario: The session stores the chosen level exactly and the turn sends it unchanged
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_session_stores_the_chosen_level_exactly_and_the_turn_sends_it_unchanged() -> TestResult
{
    // @step Given a session whose model is anthropic/claude-haiku-4-5
    let (manager, session, id) = haiku_session()?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    // @step When the user chooses a token budget of 8000
    handle.set_thinking_level(&id, ThinkingLevel::Budget(8000))?;
    // @step Then get_thinking_level returns a token budget of 8000
    assert_eq!(handle.get_thinking_level(&id)?, ThinkingLevel::Budget(8000));
    // @step And the turn sends thinking type enabled with budget_tokens 8000
    assert_eq!(
        turn_sends(&session).await?,
        Some(json!({"thinking": {"type": "enabled", "budget_tokens": 8000}}))
    );
    // @step When the user chooses a budget of 512
    let refusal = handle
        .set_thinking_level(&id, ThinkingLevel::Budget(512))
        .err()
        .or_fail("a budget under the minimum is refused")?;
    // @step Then the choice is refused naming the minimum 1024 and get_thinking_level still returns a token budget of 8000
    assert!(
        refusal.contains("anthropic/claude-haiku-4-5") && refusal.contains("1024"),
        "{refusal}"
    );
    assert_eq!(handle.get_thinking_level(&id)?, ThinkingLevel::Budget(8000));
    // @step When the user chooses Model default
    handle.set_thinking_level(&id, ThinkingLevel::ModelDefault)?;
    // @step Then get_thinking_level returns Model default and the turn sends no thinking parameter
    assert_eq!(handle.get_thinking_level(&id)?, ThinkingLevel::ModelDefault);
    assert_eq!(turn_sends(&session).await?, None);
    Ok(())
}
