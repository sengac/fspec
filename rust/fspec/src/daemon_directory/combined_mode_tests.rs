//! Feature: spec/features/combined-mode-daemon-switcher.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// BUG-430 — Feature: spec/features/combined-mode-daemon-switcher.feature

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_switcher_opens_with_the_in_process_daemon_connected() -> TestResult {
    // @step Given an app with the daemon switcher installed for its own local daemon
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("spec"))?;
    std::fs::write(
        dir.path().join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    let service = Arc::new(SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(
        dir.path(),
    )?)));
    let backend =
        codelet_fspec_tui::EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service);
    let mut app = codelet_fspec_tui::App::new(Arc::new(backend));
    let url = url::Url::parse("ws://127.0.0.1:41234")?;
    super::install_daemon_switcher(&mut app, &url, None)?;
    // @step When the daemon switcher is opened
    app.dispatch(codelet_fspec_tui::Action::OpenDaemonSwitcher);
    // @step Then it opens and names the in-process daemon as connected
    assert!(
        app.board_store().daemon_switcher().is_some(),
        "the switcher must open"
    );
    assert_eq!(
        app.board_store().connected_daemon(),
        Some(super::name_for(&url, None)?.as_str())
    );
    Ok(())
}
