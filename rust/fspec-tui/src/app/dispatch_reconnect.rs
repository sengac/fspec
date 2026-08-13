//! `App::handle_reconnected` — RPC-415 respawn-on-Reconnected handler.
//!
//! Extracted from `app/dispatch.rs` to keep that file's `match` slim (the
//! 300-LoC ceiling pinned by RPC-049 / RPC-053). On `Action::Reconnected`
//! the transport supervisor has swapped in a fresh RPC client, so the five
//! broadcast subscriber loops spawned at bootstrap have exited on
//! `RecvError::Closed`. We respawn them against the new client's receivers
//! and preserve the RPC-011 one-shot re-bootstrap.

use crate::components::Action;

use codelet_rpc_types::SessionId;
use ratatui::style::Color;
use std::time::Duration;

use super::state::App;

/// RPC-416: canonical inline reconnect-notice strings.
const RECONNECTING: &str = "\u{27F3} Reconnecting\u{2026}";
const RECONNECTED: &str = "\u{2713} Reconnected";
/// Auto-dismiss delay for the success line (design: ~1.5-2s).
const DISMISS_DELAY: Duration = Duration::from_millis(1500);

impl App {
    /// RPC-416: on `Action::Disconnected`, push a single inline
    /// `⟳ Reconnecting…` notice into the FOCUSED session and track its
    /// `(SessionId, seq)`. On a RE-DROP during the success window, abort
    /// the pending auto-dismiss timer and REVERT the existing line in
    /// place (reusing its seq) rather than pushing a second line. If the
    /// originating session was CLOSED before the re-drop, the stale
    /// tracking is cleared and a FRESH reconnecting line is pushed into
    /// the currently-focused session instead. No focused session → no-op
    /// (never panics).
    pub(crate) fn handle_disconnected(&mut self) {
        // Re-drop: a notice already exists — revert it in place and
        // cancel any armed auto-dismiss timer so the reconnecting line
        // survives. If the originating session has since been CLOSED
        // (`session_context_mut_for` → None) we must NOT leave the stale
        // tracking pointing at the dead session; instead we clear it and
        // fall through to push a fresh reconnecting line into whatever
        // session is currently focused.
        if let Some((sid, seq)) = self.reconnect_notice.clone() {
            self.abort_reconnect_dismiss();
            if let Some(ctx) = self.agent_view_store.session_context_mut_for(&sid) {
                ctx.replace_notice_by_seq(seq, RECONNECTING, Color::Yellow);
                return;
            }
            // Originating session gone: drop the stale tracking and fall
            // through to the fresh-line path below.
            self.reconnect_notice = None;
        }
        let Some(sid) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if let Some(ctx) = self.agent_view_store.session_context_mut_for(&sid) {
            let seq = ctx.push_notice_line(RECONNECTING, Color::Yellow);
            self.reconnect_notice = Some((sid, seq));
        }
    }

    /// RPC-416: on `Action::Reconnecting(n)`, update the tracked notice
    /// line in place to show the attempt count. Never pushes a new line.
    pub(crate) fn handle_reconnecting(&mut self, attempt: u32) {
        if let Some((sid, seq)) = self.reconnect_notice.clone() {
            if let Some(ctx) = self.agent_view_store.session_context_mut_for(&sid) {
                ctx.replace_notice_by_seq(
                    seq,
                    format!("{RECONNECTING} (attempt {attempt})"),
                    Color::Yellow,
                );
            }
        }
    }

    /// RPC-416: replace the tracked notice line in place with the
    /// `✓ Reconnected` success message and arm the auto-dismiss timer.
    fn show_reconnected_success(&mut self) {
        let Some((sid, seq)) = self.reconnect_notice.clone() else {
            return;
        };
        if let Some(ctx) = self.agent_view_store.session_context_mut_for(&sid) {
            ctx.replace_notice_by_seq(seq, RECONNECTED, Color::Green);
        }
        self.arm_reconnect_dismiss(sid, seq);
    }

    /// RPC-416: spawn the sleep→`ClearReconnectNotice` timer, storing the
    /// handle so a re-drop can abort it.
    fn arm_reconnect_dismiss(&mut self, session_id: SessionId, seq: u64) {
        self.abort_reconnect_dismiss();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(DISMISS_DELAY).await;
            let _ = action_tx.send(Action::ClearReconnectNotice { session_id, seq });
        });
        self.reconnect_dismiss_handle = Some(handle);
    }

    /// RPC-416: abort any armed auto-dismiss timer (re-drop / supersede).
    fn abort_reconnect_dismiss(&mut self) {
        if let Some(handle) = self.reconnect_dismiss_handle.take() {
            handle.abort();
        }
    }

    /// RPC-416: on the auto-dismiss `Action::ClearReconnectNotice`,
    /// remove the notice chunk from the ORIGINATING session. Silent
    /// no-op when the session closed, the seq is gone, or the notice was
    /// already superseded by a fresh re-drop (tracked seq differs).
    pub(crate) fn handle_clear_reconnect_notice(&mut self, session_id: &SessionId, seq: u64) {
        // Only clear if this is still the tracked notice — a re-drop may
        // have superseded it with a fresh reconnecting line.
        if self.reconnect_notice.as_ref() != Some(&(session_id.clone(), seq)) {
            return;
        }
        if let Some(ctx) = self.agent_view_store.session_context_mut_for(session_id) {
            ctx.remove_notice_by_seq(seq);
        }
        self.reconnect_notice = None;
        self.reconnect_dismiss_handle = None;
    }

    /// RPC-415: on reconnect, respawn the five broadcast subscriber tasks
    /// (work_units / chunks / logs / status_changes / session_created) bound
    /// to the CURRENT client's `*_rx()` receivers, then run the RPC-011
    /// one-shot re-bootstrap (`list_work_units` refetch + `create_session`).
    ///
    /// The old subscriber loops have exited (or will shortly) on
    /// `RecvError::Closed` because the transport supervisor dropped the old
    /// client's broadcast Senders on the WS drop. We abort + clear the dead
    /// handles FIRST so repeated Reconnected actions under flapping cannot
    /// accumulate N×5 tasks — `subscriber_task_count()` stays at the fixed
    /// stream count. Respawn reuses the SINGLE `spawn_subscriber_tasks()`
    /// code path shared with `App::bootstrap` (DRY — no duplicated loops).
    pub(crate) fn handle_reconnected(&mut self) {
        // RPC-416: replace the inline reconnect notice in place with the
        // success message and arm the auto-dismiss timer (was: remove the
        // DisconnectDialog modal layer).
        self.show_reconnected_success();
        for task in self.subscriber_tasks.drain(..) {
            task.abort();
        }
        self.spawn_subscriber_tasks();
        // RPC-011 rule [5]: re-bootstrap on reconnect.
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let active_session_tx = self.active_session_tx.clone();
        tokio::spawn(async move {
            if let Ok(units) = backend.list_work_units().await {
                let _ = action_tx.send(Action::WorkUnitsLoaded(units));
            }
            if let Ok(session) = backend.create_session(None).await {
                // PROV-101 FIX 1: an empty id is a decline — surface it
                // explicitly and never seed it as the active session.
                crate::app::session_creation::route_bootstrap_create_session(
                    session,
                    &active_session_tx,
                    &action_tx,
                );
            }
        });
    }
}
