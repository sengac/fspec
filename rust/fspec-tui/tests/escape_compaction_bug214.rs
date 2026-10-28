#[path = "bug210_progress/support.rs"]
pub mod progress_support;
#[path = "bug214_escape/support.rs"]
pub mod support;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use progress_support::TestResult;
use support::EscapeFixture;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_escape_cancels_pending_pre_prompt_compaction_through_the_embedded_transport(
) -> TestResult {
    // @step Given the TUI uses the embedded session transport
    // @step And the focused session has a pending pre-prompt automatic compaction builder
    let mut fixture = EscapeFixture::new(false).await?;
    // @step And no dialog, popup or selection consumes Escape
    fixture.assert_no_transient_ui();
    // @step When I press Escape through the TUI event handler
    fixture
        .shared
        .app
        .handle_event(&Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    fixture.drain_keypress().await?;
    // @step Then the focused session receives the interrupt and its pending compaction stops promptly
    fixture.assert_stopped().await?;
    // @step And no successful compaction or continuation inference occurs
    fixture.assert_no_success_or_continuation();
    // @step And the original conversation remains unchanged and accessible
    fixture.assert_conversation_preserved().await?;
    // @step And compaction activity is cleared for that session without affecting another session
    fixture.assert_activity_cleared()?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_escape_cancels_pending_pre_prompt_compaction_through_the_websocket_transport(
) -> TestResult {
    // @step Given the TUI uses a local WebSocket session transport
    // @step And the focused session has a pending pre-prompt automatic compaction builder
    let mut fixture = EscapeFixture::new(true).await?;
    // @step And no dialog, popup or selection consumes Escape
    fixture.assert_no_transient_ui();
    // @step When I press Escape through the TUI event handler
    fixture
        .shared
        .app
        .handle_event(&Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    fixture.drain_keypress().await?;
    // @step Then the focused session receives the interrupt and its pending compaction stops promptly
    fixture.assert_stopped().await?;
    // @step And no successful compaction or continuation inference occurs
    fixture.assert_no_success_or_continuation();
    // @step And the original conversation remains unchanged and accessible
    fixture.assert_conversation_preserved().await?;
    // @step And compaction activity is cleared for that session without affecting another session
    fixture.assert_activity_cleared()?;
    Ok(())
}
