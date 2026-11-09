//! BUG-355: action-bus helpers for tests/bug181_live_checkpoint_counts.rs
//! (the support-module pattern: helpers here, every test in the one root
//! file).

use codelet_fspec_tui::App;
use codelet_rpc_types::CheckpointCounts;

/// Dispatch every action currently on the bus (best-effort, no wait).
pub fn drain_bus(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Wait for the BoardStore's checkpoint counts to reach `want`,
/// dispatching whatever actions arrive on the bus along the way.
pub async fn wait_for_board_counts(
    app: &mut App,
    want: CheckpointCounts,
) -> Result<(), codelet_test_helpers::TestError> {
    // TEST-042: wait for the counts under the deadlock guard, not a deadline.
    codelet_test_helpers::deadlock_guard("the BoardStore checkpoint counts", async {
        loop {
            drain_bus(app);
            if app.board_store().checkpoint_counts() == want {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
