//! BUG-355: helpers shared by the RPC-434 and RPC-437 session-event test
//! files (the support-module pattern: helpers here, every test in the one
//! root file per feature).

use codelet_test_helpers::OrFail;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;

use crate::common::MockBackend;

pub fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

/// A fresh App over a MockBackend, plus the mock for scripting pushes.
pub fn app_with_mock() -> (Arc<MockBackend>, App) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    (mock, App::new(backend))
}

/// `app_with_mock` after `App::bootstrap` ("connected to a daemon": the
/// bootstrap subscribers fold the mock's broadcasts onto the action bus).
pub async fn booted_app() -> Result<(Arc<MockBackend>, App), codelet_test_helpers::TestError> {
    let (mock, mut app) = app_with_mock();
    app.bootstrap().await.or_fail("bootstrap")?;
    Ok((mock, app))
}

/// Count the open tabs whose SessionContext id matches `id`.
pub fn tabs_for(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .open_sessions()
        .iter()
        .filter(|c| &c.id == id)
        .count()
}

/// Drain the App's action bus until a matching action arrives or 2s
/// elapses. Drains every queued action each tick so a backlog cannot
/// starve the predicate.
pub async fn wait_for_action<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    // TEST-042: wait for the action itself under the deadlock guard (None
    // only when the guard expires), never for a fixed window.
    codelet_test_helpers::deadlock_guard("the awaited action", async {
        loop {
            while let Some(action) = app.try_recv_action() {
                if pred(&action) {
                    return action;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .ok()
}

/// TEST-042: the actions on the bus once every spawned subscriber has run
/// to its first await. On the current-thread test runtime a yield lets each
/// ready task run, so a subscriber whose receiver is closed has ended and
/// can send nothing later. The yields are scheduler rounds, not a window.
pub async fn actions_after_subscribers_ran(app: &mut App) -> Vec<Action> {
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    std::iter::from_fn(|| app.try_recv_action()).collect()
}
