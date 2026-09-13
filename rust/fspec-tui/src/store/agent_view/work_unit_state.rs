//! RPC-050 — per-session work-unit binding state held by
//! `AgentViewStore`.
//!
//! Feature files:
//!   - spec/features/work-unit-attach-binding.feature
//!   - spec/features/slash-command-detach-and-work-unit-binding.feature
//!
//! This sub-module hosts the AgentViewStore accessors for the new
//! per-session `work_unit_context_by_session: HashMap<SessionId,
//! WorkUnitContext>` slot — updated by `Action::WorkUnitAttached`
//! (BoardView attach path) and cleared by `Action::WorkUnitDetached`
//! (`/detach` slash command). Read by the SessionHeader chip renderer
//! in `views/agent.rs::render_with_store`.
//!
//! Also hosts `reset_token_state(&SessionId)` — invoked by the
//! `Action::WorkUnitDetached` arm to mirror the TS
//! `prepareForNewSession` tokenUsage reset.
//!
//! BUG-180: also hosts `sync_work_unit_contexts(&[WorkUnitInfo])` — the
//! snapshot projection invoked by `App::dispatch` on
//! `Action::WorkUnitsLoaded` AFTER `BoardStore::replace_work_units`, so
//! the per-session chip (and the legacy fallback slots) track the live
//! status instead of freezing at attach time.
//!
//! The block lives in its own sub-module so the parent `agent_view.rs`
//! continues to satisfy the 300-LoC source-shape ceiling pinned by
//! `rpc025-source-shape.feature` and `slash-command-detach-source-shape.feature`.

use codelet_rpc_types::{SessionId, WorkUnitContext, WorkUnitInfo};

use super::AgentViewStore;
use crate::terminal::sanitize::sanitize_for_terminal;

impl AgentViewStore {
    /// Borrow the per-session `WorkUnitContext` bound to `session`, if any.
    pub fn work_unit_context_for(&self, session: &SessionId) -> Option<&WorkUnitContext> {
        self.work_unit_context_by_session.get(session)
    }

    /// Bind a `WorkUnitContext` to `session`. Replaces any existing
    /// binding. Mutated only on the App task.
    pub fn set_work_unit_context(&mut self, session: SessionId, ctx: WorkUnitContext) {
        self.work_unit_context_by_session.insert(session, ctx);
    }

    /// Clear the per-session work-unit binding for `session`. No-op
    /// when no binding exists.
    pub fn clear_work_unit_context(&mut self, session: &SessionId) {
        self.work_unit_context_by_session.remove(session);
    }

    /// RPC-050: wipe the cached TokenState for `session` so the
    /// SessionHeader's token badges reset to defaults on the next
    /// render. Mirrors TS `prepareForNewSession`'s tokenUsage reset.
    pub fn reset_token_state(&mut self, session: &SessionId) {
        self.token_state_by_session.remove(session);
    }

    /// BUG-180: project a fresh `WorkUnitsWatcher` snapshot onto every
    /// per-session work-unit binding so the SessionHeader chip tracks
    /// the LIVE status instead of freezing at attach time.
    ///
    /// Semantics:
    /// - A binding whose unit id appears in `units` gets its `title`
    ///   and `status` rewritten from the snapshot (the snapshot is the
    ///   single source of truth — `spec/work-units.json`).
    /// - A binding whose unit is ABSENT from `units` (deleted) is
    ///   preserved verbatim: a deleted unit must not silently detach
    ///   the session. The board shows the unit as gone; the header
    ///   keeps painting the last-known status until the user runs
    ///   `/detach` or attaches a different unit.
    /// - The legacy fallback slots (`current_work_unit_id` /
    ///   `current_work_unit_status`, the RPC-029 pre-per-session
    ///   chrome source) are re-synced from the snapshot when they hold
    ///   an id present in `units`. They are cleared when the snapshot
    ///   no longer contains that id, matching the "deleted units
    ///   vanish from the board" semantics for the fallback path — the
    ///   per-session bindings are the authoritative slots and survive
    ///   deletion (see rule 2).
    ///
    /// Called ONLY from `App::dispatch` on `Action::WorkUnitsLoaded`
    /// (single-writer tenere pattern, RPC-009) after
    /// `BoardStore::replace_work_units`. Idempotent: re-syncing with
    /// unchanged values is a no-op in effect.
    pub fn sync_work_unit_contexts(&mut self, units: &[WorkUnitInfo]) {
        // Fast lookup: unit id → (title, status). Built per call; the
        // snapshot is at most the board size (hundreds), and this runs
        // once per watcher push, not per frame.
        //
        // TUI-111: the snapshot title is sanitized on ingress so the
        // SessionHeader WU chip (and legacy fallback slots) always
        // paint clean text — same field definition as
        // `crate::store::work_unit_sanitize::sanitize_work_unit`.
        let lookup = |id: &str| -> Option<(String, String)> {
            units
                .iter()
                .find(|u| u.id == id)
                .map(|u| (sanitize_for_terminal(&u.title), u.status.clone()))
        };

        for ctx in self.work_unit_context_by_session.values_mut() {
            if let Some((title, status)) = lookup(&ctx.id) {
                ctx.title = title;
                ctx.status = status;
            }
            // Absent id → binding preserved (last-known status).
        }

        if let Some(id) = self.current_work_unit_id.clone() {
            match lookup(&id) {
                Some((_, status)) => self.current_work_unit_status = Some(status),
                None => self.current_work_unit_status = None,
            }
        }
    }
}
