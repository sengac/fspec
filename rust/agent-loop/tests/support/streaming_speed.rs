use codelet_agent_loop::{background_output::LoopDetectionWiring, BackgroundOutput};

#[path = "bug246.rs"]
mod recovery;

pub use recovery::TestResult;
type BackgroundFixture = (
    tempfile::TempDir,
    std::sync::Arc<codelet_sessions::background_session::BackgroundSession>,
    BackgroundOutput,
);

pub fn background() -> Result<BackgroundFixture, Box<dyn std::error::Error>> {
    let (directory, session, _) = recovery::fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    if !recovery::read(session.id)?.is_empty() {
        return Err("streaming fixture must start without conversation history".into());
    }
    let output = BackgroundOutput::with_provider(
        session.clone(),
        "fixture-no-network".into(),
        Some(LoopDetectionWiring {
            enabled: false,
            ..Default::default()
        }),
    );
    Ok((directory, session, output))
}
