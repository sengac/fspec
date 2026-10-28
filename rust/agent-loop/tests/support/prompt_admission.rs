use codelet_cli::session::Session;
use codelet_core::persistence::{create_session, reset_stores_for_tests};
use codelet_providers::{ProviderManager, ProviderType};
#[cfg(feature = "test-support")]
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::{BackgroundSession, PromptInput};
#[cfg(feature = "test-support")]
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
#[cfg(feature = "test-support")]
pub type Fixture = (
    tempfile::TempDir,
    Arc<BackgroundSession>,
    mpsc::Receiver<PromptInput>,
    Arc<AtomicUsize>,
);

pub type SessionFixture = (
    tempfile::TempDir,
    Arc<BackgroundSession>,
    mpsc::Receiver<PromptInput>,
);

pub fn session_fixture() -> Result<SessionFixture, Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let (session, receiver) = new_session(directory.path())?;
    Ok((directory, session, receiver))
}

pub fn new_session(
    directory: &std::path::Path,
) -> Result<(Arc<BackgroundSession>, mpsc::Receiver<PromptInput>), Box<dyn std::error::Error>> {
    let manifest = create_session("admission", directory)?;
    let (sender, receiver) = mpsc::channel(8);
    let inner = Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::Custom("stub".into()),
        Some(272_000),
        Some(4096),
    ));
    let session = Arc::new(BackgroundSession::new(
        manifest.id,
        "admission".into(),
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

#[cfg(feature = "test-support")]
pub fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (directory, session, receiver) = session_fixture()?;
    codelet_providers::stub_provider::register_stub_provider();
    let polls = Arc::new(AtomicUsize::new(0));
    codelet_providers::stub_model::set_looping_stream_hook(
        vec!["local response".into()],
        polls.clone(),
    );
    Ok((directory, session, receiver, polls))
}

#[cfg(feature = "test-support")]
pub struct StubGuard;

#[cfg(feature = "test-support")]
impl Drop for StubGuard {
    fn drop(&mut self) {
        codelet_providers::stub_model::clear_looping_stream_hook();
    }
}

#[cfg(feature = "test-support")]
pub async fn done(events: &mut broadcast::Receiver<StreamChunk>, count: usize) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let mut completed = 0;
        while completed < count {
            if matches!(events.recv().await?, StreamChunk::Done) {
                completed += 1;
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await?
}

pub fn originals(id: uuid::Uuid) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    reset_stores_for_tests();
    codelet_core::persistence::load_session(id)?
        .messages
        .into_iter()
        .map(|reference| {
            let stored = codelet_core::persistence::get_message(reference.message_id)?
                .ok_or("missing original")?;
            Ok(serde_json::to_value(stored.metadata)?)
        })
        .collect()
}

#[cfg(feature = "test-support")]
pub async fn wait_for(path: &std::path::Path) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while !path.exists() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await?;
    Ok(())
}

#[cfg(feature = "test-support")]
pub fn block_hook(session: &mut Arc<BackgroundSession>, root: &std::path::Path) -> TestResult {
    use codelet_core::lifecycle_hooks::{CompiledHookDefinition, CompiledLifecycleHooks};
    let command = format!(
        "touch '{}/entered'; while [ ! -e '{}/release' ]; do sleep 0.01; done",
        root.display(),
        root.display()
    );
    Arc::get_mut(session)
        .ok_or("session already shared")?
        .lifecycle_hooks = Some(Arc::new(CompiledLifecycleHooks {
        global_timeout: 10,
        global_shell: None,
        session_start: Vec::new(),
        session_end: Vec::new(),
        user_prompt_submit: vec![CompiledHookDefinition {
            name: "deterministic barrier".into(),
            command,
            blocking: true,
            timeout: 10,
        }],
        notification: Vec::new(),
        pre_tool_use: Vec::new(),
        post_tool_use: Vec::new(),
    }));
    Ok(())
}
