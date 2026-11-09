//! BUG-468 — mock-backed TUI sessions open without error notices.
//!
//! Feature: spec/features/mock-backed-sessions-open-cleanly.feature

mod common;
use codelet_test_helpers::TestResult;
use common::harness::{seed_sid, AppTestHarness};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_session_on_the_mock_backend_opens_without_an_error_notice() -> TestResult {
    // @step Given an App on the mock backend with a focused agent session
    let mut harness = AppTestHarness::new();
    // @step When the pending startup work is drained
    harness.drain_pending().await?;
    // @step Then the session's scrollback holds no error notice
    let text = harness.scrollback_text(&seed_sid());
    assert!(!text.contains("[error]"), "{text}");
    Ok(())
}
