//! Catch-all capability fallback + post-dispatch housekeeping.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator stays under the
//! 300-LoC ceiling. The capability chain (`try_dispatch_*`) is a
//! last-resort fall-through for Actions not routed by the explicit
//! match arms; after it runs, every action still goes through
//! `navigator.apply_action` + the Compositor update + the MUX-001
//! persisted-config sync.

use crate::components::Action;

use super::state::App;

impl App {
    /// Route the action through the capability `try_dispatch_*` chain.
    /// Returns true when one of the capabilities claimed the action.
    pub(crate) fn dispatch_capability_fallback(&mut self, action: &Action) -> bool {
        self.try_dispatch_model_selector(action)
            || self.try_dispatch_model_thinking_dialogs(action)
            || self.try_dispatch_pause_hitl(action)
            || self.try_dispatch_exec_stdin(action)
            || self.try_dispatch_provider_settings(action)
            || self.try_dispatch_blocklist(action)
            || self.try_dispatch_changed_files(action)
            || self.try_dispatch_viewer(action)
            || self.try_dispatch_work_unit_search(action)
            || self.try_dispatch_checkpoints(action)
            || self.try_dispatch_merge_worktree(action)
            || self.try_dispatch_worktrees(action)
            || self.try_dispatch_slash_schedule(action)
            || self.try_dispatch_slash_loop(action)
            || self.try_dispatch_create_session_dialog(action)
            || self.try_dispatch_supervisor_links(action)
            || self.try_dispatch_dialog_dismiss(action)
    }

    /// Post-dispatch housekeeping shared by every `dispatch` tick:
    /// view routing (`apply_action`), Compositor update, the MUX-001
    /// persisted-config lockstep + R6 auto-save on mux exit, and the
    /// render flag. `mux_enabled_before` is the mux enabled-flag captured
    /// at the top of the tick so a mux EXIT can be detected for R6.
    pub(crate) fn finish_dispatch_tick(&mut self, action: Action, mux_enabled_before: bool) {
        self.navigator.apply_action(&action);
        let _ = self.compositor.update(action);
        // MUX-001: keep the persisted MuxState config in lockstep with
        // the live Navigator mux layout so `app.mux_state().config()`
        // always reflects the current grid (tests + /mux save read it).
        self.mux_state
            .config_mut()
            .clone_from(self.navigator.mux.config());
        // R6: auto-save on mux exit — persist the post-exit config
        // (enabled=false) so a restart comes back with mux off.
        if mux_enabled_before && !self.navigator.mux.config().enabled {
            if let Err(err) = self.save_mux_config() {
                tracing::warn!(error = %err, "mux-exit auto-save failed (non-fatal)");
            }
        }
        self.should_render = true;
    }
}
