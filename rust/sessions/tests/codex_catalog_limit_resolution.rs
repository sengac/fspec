//! Feature: spec/features/codex-catalog-limit-resolution.feature

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::ProviderType;
use codelet_rpc_types::SessionId;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use std::sync::atomic::Ordering;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn create_session(
) -> Result<(tempfile::TempDir, SessionManager, Arc<BackgroundSession>), Box<dyn std::error::Error>>
{
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    std::fs::create_dir_all(directory.path().join("cache"))?;
    std::fs::write(
        directory.path().join("cache/models.json"),
        include_str!("fixtures/prov129_codex_models.json").replace("gpt-5.4", "gpt-6.1-sol"),
    )?;
    let manager = SessionManager::new();
    let id = manager
        .create_session(
            "codex/gpt-6.1-sol",
            directory.path().to_str().ok_or("path")?,
        )
        .await?;
    let session = manager.get_session(&id)?;
    Ok((directory, manager, session))
}

async fn assert_limits(
    session: &BackgroundSession,
    model: &str,
    context: u32,
    output: u32,
) -> TestResult {
    let inner = session.inner.lock().await;
    let provider = inner.provider_manager();
    assert_eq!(provider.current_provider_type(), &ProviderType::Codex);
    assert_eq!(provider.selected_model_string().as_deref(), Some(model));
    assert_eq!(provider.context_window(), context as usize);
    assert_eq!(provider.max_output_tokens(), output as usize);
    assert_eq!(
        session.cached_context_window.load(Ordering::Acquire),
        context
    );
    assert_eq!(
        session.cached_max_output_tokens.load(Ordering::Acquire),
        output
    );
    let threshold = codelet_cli::compaction_threshold::resolve_compaction_threshold(
        context as u64,
        output as u64,
        provider.selected_model_id().as_deref(),
        None,
    ) as u32;
    assert_eq!(
        session.cached_compaction_threshold.load(Ordering::Acquire),
        threshold
    );
    if context > 0 {
        assert!(threshold > 0);
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn creating_a_codex_session_resolves_limits_from_its_source_catalog() -> TestResult {
    let (_directory, _manager, session) = create_session().await?;
    assert_limits(&session, "codex/gpt-6.1-sol", 400_000, 128_000).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn switching_codex_models_refreshes_limits_from_the_source_catalog() -> TestResult {
    let (_directory, manager, session) = create_session().await?;
    let handle: &dyn SessionManagerHandle = &manager;
    handle.set_model(
        &SessionId {
            value: session.id.to_string(),
        },
        "codex",
        "gpt-4o",
    )?;
    assert_limits(&session, "codex/gpt-4o", 128_000, 16_384).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn an_uncatalogued_codex_selection_does_not_inherit_guessed_limits() -> TestResult {
    let (_directory, manager, session) = create_session().await?;
    let handle: &dyn SessionManagerHandle = &manager;
    handle.set_model(
        &SessionId {
            value: session.id.to_string(),
        },
        "codex",
        "unknown-model",
    )?;
    assert_limits(&session, "codex/unknown-model", 0, 0).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_partial_codex_model_name_does_not_authorize_catalog_capacity() -> TestResult {
    let (_directory, manager, session) = create_session().await?;
    let handle: &dyn SessionManagerHandle = &manager;
    handle.set_model(
        &SessionId {
            value: session.id.to_string(),
        },
        "codex",
        "gpt-4",
    )?;
    assert_limits(&session, "codex/gpt-4", 0, 0).await
}
