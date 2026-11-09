//! BUG-453 — requests a service or backend cannot serve are refused.
//!
//! Feature: spec/features/unserved-requests-are-refused.feature

use codelet_test_helpers::TestResult;
use std::sync::Arc;

use async_trait::async_trait;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::daemons::{DaemonChoice, DaemonDirectory};
use codelet_fspec_tui::{Action, FspecBackend};
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_rpc_types::ScheduledJob;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
#[path = "common/model_backend.rs"]
#[allow(dead_code)]
mod model_backend;
use common::harness::AppTestHarness;
use model_backend::ModelBackend;

#[tokio::test]
async fn a_session_rpc_without_a_session_manager_is_refused() -> TestResult {
    // @step Given an fspec service with no session manager attached
    let start = tempfile::tempdir()?;
    let service = FspecServiceImpl::new(Arc::new(SharedFspecService::new(Arc::new(
        WorkUnitsWatcher::new(start.path())?,
    ))));
    // @step When a scheduled job is added
    let added = service
        .schedule_add(tarpc::context::current(), ScheduledJob::default())
        .await;
    // @step Then the request fails saying no session manager is attached
    let error = added
        .err()
        .ok_or("a job was added without a session manager")?;
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

#[tokio::test]
async fn a_backend_method_a_backend_does_not_implement_is_refused() -> TestResult {
    // @step Given a backend that does not implement isolated session creation
    let backend = ModelBackend::new();
    // @step When an isolated session is created through it
    let created = backend.create_isolated_session(None).await;
    // @step Then the call fails saying create_isolated_session is not implemented by this backend
    let error = created
        .err()
        .ok_or("an isolated session was faked")?
        .to_string();
    assert!(
        error.contains("create_isolated_session is not implemented by this backend"),
        "{error}"
    );
    Ok(())
}

/// A directory whose saved-daemons file is unreadable.
struct UnreadableDirectory;

#[async_trait]
impl DaemonDirectory for UnreadableDirectory {
    fn list(&self) -> anyhow::Result<Vec<DaemonChoice>> {
        Err(anyhow::anyhow!("~/.fspec/daemons.json is not valid JSON"))
    }
    async fn connect(&self, _choice: &DaemonChoice) -> anyhow::Result<Arc<dyn FspecBackend>> {
        Err(anyhow::anyhow!("no daemon can be listed"))
    }
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
async fn an_unreadable_daemon_list_is_shown_as_an_error() -> TestResult {
    // @step Given a daemon directory that cannot read its saved daemons
    let mut harness = AppTestHarness::empty();
    harness.drain_pending().await?;
    harness
        .app
        .set_daemon_directory(Arc::new(UnreadableDirectory), "alpha".into());
    // @step When the daemon switcher is opened
    harness.app.dispatch(Action::OpenDaemonSwitcher);
    harness.drain_pending().await?;
    // @step Then the daemon switcher shows the daemon directory's error
    let screen = screen(&mut harness);
    assert!(
        screen.contains("daemons.json is not valid JSON"),
        "{screen}"
    );
    Ok(())
}
