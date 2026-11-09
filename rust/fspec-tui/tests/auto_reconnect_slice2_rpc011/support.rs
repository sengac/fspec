//! Shared helpers for `auto_reconnect_slice2_rpc011.rs` (REFAC-025 split).

use codelet_test_helpers::OrFail;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use tokio::sync::mpsc::UnboundedReceiver;

use super::common::MockBackend;

/// Drain the App's action bus, dispatching each Action.
pub fn pump_actions(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// RPC-416: source texts of every scrollback chunk in session `id` whose text
/// mentions a reconnect state ("Reconnecting" / "Reconnected"). Used by the
/// inline-reconnect assertions that replaced the DisconnectDialog modal ones.
pub fn reconnect_lines(app: &App, id: &SessionId) -> Vec<String> {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.source.as_ref().map(|s| s.text.clone()))
        .filter(|t| t.contains("Reconnect"))
        .collect()
}

/// Exactly one inline reconnect line remains in focused session "s-1" (an
/// in-place update) and it contains `needle`.
pub fn assert_single_reconnect_line(app: &App, needle: &str) {
    let lines = reconnect_lines(app, &SessionId::new("s-1"));
    assert_eq!(
        lines.len(),
        1,
        "the reconnect line must update in place — exactly one remains, got: {lines:?}"
    );
    assert!(
        lines[0].contains(needle),
        "the inline reconnect line must contain {needle:?}, got: {:?}",
        lines[0]
    );
}

/// Wait for an action matching `wanted` (TEST-042: under the deadlock guard;
/// `false` when the bus closes or the guard expires).
pub async fn wait_for_action(
    rx: &mut UnboundedReceiver<Action>,
    wanted: impl Fn(&Action) -> bool,
) -> bool {
    codelet_test_helpers::deadlock_guard("the awaited action", async {
        while let Some(action) = rx.recv().await {
            if wanted(&action) {
                return true;
            }
        }
        false
    })
    .await
    .unwrap_or(false)
}

/// Drain until both `Disconnected` and one `Reconnecting(_)` were seen
/// (TEST-042: under the deadlock guard).
pub async fn saw_disconnect_then_reconnecting(rx: &mut UnboundedReceiver<Action>) -> (bool, bool) {
    let (mut disconnected, mut reconnecting) = (false, false);
    let _ = codelet_test_helpers::deadlock_guard("Disconnected then Reconnecting", async {
        while !(disconnected && reconnecting) {
            match rx.recv().await {
                Some(Action::Disconnected) => disconnected = true,
                Some(Action::Reconnecting(_)) => reconnecting = true,
                Some(_) => continue,
                None => break,
            }
        }
    })
    .await;
    (disconnected, reconnecting)
}

/// Capture the tokio-clock timestamps of the first seven `Reconnecting(n)`
/// frames (TEST-042: under the deadlock guard; on a paused test clock the
/// gaps are the supervisor's exact backoff).
pub async fn observe_reconnect_attempts(
    rx: &mut UnboundedReceiver<Action>,
) -> Vec<(u32, tokio::time::Instant)> {
    let mut observed = Vec::new();
    let _ = codelet_test_helpers::deadlock_guard("seven reconnect attempts", async {
        while observed.len() < 7 {
            match rx.recv().await {
                Some(Action::Reconnecting(n)) => observed.push((n, tokio::time::Instant::now())),
                Some(_) => continue,
                None => break,
            }
        }
    })
    .await;
    observed
}

/// Assert the gaps between attempts follow 250/500/1000/2000/5000/5000 ms
/// (±150 ms below, +1 s above for CI load).
pub fn assert_backoff_gaps(observed: &[(u32, tokio::time::Instant)]) {
    let expected_ms: [u64; 6] = [250, 500, 1000, 2000, 5000, 5000];
    for (i, expected) in expected_ms.iter().enumerate() {
        let dt_ms = observed[i + 1]
            .1
            .saturating_duration_since(observed[i].1)
            .as_millis() as u64;
        let lower = expected.saturating_sub(150);
        let upper = expected + 1000;
        assert!(
            dt_ms >= lower && dt_ms <= upper,
            "gap {i} between attempt {} and attempt {} = {dt_ms}ms; expected ~{expected}ms",
            observed[i].0,
            observed[i + 1].0,
        );
    }
}

fn live_unit(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// RPC-415/416: drive a MockBackend App through disconnect → reconnect →
/// `Action::Reconnected` with a focused session "s-1" showing an inline
/// reconnecting line, and prove the App respawns the full subscriber set
/// (count AND a live post-reconnect work-units broadcast). Returns the App
/// for the caller's inline-line assertions.
pub async fn app_respawns_subscribers_after_reconnect(
) -> Result<App, codelet_test_helpers::TestError> {
    let mock = Arc::new(MockBackend::new());
    let app_backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(app_backend);
    app.bootstrap().await.or_fail("mock App bootstrap")?;
    let full_stream_count = app.subscriber_task_count();
    // BUG-181/RPC-434/RPC-437/RPC-445: work_units, chunks, logs,
    // status_changes, session_created, checkpoints_progress, git_state,
    // session_removed, session_cleared, the checkpoint-counts push, the
    // workspace-keyed work-units stream and (RPC-454) the workspace-keyed
    // git-state stream.
    assert_eq!(
        full_stream_count, 12,
        "bootstrap must spawn all twelve subscriber tasks"
    );
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app.dispatch(Action::Disconnected);
    assert_eq!(
        reconnect_lines(&app, &SessionId::new("s-1")).len(),
        1,
        "an inline reconnecting line must show in the focused session after disconnect"
    );
    mock.disconnect_all();
    // TEST-042: no settle sleep; let the old subscriber loops observe the
    // close, then reconnect.
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }
    mock.reconnect_all();
    app.dispatch(Action::Reconnected);
    assert_eq!(
        app.subscriber_task_count(),
        full_stream_count,
        "Action::Reconnected must respawn the full set of subscriber tasks"
    );
    // A respawned subscriber binds to the new broadcast on its first poll,
    // and a send to a channel with no receiver yet is dropped, so the push
    // is repeated until it arrives (under the deadlock guard).
    let mut saw_respawned_stream = false;
    let _ = codelet_test_helpers::deadlock_guard("a post-reconnect work_units push", async {
        while !saw_respawned_stream {
            mock.push_work_units(vec![live_unit("RPC415-LIVE")]);
            tokio::task::yield_now().await;
            while let Some(action) = app.try_recv_action() {
                if matches!(&action, Action::WorkUnitsLoaded(units) if units.iter().any(|u| u.id == "RPC415-LIVE"))
                {
                    saw_respawned_stream = true;
                }
            }
        }
    })
    .await;
    assert!(
        saw_respawned_stream,
        "a post-reconnect work_units broadcast must reach the bus, proving the respawned \
         subscriber tasks are live and bound to the new client"
    );
    Ok(app)
}
