//! COV-057 shared offline `BackgroundSession` fixture.
//!
//! Mirrors `rust/agent-loop/tests/support/prompt_admission.rs`: a real
//! `BackgroundSession` backed by the deterministic `stub` provider manager
//! and a persisted manifest in a hermetic temp data directory. No network,
//! no credentials.

use codelet_cli::session::Session;
use codelet_core::persistence::{create_session, reset_stores_for_tests};
use codelet_providers::{ProviderManager, ProviderType};
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Point the process-global data directory at a fresh temp dir and reset
/// the persistence singletons so manifests land there.
pub fn data_directory() -> TestResult<tempfile::TempDir> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    Ok(directory)
}

/// Build a real stub-provider `BackgroundSession` with its own persisted
/// manifest; returns the session and the receiving end of its input queue.
pub fn stub_session(
    directory: &Path,
    name: &str,
) -> TestResult<(Arc<BackgroundSession>, mpsc::Receiver<PromptInput>)> {
    let manifest = create_session(name, directory)?;
    let (sender, receiver) = mpsc::channel(8);
    let inner = Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::Custom("stub".into()),
        Some(272_000),
        Some(4096),
    ));
    let session = Arc::new(BackgroundSession::new(
        manifest.id,
        name.into(),
        directory.to_string_lossy().into(),
        Some("stub".into()),
        Some("canned".into()),
        inner,
        sender,
        None,
        None,
        None,
        broadcast::channel(64).0,
        broadcast::channel(64).0,
        broadcast::channel(32).0,
    ));
    Ok((session, receiver))
}
