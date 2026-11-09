//! Feature: spec/features/codex-catalog-limit-resolution.feature

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_providers::ProviderType;
use codelet_rpc_types::SessionId;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_catalog() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    std::fs::create_dir_all(directory.path().join("cache"))?;
    std::fs::write(
        directory.path().join("cache/models.json"),
        include_str!("fixtures/prov129_codex_models.json").replace("gpt-5.4", "gpt-6.1-sol"),
    )?;
    Ok(directory)
}

async fn create_codex_session(
    directory: &tempfile::TempDir,
) -> Result<(Arc<SessionManager>, Arc<BackgroundSession>), Box<dyn std::error::Error>> {
    let manager = SessionManager::new();
    let id = manager
        .create_session(
            "codex/gpt-6.1-sol",
            directory.path().to_str().ok_or("path")?,
        )
        .await?;
    let session = manager.get_session(&id)?;
    Ok((manager, session))
}

async fn create_session() -> Result<
    (
        tempfile::TempDir,
        Arc<SessionManager>,
        Arc<BackgroundSession>,
    ),
    Box<dyn std::error::Error>,
> {
    let directory = write_catalog()?;
    let (manager, session) = create_codex_session(&directory).await?;
    assert_limits(&session, "codex/gpt-6.1-sol", Some(400_000), Some(128_000)).await?;
    Ok((directory, manager, session))
}

fn switch_model(manager: &SessionManager, session: &BackgroundSession, model: &str) -> TestResult {
    let handle: &dyn SessionManagerHandle = manager;
    handle.set_model(
        &SessionId {
            value: session.id.to_string(),
        },
        "codex",
        model,
    )?;
    Ok(())
}

async fn assert_limits(
    session: &BackgroundSession,
    model: &str,
    context: Option<u32>,
    output: Option<u32>,
) -> TestResult {
    let inner = session.inner.lock().await;
    let provider = inner.provider_manager();
    assert_eq!(provider.current_provider_type(), &ProviderType::Codex);
    assert_eq!(provider.selected_model_string().as_deref(), Some(model));
    assert_eq!(
        provider.context_window(),
        context.map(|value| value as usize)
    );
    assert_eq!(
        provider.max_output_tokens(),
        output.map(|value| value as usize)
    );
    let cached = session.model_limits();
    assert_eq!(cached.context_window, context);
    assert_eq!(cached.max_output_tokens, output);
    let threshold = codelet_cli::compaction_threshold::resolve_compaction_threshold(
        context.map(u64::from),
        output.map(u64::from),
        provider.selected_model_id().as_deref(),
        None,
    )
    .map(|threshold| threshold as u32);
    assert_eq!(cached.compaction_threshold, threshold);
    if context.is_some() {
        assert!(threshold.is_some_and(|threshold| threshold > 0));
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn creating_a_codex_session_resolves_limits_from_its_source_catalog() -> TestResult {
    // @step Given the OpenAI catalog contains a model offered through Codex
    let directory = write_catalog()?;

    // @step When a session is created with that Codex model
    let (_manager, session) = create_codex_session(&directory).await?;

    // @step Then active and cached limits match the model catalog
    // @step And its compaction threshold is positive
    // @step And its selected transport remains Codex
    assert_limits(&session, "codex/gpt-6.1-sol", Some(400_000), Some(128_000)).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn switching_codex_models_refreshes_limits_from_the_source_catalog() -> TestResult {
    // @step Given a Codex session with resolved model limits
    let (_directory, manager, session) = create_session().await?;

    // @step When the session switches to another catalog-backed Codex model
    switch_model(&manager, &session, "gpt-4o")?;

    // @step Then active and cached limits match the new model catalog
    // @step And its compaction threshold follows the new capacity
    // @step And its selected transport remains Codex
    assert_limits(&session, "codex/gpt-4o", Some(128_000), Some(16_384)).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn an_uncatalogued_codex_selection_does_not_inherit_guessed_limits() -> TestResult {
    // @step Given a Codex session with resolved model limits
    let (_directory, manager, session) = create_session().await?;

    // @step When the session switches to a model absent from the catalog
    let switched = switch_model(&manager, &session, "unknown-model");

    // @step Then the switch is refused naming the missing context window and output limit
    assert_refused(switched, "unknown-model")?;

    // @step And the session keeps its catalogued model and limits
    assert_limits(&session, "codex/gpt-6.1-sol", Some(400_000), Some(128_000)).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_partial_codex_model_name_does_not_authorize_catalog_capacity() -> TestResult {
    // @step Given a Codex session with resolved model limits
    let (_directory, manager, session) = create_session().await?;

    // @step When the session switches to a name that only prefixes a catalog model
    let switched = switch_model(&manager, &session, "gpt-4");

    // @step Then the switch is refused naming the missing context window and output limit
    assert_refused(switched, "gpt-4")?;

    // @step And the session keeps its catalogued model and limits
    assert_limits(&session, "codex/gpt-6.1-sol", Some(400_000), Some(128_000)).await
}

/// PROV-176: a Codex model models.dev has no limits for is refused, naming
/// the missing limits.
fn assert_refused(switched: TestResult, model: &str) -> TestResult {
    let error = match switched {
        Ok(()) => return Err(format!("switching to codex/{model} succeeded").into()),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&format!("codex/{model}")), "{error}");
    assert!(error.contains("context window"), "{error}");
    assert!(error.contains("output limit"), "{error}");
    Ok(())
}
