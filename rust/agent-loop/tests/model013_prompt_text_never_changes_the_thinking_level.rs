//! Feature: spec/features/prompt-text-never-changes-the-thinking-level.feature

#[allow(dead_code)]
#[path = "prov145_profile_config_applied_support/mod.rs"]
mod support;

use codelet_agent_loop::agent_loop::thinking::resolve_turn_thinking;
use codelet_agent_loop::thinking_config::{ThinkingTarget, ThinkingWire};
use codelet_agent_loop::thinking_level_detection::base_thinking_level;
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

/// The thinking parameters an OpenAI turn sends at the level stored as `stored`.
fn openai_config(stored: ThinkingLevel) -> Result<Option<serde_json::Value>, String> {
    let level = base_thinking_level(&stored)?.ok_or("no level is stored")?;
    gpt_5_5().config_for(Some(&level))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_words_never_lower_a_chosen_level() -> TestResult {
    let _guard = support::DATA_DIR_GUARD.lock().await;
    // @step Given a session whose thinking level is High
    let (_data_dir, manager, session) = support::fresh_background_session().await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle.set_thinking_level(
        &SessionId::new(session.id.to_string()),
        ThinkingLevel::Effort("high".to_string()),
    )?;
    // @step When a turn is prompted with "quickly list files"
    let sent = resolve_turn_thinking(&session, None, &gpt_5_5())?;
    // @step Then the turn thinks at High
    assert_eq!(
        sent,
        openai_config(ThinkingLevel::Effort("high".to_string()))?
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_words_never_raise_a_chosen_level() -> TestResult {
    let _guard = support::DATA_DIR_GUARD.lock().await;
    // @step Given a session whose thinking level is Low
    let (_data_dir, manager, session) = support::fresh_background_session().await;
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    handle.set_thinking_level(
        &SessionId::new(session.id.to_string()),
        ThinkingLevel::Effort("low".to_string()),
    )?;
    // @step When a turn is prompted with "ultrathink about the architecture"
    let sent = resolve_turn_thinking(&session, None, &gpt_5_5())?;
    // @step Then the turn thinks at Low
    assert_eq!(
        sent,
        openai_config(ThinkingLevel::Effort("low".to_string()))?
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
