//! Feature: spec/features/streaming-loop-fresh-escalation.feature

use std::sync::{atomic::Ordering, Arc};

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_sessions::{background_session::BackgroundSession, session_manager::SessionManager};

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn background(
) -> Result<(tempfile::TempDir, Arc<BackgroundSession>, BackgroundOutput), Box<dyn std::error::Error>>
{
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    codelet_providers::stub_provider::register_stub_provider();
    let manager = SessionManager::new();
    let id = manager
        .create_session(
            "stub/canned",
            directory
                .path()
                .to_str()
                .ok_or("project path is not UTF-8")?,
        )
        .await?;
    let session = manager.get_session(&id)?;
    let output = BackgroundOutput::with_provider(session.clone(), "stub".into(), None);
    Ok((directory, session, output))
}

fn text_words(output: &BackgroundOutput, count: usize) {
    for _ in 0..count {
        output.emit(StreamEvent::Text("yes ".into()));
    }
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_a_warning_cannot_escalate_from_latched_evidence_alone() -> TestResult {
    // @step Given a background output whose text channel has just warned on a genuine loop
    let (_directory, session, output) = background().await?;
    text_words(&output, 30);
    assert!(!session.is_interrupted.load(Ordering::Acquire));
    assert!(!session.has_pending_loop_abort_note());

    // @step When empty deltas and fresh technical prose arrive
    output.emit(StreamEvent::Text(String::new()));
    output.emit(StreamEvent::Text(
        "rust/fspec-tui/src/app/dispatch_esc_cascade.rs: ".into(),
    ));
    for index in 0..200 {
        output.emit(StreamEvent::Text(format!("fresh{index} ")));
    }

    // @step Then the session is not interrupted and no corrective note is staged
    assert!(!session.is_interrupted.load(Ordering::Acquire));
    assert!(!session.has_pending_loop_abort_note());

    // @step When a fresh complete evidence window repeats in the text channel
    text_words(&output, 200);

    // @step Then the session is interrupted and a corrective note is staged
    assert!(session.is_interrupted.load(Ordering::Acquire));
    assert!(session.has_pending_loop_abort_note());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_channel_evidence_is_independent_and_new_turns_clear_partial_words() -> TestResult
{
    // @step Given a background output whose text channel has just warned on a genuine loop
    let (_directory, session, output) = background().await?;
    text_words(&output, 30);
    assert!(!session.is_interrupted.load(Ordering::Acquire));

    // @step When the thinking channel receives a genuine complete word loop
    for _ in 0..30 {
        output.emit(StreamEvent::Thinking("yes ".into()));
    }

    // @step Then the session is interrupted and a corrective note is staged
    assert!(session.is_interrupted.load(Ordering::Acquire));
    assert!(session.take_pending_loop_abort_note().is_some());

    // @step When a new turn resets both channels and receives fresh prose
    session.reset_interrupt();
    output.reset_turn_loop_detectors();
    output.emit(StreamEvent::Text("partial".into()));
    output.reset_turn_loop_detectors();
    for index in 0..100 {
        output.emit(StreamEvent::Text(format!("independent{index} ")));
        output.emit(StreamEvent::Thinking(format!("analysis{index} ")));
    }

    // @step Then no new interruption or corrective note occurs
    assert!(!session.is_interrupted.load(Ordering::Acquire));
    assert!(!session.has_pending_loop_abort_note());
    Ok(())
}
