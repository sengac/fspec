//! `App` TUI-106/TUI-109 navigator view-status accessors (test seams).
//!
//! Factored out of `app/state.rs` so the parent stays under the
//! 300-LoC ceiling pinned by `source_shape_stores_rpc012`. These are
//! pure read-throughs onto the Navigator lazy mode views.

use super::state_types::App;

impl App {
    /// TUI-106: the checkpoints view's list stage has flushed AND the
    /// view holds no checkpoints — the real "No checkpoints available"
    /// empty state may surface (loading ≠ empty discriminator).
    pub fn navigator_checkpoints_loaded_and_empty(&self) -> bool {
        self.navigator.checkpoints.load.is_loaded()
            && !self.navigator.checkpoints.load.is_loading()
            && self.navigator.checkpoints.is_empty()
    }

    /// TUI-106: the changed-files view's scan has flushed AND the view
    /// holds no files — the real "No changed files" empty state may
    /// surface (loading ≠ empty discriminator).
    pub fn navigator_changed_files_loaded_and_empty(&self) -> bool {
        self.navigator.changed_files.load.is_loaded()
            && !self.navigator.changed_files.load.is_loading()
            && self.navigator.changed_files.is_empty()
    }

    /// TUI-106: the checkpoints cascade's in-flight stage label ("Loading
    /// files for {name}…" / "Loading diff for {path}…"), or the list
    /// label while the list is loading; `None` once the cascade idles.
    pub fn navigator_checkpoints_active_label(&self) -> Option<String> {
        self.navigator.checkpoints.load.active_label()
    }

    /// TUI-109: a clone of the CheckpointsView's LoadingDialog (test
    /// seam for asserting the counter row the progress fold feeds).
    pub fn navigator_checkpoints_loading_dialog(
        &self,
    ) -> crate::components::loading_dialog::LoadingDialog {
        self.navigator.checkpoints.loading.clone()
    }

    /// TUI-109: the number of checkpoints folded into the
    /// CheckpointsView (test seam).
    pub fn navigator_checkpoints_len(&self) -> usize {
        self.navigator.checkpoints.checkpoints_len()
    }

    /// TUI-109: whether the CheckpointsView's list stage has flushed
    /// (test seam for the progress stale-drop guard).
    pub fn navigator_checkpoints_list_loaded(&self) -> bool {
        self.navigator.checkpoints.load.is_loaded()
    }

    // ── RPC-009/RPC-011 legacy shims (migrated to the new stores) ───────

    /// Snapshot of work units currently in the BoardStore (legacy
    /// accessor used by RPC-009 tests). Returns a flat list in
    /// insertion order — the new code uses
    /// [`crate::store::BoardStore::column_units`] for column-grouped
    /// queries.
    pub fn work_units_snapshot(&self) -> Vec<codelet_rpc_types::WorkUnitInfo> {
        use crate::store::COLUMN_ORDER;
        let mut out = Vec::new();
        for column in COLUMN_ORDER {
            for unit in self.board_store.column_units(column) {
                out.push(unit.clone());
            }
        }
        out
    }

    /// AgentViewStore's current session id (legacy accessor used by
    /// RPC-009/RPC-011 tests).
    pub fn current_session(&self) -> Option<codelet_rpc_types::SessionId> {
        self.agent_view_store.current_session().cloned()
    }

    /// RPC-430: read the pre-session debug-capture toggle flag.
    pub fn pre_session_debug_enabled(&self) -> bool {
        self.pre_session_debug_enabled
    }

    /// RPC-430: set the pre-session debug-capture toggle flag.
    pub fn set_pre_session_debug_enabled(&mut self, val: bool) {
        self.pre_session_debug_enabled = val;
    }

    // ── MUX-001: mux state ──────────────────────────────────────────────

    /// Borrow the mux persistence state (shared fspec-config.json, tui.mux).
    pub fn mux_state(&self) -> &crate::store::MuxState {
        &self.mux_state
    }

    /// Load the persisted mux config from the shared `fspec-config.json`
    /// (`tui.mux`; R6: missing key → default preset). Called at
    /// bootstrap; the loaded config is mirrored into the Navigator's
    /// live mux layout.
    ///
    /// BUG-167: the shared-config dirs resolve themselves (the
    /// `codelet_sessions::mux_config_persistence` globals read the
    /// process-global data directory + current dir — the same CONFIG-008
    /// resolution every other shared-config persistence uses), so no
    /// manual persist-dirs wiring is needed here or in `App::new`.
    ///
    /// BUG-175: the persisted `enabled` flag is a SAVED LAYOUT
    /// PREFERENCE, not a runtime mode. A restart always lands on the
    /// single Board view, so the flag is force-disabled on BOTH the
    /// persistence mirror and the live layout — a persisted
    /// `enabled=true` (written while the user was in the grid) must not
    /// leak into view routing outside the grid (BackToBoard /
    /// EnterWorkUnit gate on `active_view == ViewMode::Mux`, and the
    /// R6 auto-save reads the mirror). `/mux on` re-enables the grid
    /// with the saved layout.
    pub fn load_mux_config(&mut self) {
        self.mux_state.load();
        self.mux_state.config_mut().enabled = false;
        self.navigator.mux.config = self.mux_state.config().clone();
    }

    /// Persist the live mux config under `tui.mux` in the shared
    /// `fspec-config.json` (R6). Best-effort: failures surface as `Err`
    /// for the caller to log.
    pub fn save_mux_config(&mut self) -> Result<(), String> {
        self.mux_state
            .config_mut()
            .clone_from(self.navigator.mux.config());
        self.mux_state.save()
    }
}
