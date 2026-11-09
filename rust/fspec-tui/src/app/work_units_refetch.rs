//! BUG-361 / BUG-379: work-unit and checkpoint-count refetches outside the
//! normal broadcast (after a reconnect, a lagged work-unit broadcast and a
//! daemon switch). A failed fetch is logged and sent as
//! `Action::SessionOperationFailed`, which the session storage error dialog
//! shows, so the user knows the board may be stale.

use tokio::sync::mpsc::UnboundedSender;

use crate::components::Action;
use crate::transport::FspecBackend;

/// Re-list the work units and send them, or the failure. `when` completes
/// "Work units could not be reloaded <when>: <error>".
pub(crate) async fn refetch_work_units(
    backend: &dyn FspecBackend,
    action_tx: &UnboundedSender<Action>,
    when: &str,
) {
    let action = match backend.list_work_units().await {
        Ok(units) => Action::WorkUnitsLoaded(units),
        Err(err) => {
            tracing::error!(error = %err, "list_work_units {when} failed");
            Action::SessionOperationFailed(format!(
                "Work units could not be reloaded {when}: {err}"
            ))
        }
    };
    send_or_log(action_tx, action);
}

/// Re-read the checkpoint counts and send them, or the failure. `when`
/// completes "Checkpoint counts could not be reloaded <when>: <error>".
pub(crate) async fn refetch_checkpoint_counts(
    backend: &dyn FspecBackend,
    action_tx: &UnboundedSender<Action>,
    when: &str,
) {
    let action = match backend.checkpoint_counts().await {
        Ok(counts) => Action::CheckpointCountsLoaded(counts),
        Err(err) => {
            tracing::error!(error = %err, "checkpoint_counts {when} failed");
            Action::SessionOperationFailed(format!(
                "Checkpoint counts could not be reloaded {when}: {err}"
            ))
        }
    };
    send_or_log(action_tx, action);
}

/// Send `action` to the app loop; a closed loop means the app is shutting
/// down, so the sending task logs it and ends.
pub(crate) fn send_or_log(action_tx: &UnboundedSender<Action>, action: Action) {
    if let Err(err) = action_tx.send(action) {
        tracing::debug!(error = %err, "app loop closed; background task ends without delivering");
    }
}
