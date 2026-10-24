//! RPC-434 / RPC-437 — `Action::SessionRemoved` / `Action::SessionCleared`
//! handlers + connect-time snapshot replay.
//!
//! Feature: spec/features/session-events-tui-consumption.feature
//! Feature: spec/features/session-events-tui-cleared-consumption.feature
//!
//! Two RPC-434 behaviors live here (factored from `app/dispatch.rs` /
//! `app/bootstrap.rs` to keep those files under the 300-LoC ceiling):
//!
//!  * `handle_session_removed` — the idempotent App-level fold for a
//!    session destroyed on the daemon (from ANY connected machine).
//!    Mirrors the `dispatch_agent_exit.rs` `CloseSession` cleanup minus
//!    the backend destroy (the session is ALREADY gone on the daemon —
//!    this is the REMOTE event, never the local close path's trigger):
//!
//!    1. Gate on `remove_session_if_open` — a session whose tab is not
//!       open (local close already removed it) makes the WHOLE handler
//!       a no-op (scenario "Removal events are idempotent").
//!    2. Board detach — every work-unit attachment pointing at the
//!       removed session is released (the TS `detachSession` parity
//!       step; without it `BoardView::selected_session` would keep
//!       returning the dead SessionId).
//!    3. Per-session work-unit binding drop (`clear_work_unit_context`)
//!       — the SessionHeader chip must not keep painting the doomed
//!       session's `#N (WU-ID: status)` segment.
//!    4. Current-work-unit pointer clear when it pointed at a WU we
//!       just detached (TS parity step 3 —
//!       `sessionStore.setCurrentWorkUnit(null, null)`), so the
//!       surviving focused session doesn't inherit a stale WU.
//!    5. `mux_sync_window` — re-clamp the mux agent window to the
//!       shrunken open-session list (agent slots stay in the grid).
//!
//!    Focus: `remove_session_if_open` already clamps the focused index
//!    to a surviving session (or to the empty list — the board), so
//!    the "focus falls back" rule needs no extra step.
//!
//!  * `replay_session_snapshot` (bootstrap) /
//!    `spawn_replay_session_snapshot` (reconnect) — the connect-time /
//!    reconnect open-session snapshot (RPC-433
//!    `Envelope::SessionSnapshot` over WS;
//!    `SharedFspecService::live_sessions_snapshot` embedded). Each
//!    entry rides the EXISTING `Action::SessionCreated` handler
//!    (`handle_session_created` — idempotent append), so a tab that
//!    already exists is untouched. Bootstrap runs it synchronously
//!    BEFORE `spawn_session_restore` so the BUG-206 auto-restore
//!    re-asserts the daemon active pointer AFTER replay (R4). No
//!    replay flag on the wire (research §2.1).

use std::sync::Arc;

use codelet_rpc_types::{SessionId, SessionInfo, SessionStatus};

use crate::components::Action;
use crate::transport::FspecBackend;

use super::state::App;

/// RPC-434 (R4): the shared snapshot fetch — both the bootstrap and
/// the reconnect replay paths pull the open-session set through
/// `FspecBackend::session_snapshot` (in-memory only; best effort — a
/// failure degrades to an empty list and seeds no tabs).
pub(crate) async fn fetch_session_snapshot(backend: &Arc<dyn FspecBackend>) -> Vec<SessionInfo> {
    backend.session_snapshot().await
}

impl App {
    /// RPC-434 (R4): bootstrap-time replay — fold EVERY snapshot
    /// `SessionInfo` through the existing `handle_session_created`
    /// (the `Action::SessionCreated` handler: idempotent append, side
    /// effects fire once per fresh tab). Synchronous AFTER the fetch
    /// and MUST run BEFORE `spawn_session_restore` so the BUG-206
    /// auto-restore re-asserts the daemon active pointer after the
    /// tabs exist. The last replayed session grabs focus (most-recent
    /// intuition) unless the restore re-asserts the daemon pointer.
    pub(crate) async fn replay_session_snapshot(&mut self) {
        for info in fetch_session_snapshot(&self.backend).await {
            self.handle_session_observed(info);
        }
    }

    /// RPC-434 (R4): reconnect-time replay — the reconnect handler is
    /// synchronous (dispatch-driven), so the fetch runs on a spawned
    /// task that rides the existing `Action::SessionCreated` frames on
    /// the action bus (same fold, same idempotency). Spawned BEFORE
    /// `spawn_session_restore` so the replay frames normally reach the
    /// bus ahead of the restore's `ReattachSession` (restore resolves
    /// through two RPC round-trips first). A fetch failure seeds no
    /// tabs and never blocks the reconnect.
    pub(crate) fn spawn_replay_session_snapshot(&mut self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            for info in fetch_session_snapshot(&backend).await {
                let _ = action_tx.send(Action::SessionObserved(info));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-434 (R3): fold a daemon-side session removal. IDEMPOTENT —
    /// a session whose tab is not open is a complete no-op (the local
    /// close path removes the tab synchronously BEFORE the daemon fires
    /// the event, so the double-fire never double-cleans).
    pub(crate) fn handle_session_removed(&mut self, session: SessionId) {
        self.transcript_sync.remove(&session);
        // Step 1: the gate. `false` = tab already gone → no-op.
        if !self.agent_view_store.remove_session_if_open(&session) {
            return;
        }
        // Step 2: board detach — release every work-unit attachment
        // whose attached session IS the removed one (snapshot the keys
        // first: detaching mutates the attachment map).
        let doomed_wus: Vec<String> = self
            .board_store
            .session_attachments_for(&session)
            .iter()
            .map(|(wu, _)| (*wu).to_string())
            .collect();
        for wu in doomed_wus.iter() {
            self.board_store.detach_session(wu);
        }
        // Step 3: drop the per-session work-unit binding so the
        // SessionHeader chip stops painting the doomed session's WU.
        self.agent_view_store.clear_work_unit_context(&session);
        self.rebuild_focus_occupancy();
        // Step 4: current-work-unit pointer clear (TS parity step 3)
        // only when it pointed at a WU we just detached — never steal
        // a surviving session's live binding.
        let current_wu = self
            .agent_view_store
            .current_work_unit_id()
            .map(str::to_string);
        if let Some(current_wu) = current_wu {
            if doomed_wus.contains(&current_wu) {
                self.agent_view_store.set_current_work_unit(None, None);
            }
        }
        // Step 5: re-clamp the mux agent window to the shrunken
        // session list (focus already clamped inside
        // `remove_session_if_open` — step 1 above).
        self.mux_sync_window();
    }

    /// RPC-437 (R3): fold a daemon-side session clear (the RPC-436
    /// `SessionEvent::Cleared` frame). IDEMPOTENT and per-session —
    /// a double-fire (local /clear + echoed event, or two consecutive
    /// clears) is a no-op on the second pass. Never touches focus or
    /// other sessions.
    ///
    /// 1. Gate on the session's `SessionContext` — a session whose tab
    ///    is not open (never opened, or closed locally) makes the
    ///    WHOLE handler a no-op (no tab creation, no panic).
    /// 2. Per-session reset: scrollback + in-flight slots +
    ///    `input_draft` (complete for background sessions too).
    /// 3. Focused session only: the AgentView-local step (input
    ///    buffer + popups) through the shared
    ///    `AgentView::reset_local_scrollback` — the SAME code path
    ///    the local /clear uses (DRY).
    /// 4. RPC-100 chrome mirror (the chunk-side Cleared state reset):
    ///    status → Cleared + token state reset + compaction reduction
    ///    cleared, so the SessionHeader badge returns to [0%] even
    ///    when the chunk stream lags the lifecycle event.
    pub(crate) fn handle_session_cleared(&mut self, session: SessionId) {
        self.transcript_sync.remove(&session);
        // Step 1: the gate — no open tab is a complete no-op.
        if self
            .agent_view_store
            .session_context_for(&session)
            .is_none()
        {
            return;
        }
        // Step 2: per-session reset (store-side, focus-independent).
        if let Some(ctx) = self.agent_view_store.session_context_mut_for(&session) {
            ctx.reset_scrollback();
            ctx.input_draft.clear();
        }
        // Step 3: focused-session view-local reset — ONE shared code
        // path with the local /clear (DRY). A background-session
        // clear never touches the focused input buffer.
        if self.agent_view_store.current_session() == Some(&session) {
            self.navigator
                .agent
                .reset_local_scrollback(&mut self.agent_view_store);
        }
        // Step 4: RPC-100 chrome mirror — the SessionHeader badge
        // returns to the cleared state even when the chunk stream
        // lags (or never delivers) the SessionStateChange frame.
        self.agent_view_store
            .set_session_status(session.clone(), SessionStatus::Cleared);
        self.agent_view_store.reset_token_state(&session);
        self.agent_view_store.clear_compaction_reduction(&session);
    }
}
