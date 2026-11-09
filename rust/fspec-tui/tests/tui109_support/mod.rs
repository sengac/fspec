//! BUG-355: helpers for tests/tui109_checkpoints_progress.rs (the
//! support-module pattern: helpers here, every test in the one root file).

use std::sync::Arc;

use codelet_fspec_tui::components::loading_dialog::{render_loading_dialog, LoadingDialog};
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{CheckpointInfo, CheckpointsProgress};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::common::MockBackend;

pub fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    (App::new(backend), mock)
}

pub fn checkpoint_info(wu: &str, name: &str) -> CheckpointInfo {
    CheckpointInfo {
        work_unit_id: wu.into(),
        name: name.into(),
        timestamp: "2026-01-01T00:00:00Z".into(),
        is_automatic: false,
    }
}

/// `count` checkpoints named `cp-000`.. on AUTH-001.
pub fn checkpoint_list(count: usize) -> Vec<CheckpointInfo> {
    (0..count)
        .map(|i| checkpoint_info("AUTH-001", &format!("cp-{i:03}")))
        .collect()
}

pub fn progress(loaded: u32, total: u32, done: bool) -> CheckpointsProgress {
    CheckpointsProgress {
        loaded,
        total,
        done,
    }
}

/// Drain the App's action bus until a matching action arrives or 200ms
/// elapses (mirrors `app_bootstrap_rpc009.rs::wait_for_action`).
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

/// Paint `dialog` into a 60x14 buffer and return its text, row by row.
pub fn render_text(dialog: &LoadingDialog) -> String {
    let area = Rect::new(0, 0, 60, 14);
    let mut buf = Buffer::empty(area);
    render_loading_dialog(area, &mut buf, dialog, 0);
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}
