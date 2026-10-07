//! MENU-009 — `App::dispatch` arm for the right-aligned Zone C action
//! button (`Action::MenuZoneCActivate`).
//!
//! Feature: spec/features/menubar-zone-c-right-aligned-new-agent-close-agent-buttons.feature
//!          spec/features/bug199-agent-bar-zone-c-new-agent-close-agent-esc-semantics.feature
//! Card: MENU-009 (R2, R4), BUG-199 (R1, R2).
//!
//! Split out of `dispatch_menu.rs` for the 300-LoC ceiling (the
//! `no_codelet_napi_reference_and_300_loc_ceiling` source-shape guard
//! covers every file under `app/`). The arm reuses existing bus
//! actions — `OpenCreateSessionDialog{preselect: None}` for the agent
//! bar's `New Agent` (BUG-199 R1: start a NEW agent, board-button
//! parity) and `AgentEscPressed` for `Close Agent [esc]` (BUG-199 R2:
//! the button IS the Esc gesture) — so no new backend/RPC work exists
//! here; this module only resolves the button index against the ACTIVE
//! surface's own `zone_c` slice.

use crate::components::menu_bar::items::MenuAction;
use crate::components::Action;
use crate::views::ViewMode;

use super::state::App;

impl App {
    /// MENU-009 R2 / BUG-199: a Zone C button activation (Enter / left
    /// click). The `index` addresses the ACTIVE surface's own `zone_c`
    /// slice — the agent view already cleared its bar focus in-view
    /// (`activate_menu_zone_c`), so only the resolution differs per
    /// surface.
    pub(crate) fn dispatch_menu_zone_c(&mut self, index: usize) {
        if self.navigator.active_view == ViewMode::Agent {
            // The agent bar's `[New Agent, Close Agent [esc]]` —
            // resolve through `AGENT_ZONE_C`'s registry `MenuAction`
            // (payload-free: BUG-199 R1 drops the R8 current-session
            // substitution — the button starts a NEW agent, and
            // `Close Agent` runs the Esc cascade untouched by a
            // session target).
            let Some(button) = crate::views::agent::menu_render::AGENT_ZONE_C.get(index) else {
                return;
            };
            match button.action {
                // BUG-199 R1: the agent bar's 'New Agent' ALWAYS starts
                // a new agent — mount the CreateSessionDialog as an
                // overlay (the RPC-097 reopen #1 contract: the user
                // stays on the Agent view until the dialog is
                // confirmed; the current session is never disturbed).
                // Idempotent on the dialog id (RPC-060).
                MenuAction::NewAgent => {
                    self.handle_open_create_session_dialog(None);
                }
                // BUG-199 R2: 'Close Agent [esc]' is the Esc gesture —
                // the exact `AgentEscPressed` cascade (running →
                // interrupt, non-empty draft → clear, else the 'Exit
                // Session?' confirmation dialog).
                MenuAction::CloseAgent => self.handle_agent_esc_pressed(),
                _ => {}
            }
            return;
        }
        // The board's single `New Agent` button: the EXACT
        // `.`-key semantics (R4 of the card) — the selected
        // work unit's attached session, else the None-target
        // path (`handle_open_agent_view`: resume the first
        // open session, else mount the CreateSessionDialog).
        // The activation clears the bar's ring focus (the
        // `execute_dropdown_row` parity — executing is a
        // 'leave the bar' gesture).
        let target = self
            .board_store
            .selected_work_unit()
            .and_then(|unit| self.board_store.session_for(&unit.id).cloned());
        self.board_store.dismiss_menu();
        self.dispatch(Action::OpenAgentView(target));
    }
}
