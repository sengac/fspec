#![doc = "Feature: spec/features/provider-runtime-settings-reload.feature"]

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::codex::session::CodexSession;
use codelet_rpc_types::{ProfileDefinition, SessionId};
use fixtures::{Fixture, TestResult};
use rig::message::Message;

#[path = "fixtures/prov161_runtime.rs"]
pub mod fixtures;

fn changed_settings() -> ProfileDefinition {
    ProfileDefinition {
        context_window: Some(800_000),
        max_output_tokens: Some(32_000),
        compaction_threshold_type: Some("tokens".into()),
        compaction_threshold_value: Some(600_000),
        auto_continue: Some(11),
        preserve_thinking: Some(true),
        max_images: Some(9),
        loop_detection_enabled: Some(true),
        loop_detection_window: Some(250),
        ..ProfileDefinition::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_next_turn_reloads_settings_without_replacing_conversation() -> TestResult {
    let fixture = Fixture::new()?;
    let identifier = SessionId::new(fixture.create().await?);
    let background = fixture.manager.get_session(&identifier.value)?;
    let mut inner = background.inner.lock().await;
    inner.messages.push(Message::user("keep this history"));
    inner.provider_session.ensure_provider("codex");
    let extension = inner
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("missing provider extension")? as *const CodexSession;
    let model = inner.provider_manager().selected_model_string();
    let history = serde_json::to_value(&inner.messages)?;
    let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
    handle.save_provider_settings("openai", &changed_settings())?;
    assert_eq!(inner.provider_manager().context_window(), 180_000);

    background.refresh_runtime_settings_for_turn(&mut inner)?;

    assert_eq!(inner.provider_manager().context_window(), 800_000);
    assert_eq!(inner.provider_manager().max_output_tokens(), 32_000);
    assert_eq!(inner.provider_manager().selected_model_string(), model);
    assert_eq!(serde_json::to_value(&inner.messages)?, history);
    assert!(std::ptr::eq(
        inner
            .provider_session
            .extension::<CodexSession>("codex")
            .ok_or("provider extension was replaced")?,
        extension
    ));
    assert!(inner.preserve_thinking_enabled);
    assert_eq!(background.get_continue_state(), (true, 11));
    assert_eq!((inner.continue_enabled, inner.continue_budget), (true, 11));
    assert_eq!(
        codelet_tools::model_capabilities::session_model_max_images(background.id),
        Some(9)
    );
    let loops = codelet_sessions::model_resolution::resolve_profile_loop_detection(
        inner.provider_manager(),
    );
    assert_eq!(loops.enabled, Some(true));
    assert_eq!(loops.window, Some(250));
    let cached = handle.get_session_model(&identifier);
    assert_eq!(cached.context_window, 800_000);
    assert_eq!(cached.max_output_tokens, 32_000);
    assert_eq!(cached.compaction_threshold, 600_000);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_existing_codex_session_reloads_saved_context_window() -> TestResult {
    let fixture = Fixture::new()?;
    std::fs::write(
        fixture.directory.path().join("cache/models.json"),
        include_str!("fixtures/prov129_codex_models.json").replace("gpt-5.4", "gpt-6.1-sol"),
    )?;
    let identifier = SessionId::new(fixture.create_with_model("codex/gpt-6.1-sol").await?);
    let background = fixture.manager.get_session(&identifier.value)?;
    let mut inner = background.inner.lock().await;
    let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
    assert_eq!(inner.provider_manager().context_window(), 400_000);
    handle.save_provider_settings("codex", &changed_settings())?;

    background.refresh_runtime_settings_for_turn(&mut inner)?;

    assert_eq!(
        inner.provider_manager().selected_model_string().as_deref(),
        Some("codex/gpt-6.1-sol")
    );
    assert_eq!(inner.provider_manager().context_window(), 800_000);
    assert_eq!(inner.provider_manager().max_output_tokens(), 32_000);
    let cached = handle.get_session_model(&identifier);
    assert_eq!(cached.context_window, 800_000);
    assert_eq!(cached.compaction_threshold, 600_000);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_reloading_preserves_explicit_session_overrides() -> TestResult {
    let fixture = Fixture::new()?;
    let identifier = SessionId::new(fixture.create().await?);
    let background = fixture.manager.get_session(&identifier.value)?;
    background.set_continue_state(false, 23);
    let mut inner = background.inner.lock().await;
    inner
        .provider_manager_mut()
        .override_model_limits(Some(150_000), Some(10_000));
    inner
        .provider_manager_mut()
        .set_compaction_threshold_override(Some(("tokens".into(), 100_000)));
    let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
    handle.save_provider_settings("openai", &changed_settings())?;

    background.refresh_runtime_settings_for_turn(&mut inner)?;

    assert_eq!(inner.provider_manager().context_window(), 150_000);
    assert_eq!(inner.provider_manager().max_output_tokens(), 10_000);
    assert_eq!(
        inner.provider_manager().compaction_threshold_override(),
        Some(("tokens", 100_000))
    );
    assert_eq!(background.get_continue_state(), (false, 23));
    assert_eq!((inner.continue_enabled, inner.continue_budget), (false, 23));
    assert!(inner.preserve_thinking_enabled);
    let cached = handle.get_session_model(&identifier);
    assert_eq!(cached.context_window, 150_000);
    assert_eq!(cached.max_output_tokens, 10_000);
    assert_eq!(cached.compaction_threshold, 100_000);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_invalid_settings_leave_current_session_unchanged() -> TestResult {
    let fixture = Fixture::new()?;
    let identifier = SessionId::new(fixture.create().await?);
    let background = fixture.manager.get_session(&identifier.value)?;
    let mut inner = background.inner.lock().await;
    let options = inner.provider_manager().runtime_options().clone();
    let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
    let cached = handle.get_session_model(&identifier);
    std::fs::write(
        fixture.directory.path().join("fspec-config.json"),
        br#"{"providers":{"openai":{"settings":{"contextWindow":0,"autoContinue":11}}}}"#,
    )?;

    let error = background
        .refresh_runtime_settings_for_turn(&mut inner)
        .err()
        .ok_or("invalid runtime settings must fail dispatch preparation")?;

    assert!(error.contains("Token overrides must be positive"));
    assert_eq!(inner.provider_manager().runtime_options(), &options);
    assert_eq!(background.get_continue_state(), (true, 7));
    assert_eq!(
        handle.get_session_model(&identifier).context_window,
        cached.context_window
    );
    assert_eq!(
        handle.get_session_model(&identifier).compaction_threshold,
        cached.compaction_threshold
    );
    Ok(())
}
