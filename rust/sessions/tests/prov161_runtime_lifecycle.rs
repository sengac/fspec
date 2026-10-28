#![doc = "Feature: spec/features/provider-runtime-settings-lifecycle.feature"]

use codelet_core::persistence::load_session;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::{model_resolution, SessionManager};
use fixtures::{Fixture, TestResult};

#[path = "fixtures/prov161_runtime.rs"]
pub mod fixtures;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_session_lifecycle_applies_current_provider_runtime_settings() -> TestResult {
    // @step Given saved provider settings for continuation images loop detection and thinking preservation
    let fixture = Fixture::new()?;

    // @step When sessions are created resumed and spawned as subordinates
    let original = fixture.create().await?;
    let manifest = load_session(uuid::Uuid::parse_str(&original)?)?;
    fixture.save(9)?;
    let resumed_manager = SessionManager::new();
    resumed_manager
        .create_session_from_manifest(&manifest, "openai/gpt-4o")
        .await?;
    let subordinate = fixture
        .create_with_model("anthropic:careful/claude-sonnet-5")
        .await?;
    SessionManagerHandle::add_supervisor(
        fixture.manager.as_ref(),
        &SessionId::new(subordinate.clone()),
        &SessionId::new(original.clone()),
    )?;

    // @step Then each session applies current saved runtime settings and effective token limits
    for (manager, identifier, budget, images, context, output, preserve, window) in [
        (
            fixture.manager.as_ref(),
            original.as_str(),
            7,
            2,
            180000,
            12000,
            false,
            80,
        ),
        (
            &resumed_manager,
            original.as_str(),
            9,
            2,
            180000,
            12000,
            false,
            80,
        ),
        (
            fixture.manager.as_ref(),
            subordinate.as_str(),
            5,
            8,
            300000,
            18000,
            true,
            240,
        ),
    ] {
        let background = manager.get_session(identifier)?;
        assert_eq!(background.get_continue_state(), (true, budget));
        let inner = background.inner.lock().await;
        assert_eq!(inner.preserve_thinking_enabled, preserve);
        assert_eq!(inner.provider_manager().context_window(), context);
        assert_eq!(inner.provider_manager().max_output_tokens(), output);
        assert_eq!(
            model_resolution::resolve_profile_max_images(inner.provider_manager()),
            Some(images)
        );
        let loops = model_resolution::resolve_profile_loop_detection(inner.provider_manager());
        assert_eq!(loops.enabled, Some(preserve));
        assert_eq!(loops.window, Some(window));
        assert_eq!(
            codelet_tools::model_capabilities::session_model_max_images(background.id),
            Some(images)
        );
        let model = SessionManagerHandle::get_session_model(manager, &SessionId::new(identifier));
        assert_eq!(model.context_window as usize, context);
        assert_eq!(model.max_output_tokens as usize, output);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_model_switching_replaces_inherited_settings_without_losing_explicit_session_choices(
) -> TestResult {
    // @step Given an active session with inherited provider settings and an explicit continuation choice
    let fixture = Fixture::new()?;
    let inherited = SessionId::new(fixture.create().await?);
    let explicit = SessionId::new(fixture.create().await?);
    let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
    handle.set_continue_state(&explicit, false, 23)?;

    // @step When the session switches to another provider or profile
    for identifier in [&inherited, &explicit] {
        handle.set_model(identifier, "anthropic", "claude-sonnet-5")?;
    }

    // @step Then inherited settings and budgets refresh while the explicit continuation choice is preserved
    assert_eq!(handle.get_continue_state(&inherited), (true, 3));
    assert_eq!(handle.get_continue_state(&explicit), (false, 23));
    for identifier in [&inherited, &explicit] {
        let background = fixture.manager.get_session(&identifier.value)?;
        let inner = background.inner.lock().await;
        assert!(inner.preserve_thinking_enabled);
        assert_eq!(inner.provider_manager().context_window(), 300000);
        assert_eq!(inner.provider_manager().max_output_tokens(), 16000);
        let loops = model_resolution::resolve_profile_loop_detection(inner.provider_manager());
        assert_eq!(loops.enabled, Some(true));
        assert_eq!(loops.window, Some(240));
        assert_eq!(
            codelet_tools::model_capabilities::session_model_max_images(background.id),
            Some(6)
        );
        let model = handle.get_session_model(identifier);
        assert_eq!(model.context_window, 300000);
        assert_eq!(model.max_output_tokens, 16000);
    }
    let previous_key = std::env::var_os("OPENAI_API_KEY");
    let previous_endpoint = std::env::var_os("OPENAI_BASE_URL");
    for identifier in [&inherited, &explicit] {
        handle.set_model(identifier, "anthropic:careful", "claude-sonnet-5")?;
        let model = handle.get_session_model(identifier);
        assert_eq!(model.max_output_tokens, 18000);
        let background = fixture.manager.get_session(&identifier.value)?;
        assert_eq!(
            codelet_tools::model_capabilities::session_model_max_images(background.id),
            Some(8)
        );
    }
    assert_eq!(handle.get_continue_state(&inherited), (true, 5));
    assert_eq!(handle.get_continue_state(&explicit), (false, 23));
    assert_eq!(std::env::var_os("OPENAI_API_KEY"), previous_key);
    assert_eq!(std::env::var_os("OPENAI_BASE_URL"), previous_endpoint);
    Ok(())
}
