//! RPC-056 — BlocklistView + `/blocklist` slash command end-to-end.
//!
//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(Blocklist)`
//! so it reaches the backend's `blocklist_list()` RPC and opens the new
//! `BlocklistView`. Covers view rendering, key handling (j/k, Up/Down,
//! Space/Enter toggle, Esc close), the per-session disabled-rules lift to
//! `AgentViewStore`, and the `derive_category` helper.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::views::blocklist::{derive_category, BlocklistEvent, BlocklistView};
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{BlocklistRuleInfo, SessionId};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::time::timeout;

mod common;
use common::MockBackend;

#[path = "blocklist_rpc056/dispatch.rs"]
mod dispatch;
#[path = "blocklist_rpc056/interaction.rs"]
mod interaction;
#[path = "blocklist_rpc056/rendering.rs"]
mod rendering;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn rule(id: &str, pattern: &str, action: &str, source: &str) -> BlocklistRuleInfo {
    BlocklistRuleInfo {
        id: id.to_string(),
        pattern: pattern.to_string(),
        action: action.to_string(),
        reason: format!("reason for {id}"),
        guidance: Some(format!("guidance for {id}")),
        source: source.to_string(),
    }
}

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

fn fresh_app(mock: Arc<MockBackend>) -> App {
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

fn render_view(view: &mut BlocklistView, disabled: &HashSet<String>) -> String {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| {
        view.render(frame.area(), frame.buffer_mut(), disabled);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}
