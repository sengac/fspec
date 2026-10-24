use std::sync::Arc;

use codelet_cli::session::Session;
use codelet_core::persistence::{
    create_session, get_message, load_session, reset_stores_for_tests,
};
use codelet_providers::{ProviderManager, ProviderType};
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use codelet_test_helpers::setup_temp_directory;
use serde_json::Value;
use tokio::sync::{broadcast, mpsc};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub type Fixture = (
    tempfile::TempDir,
    Arc<BackgroundSession>,
    mpsc::Receiver<PromptInput>,
);

pub fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let manifest = create_session("recovery", directory.path())?;
    let (input_tx, input_rx) = mpsc::channel(4);
    let mut inner = Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::Codex,
        Some(272_000),
        Some(4096),
    ));
    inner
        .provider_manager_mut()
        .set_facade_override(Some("bug246-no-network".into()));
    let session = Arc::new(BackgroundSession::new(
        manifest.id,
        "recovery".into(),
        directory.path().to_string_lossy().into(),
        Some("codex".into()),
        Some("shared-model".into()),
        inner,
        input_tx,
        None,
        None,
        None,
        broadcast::channel(64).0,
        broadcast::channel(64).0,
        broadcast::channel(32).0,
    ));
    std::fs::create_dir_all(directory.path().join("messages/messages.jsonl"))?;
    Ok((directory, session, input_rx))
}

pub fn read(session: uuid::Uuid) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    reset_stores_for_tests();
    load_session(session)?
        .messages
        .into_iter()
        .map(|reference| {
            let stored = get_message(reference.message_id)?
                .ok_or("manifest references an unavailable original")?;
            Ok(serde_json::to_value(stored.metadata)?)
        })
        .collect()
}
