use std::sync::Arc;

use codelet_cli::session::Session;
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use tokio::sync::{broadcast, mpsc};

pub fn overflow_error() -> anyhow::Error {
    codelet_providers::request_admission::RequestAdmissionError::BudgetExceeded {
        input_tokens: 201_000,
        output_reservation: 4096,
        context_capacity: 200_000,
    }
    .into()
}

pub fn fixture() -> (
    tempfile::TempDir,
    Arc<BackgroundSession>,
    mpsc::Receiver<PromptInput>,
) {
    let directory = codelet_test_helpers::setup_temp_directory().expect("create data directory");
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(directory.path().to_path_buf()).expect("redirect data");
    std::env::set_var("ANTHROPIC_API_KEY", "test-no-network");
    let mut inner = Session::new(Some("claude")).expect("create offline session");
    inner
        .provider_manager_mut()
        .set_model_direct(
            "anthropic",
            "claude-sonnet-4-6",
            Some(200_000),
            Some(4096),
            None,
        )
        .expect("explicit terminal selected model");
    inner
        .provider_manager_mut()
        .set_facade_override(Some("cmpct053-missing".into()));
    let (input_tx, input_rx) = mpsc::channel(4);
    let (chunks_tx, _) = broadcast::channel(64);
    let (status_tx, _) = broadcast::channel(64);
    let (cleared_tx, _) = broadcast::channel(4);
    let mut manifest = codelet_core::persistence::create_session_with_provider(
        "terminal-lock",
        directory.path(),
        "claude",
    )
    .expect("durable terminal source");
    codelet_core::persistence::append_message(&mut manifest, "user", "committed evidence")
        .expect("persist immutable original");
    codelet_core::persistence::append_message(&mut manifest, "assistant", "committed response")
        .expect("persist immutable response");
    let session = Arc::new(BackgroundSession::new(
        manifest.id,
        "terminal-lock".into(),
        directory.path().to_string_lossy().into(),
        None,
        None,
        inner,
        input_tx,
        None,
        None,
        None,
        chunks_tx,
        status_tx,
        cleared_tx,
    ));
    (directory, session, input_rx)
}
