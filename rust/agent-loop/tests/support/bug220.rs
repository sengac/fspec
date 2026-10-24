use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use std::sync::Arc;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub async fn fixture() -> Result<
    (
        tempfile::TempDir,
        Arc<BackgroundSession>,
        Arc<BackgroundOutput>,
    ),
    Box<dyn std::error::Error>,
> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    codelet_providers::stub_provider::register_stub_provider();
    let manager = SessionManager::new();
    let id = manager
        .create_session("stub/canned", &directory.path().to_string_lossy())
        .await?;
    let session = manager.get_session(&id)?;
    let output = Arc::new(BackgroundOutput::with_provider(
        session.clone(),
        "stub".into(),
        None,
    ));
    Ok((directory, session, output))
}

pub fn detector_abort(output: &BackgroundOutput) {
    for _ in 0..100 {
        output.emit(StreamEvent::Text("repeated ".into()));
    }
}

pub fn interruptions(session: &BackgroundSession) -> Vec<serde_json::Value> {
    session
        .get_buffered_output(10000)
        .iter()
        .map(codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value)
        .filter(|value| value["type"] == "interrupted")
        .collect()
}
