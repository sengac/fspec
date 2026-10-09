//! MENU-009 — `App::dispatch` arm for the right-aligned Zone C action
//! button (`Action::MenuZoneCActivate`).
//!
//! Feature: spec/features/menubar-zone-c-right-aligned-new-agent-close-agent-buttons.feature
//!          spec/features/bug199-agent-bar-zone-c-new-agent-close-agent-esc-semantics.feature
//!          spec/features/bug204-close-agent-button-always-shows-exit-confirmation-dialog.feature
//!          spec/features/board-new-agent-gesture-prompts-create-session-dialog.feature
//! Card: MENU-009 (R2, R4), BUG-199 (R1, R2), BUG-204 (Close Agent →
//!       always the dialog), BUG-203 (board button parity).
//!
//! Split out of `dispatch_menu.rs` for the 300-LoC ceiling (the
//! `no_codelet_napi_reference_and_300_loc_ceiling` source-shape guard
//! covers every file under `app/`). The arm reuses existing bus
//! actions — `OpenCreateSessionDialog{preselect: None}` for BOTH the
//! agent bar's `New Agent` (BUG-199 R1: start a NEW agent) and the
//! board's single `New Agent` (BUG-203: the board button gets the same
//! BUG-199 semantics — it NEVER resumes the selected unit's session),
//! and (BUG-204, superseding BUG-199 R2) the `Close Agent` button
//! ALWAYS mounts the `ExitConfirmationDialog` (RPC-098 L7) — it does
//! NOT run the `AgentEscPressed` cascade (no interrupt / no draft
//! clear / no BackToBoard fallback) — so no new backend/RPC work
//! exists here; this module only resolves the button index against the
//! ACTIVE surface's own `zone_c` slice.

use codelet_rpc_types::SessionStatus;

use crate::components::exit_confirmation_dialog::{
    ExitConfirmationDialog, EXIT_CONFIRMATION_DIALOG_ID,
};
use crate::components::menu_bar::items::MenuAction;
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
            // The agent bar's `[ New Agent ]` / `[ Close Agent ]` —
            // resolve through `AGENT_ZONE_C`'s registry `MenuAction`
            // (payload-free: BUG-199 R1 drops the R8 current-session
            // substitution — the button starts a NEW agent, and
            // `Close Agent` ALWAYS mounts the 'Exit Session?' dialog,
            // BUG-204 — never an Esc cascade).
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
                // BUG-204 (superseding BUG-199 R2): 'Close Agent' ALWAYS
                // mounts the 'Exit Session?' ExitConfirmationDialog
                // (RPC-098 L7) when a session is open — the button is the
                // close-action confirmation, NOT the Esc gesture. The
                // cascade's L4/L5 interrupt branch and the L6
                // draft-clear branch belong to the physical Esc key
                // (BUG-204 R5: the `AgentEscPressed` cascade is
                // unchanged for Esc). The dialog's `is_busy` flag is
                // computed from the current session's status; the
                // options commit through the existing `AgentExitChoice`
                // bus action (Close Session / Cancel — BUG-205 removed
                // the Detach option).
                // No current session → silent no-op (BUG-204 R3: the
                // dialog is meaningless without a session to exit —
                // NOT the Esc cascade's BackToBoard fallback).
                MenuAction::CloseAgent => {
                    let Some(session) = self.agent_view_store.current_session().cloned() else {
                        return;
                    };
                    // No-double-push: skip if a dialog is already on the
                    // compositor (the RPC-098 L7 parity guard).
                    if self.compositor.contains(EXIT_CONFIRMATION_DIALOG_ID) {
                        return;
                    }
                    let is_busy = matches!(
                        self.agent_view_store.session_status_for(&session).copied(),
                        Some(SessionStatus::Running) | Some(SessionStatus::Compacting),
                    );
                    let dialog =
                        ExitConfirmationDialog::new(is_busy).with_action_tx(self.action_tx.clone());
                    self.compositor.push(Box::new(dialog));
                }
                _ => {}
            }
            return;
        }
        // The board's single `New Agent` button: the EXACT
        // `.`-key semantics (BUG-203: ALWAYS start a NEW agent —
        // the CreateSessionDialog via the shared RPC-060 helper,
        // BUG-199 agent-bar parity). The MENU-009 R8 substitution
        // (the selected work unit's attached session, else resume
        // the first open session) is REMOVED: jumping into /
        // resuming a session is the Shift+Right CYCLE gesture's
        // `OpenAgentView` job, never the 'New Agent' gesture's.
        // The activation clears the bar's ring focus (the
        // `execute_dropdown_row` parity — executing is a
        // 'leave the bar' gesture).
        let Some(button) = crate::views::board::menu_snapshot::BOARD_ZONE_C.get(index) else {
            return;
        };
        self.board_store.dismiss_menu();
        // Resolve through the registry mapping (payload-free:
        // `NewAgent` → `OpenCreateSessionDialog { preselect: None }`).
        self.dispatch(button.action.to_action());
    }
}
