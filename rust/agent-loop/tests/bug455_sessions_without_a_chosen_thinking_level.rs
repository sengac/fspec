//! BUG-455 — sessions without a chosen thinking level use the model default.
//!
//! Feature: spec/features/sessions-without-a-chosen-thinking-level.feature

#[allow(dead_code)]
#[path = "prov145_profile_config_applied_support/mod.rs"]
mod support;

use codelet_agent_loop::agent_loop::thinking::resolve_turn_thinking;
use codelet_agent_loop::thinking_config::{ThinkingTarget, ThinkingWire};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionId, ThinkingLevel};

type TestResult = Result<(), String>;

/// An OpenAI gpt-5.5 turn (MODEL-010 wire table).
fn gpt_5_5() -> ThinkingTarget {
    ThinkingTarget::new(
        Ok(ThinkingWire::OpenAiChat),
        "openai/gpt-5.5",
        "gpt-5.5",
        advertised(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_new_session_without_a_saved_default_has_the_model_default_level() -> TestResult {
    let _guard = support::DATA_DIR_GUARD.lock().await;
    // @step Given no default thinking level is saved
    // @step When a session is created
    let (_data_dir, manager, session) = support::fresh_background_session().await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    // @step Then its thinking level is ModelDefault
    assert_eq!(
        handle
            .get_thinking_level(&SessionId::new(session.id.to_string()))
            .expect("get_thinking_level reads the session"),
        ThinkingLevel::ModelDefault
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_model_default_level_sends_no_thinking_parameter_whatever_the_prompt_says() -> TestResult
{
    let _guard = support::DATA_DIR_GUARD.lock().await;
    // @step Given a session whose thinking level is ModelDefault
    let (_data_dir, _manager, session) = support::fresh_background_session().await;
    // @step When turns are prompted with "hello", "ultrathink about this" and "nothink, just answer"
    // The prompt is not an input to a turn's thinking level, so every one of
    // these turns resolves the same way.
    let mut sent = Vec::new();
    for _prompt in ["hello", "ultrathink about this", "nothink, just answer"] {
        sent.push(resolve_turn_thinking(&session, None, &gpt_5_5())?);
    }
    // @step Then none of the turns sends a thinking parameter
    assert_eq!(sent, [None, None, None]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn choosing_model_default_clears_an_explicit_level() -> TestResult {
    let _guard = support::DATA_DIR_GUARD.lock().await;
    // @step Given a session whose thinking level is High
    let (_data_dir, manager, session) = support::fresh_background_session().await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let id = SessionId::new(session.id.to_string());
    handle.set_thinking_level(&id, ThinkingLevel::Effort("high".to_string()))?;
    assert_eq!(
        handle
            .get_thinking_level(&id)
            .expect("get_thinking_level reads the session"),
        ThinkingLevel::Effort("high".to_string())
    );
    // @step When ModelDefault is chosen
    handle.set_thinking_level(&id, ThinkingLevel::ModelDefault)?;
    // @step Then its thinking level is ModelDefault again
    assert_eq!(
        handle
            .get_thinking_level(&id)
            .expect("get_thinking_level reads the session"),
        ThinkingLevel::ModelDefault
    );
    Ok(())
}

/// MODEL-017: the levels openai/gpt-5.5 advertises (none, low, medium, high,
/// xhigh); a model that advertises none has no reasoning control.
fn advertised() -> Result<Option<codelet_providers::models::ReasoningCapability>, String> {
    let options: Vec<codelet_providers::models::ReasoningOption> = serde_json::from_value(
        serde_json::json!([{"type": "effort", "values": ["none", "low", "medium", "high", "xhigh"]}]),
    )
    .map_err(|error| error.to_string())?;
    codelet_providers::models::ReasoningCapability::resolve("openai/gpt-5.5", Some(&options))
        .map_err(|error| error.to_string())
}
