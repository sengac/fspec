//! RPC-451 — provider listing failures are shown.
//!
//! Feature: spec/features/provider-listing-failures-are-shown.feature

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_fspec_tui::Action;
use codelet_test_helpers::TestResult;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::harness::AppTestHarness;

#[test]
#[serial_test::serial]
fn an_invalid_provider_config_makes_the_provider_listing_fail() -> TestResult {
    // @step Given a providers directory containing an invalid "broken.json"
    let base = tempfile::tempdir()?;
    std::fs::create_dir_all(base.path().join("credentials"))?;
    std::fs::create_dir_all(base.path().join("providers"))?;
    std::fs::write(base.path().join("providers/broken.json"), "{ not json")?;
    std::env::set_var("FSPEC_HOME", base.path().join("credentials"));
    // @step When the session manager lists providers
    let manager = codelet_sessions::SessionManager::new();
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let listed = handle.list_providers();
    std::env::remove_var("FSPEC_HOME");
    // @step Then the listing fails naming the invalid provider config
    let error = listed
        .err()
        .ok_or("an invalid provider config was listed")?;
    assert!(error.contains("broken.json"), "{error}");
    Ok(())
}

fn screen(harness: &mut AppTestHarness) -> String {
    let area = Rect::new(0, 0, 200, 50);
    let mut buffer = Buffer::empty(area);
    harness.app.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_provider_listing_is_shown_when_the_model_selector_opens() -> TestResult {
    // @step Given a backend whose provider listing fails with "providers/broken.json is not valid JSON"
    let mut harness = AppTestHarness::empty();
    harness.drain_pending().await?;
    harness
        .mock
        .fail_list_providers("providers/broken.json is not valid JSON");
    // @step When the model selector opens
    harness.app.dispatch(Action::OpenModelSelectorView);
    harness.drain_pending().await?;
    // @step Then an error dialog shows "providers/broken.json is not valid JSON"
    let screen = screen(&mut harness);
    assert!(
        screen.contains("providers/broken.json is not valid JSON"),
        "{screen}"
    );
    Ok(())
}
