//! WT-018 — the Discard half of /merge-worktree: before a worktree is
//! discarded, the daemon names the cards its session created, moved or
//! edited on the shared board.
//!
//! Feature: spec/features/discarding-a-worktree-names-the-cards-its-session-changed.feature
//!
//! Flow:
//!   1. `handle_discard_confirmed` (in the parent module, the merge
//!      dialog's Discard) pops the merge dialog and asks
//!      `backend.session_card_changes(session_id)`.
//!   2. `handle_session_card_changes_loaded`: a session that changed no
//!      card discards at once, as before; otherwise the
//!      `DiscardCardsDialog` lists the cards and nothing changes until the
//!      user picks one of its choices.
//!   3. `handle_discard_session_cards`: discards the worktree
//!      (`backend.discard_session_worktree`), then, for "Discard and move
//!      back", moves the session's cards back
//!      (`backend.move_session_cards_back`) and names every card in a
//!      notice: moved back to its status, or not moved back with the
//!      gate's error.

use tokio::task::JoinHandle;

use codelet_rpc_types::{CardMoveBack, CardMoveBackOutcome, SessionCardChange, SessionId};

use crate::components::Action;
use crate::views::agent::merge_confirm_dialog::discard_cards::{
    DiscardCardsDialog, DISCARD_CARDS_DIALOG_ID,
};

use super::super::state::App;

impl App {
    /// WT-018: no card changed → discard as before; otherwise open the
    /// confirmation listing the cards.
    pub(crate) fn handle_session_card_changes_loaded(
        &mut self,
        session_id: SessionId,
        changes: Vec<SessionCardChange>,
    ) {
        if changes.is_empty() {
            self.handle_discard_session_cards(session_id, false);
            return;
        }
        let _ = self.compositor.remove(DISCARD_CARDS_DIALOG_ID);
        let dialog =
            DiscardCardsDialog::new(session_id, changes).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// WT-018: discard the worktree; with `move_back`, then move the
    /// session's cards back to their status before the session.
    pub(crate) fn handle_discard_session_cards(&mut self, session_id: SessionId, move_back: bool) {
        let _ = self.compositor.remove(DISCARD_CARDS_DIALOG_ID);
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let notice = |text: String| {
                let _ = action_tx.send(Action::EmitSessionNotice(session_id.clone(), text));
            };
            if let Err(e) = backend.discard_session_worktree(session_id.clone()).await {
                notice(format!("[error] /merge-worktree discard: {e}"));
                return;
            }
            notice("[discard] worktree discarded".to_string());
            if !move_back {
                return;
            }
            match backend.move_session_cards_back(session_id.clone()).await {
                Ok(moves) => moves.iter().map(move_back_notice).for_each(notice),
                Err(e) => notice(format!(
                    "[error] /merge-worktree discard: the cards were not moved back: {e}"
                )),
            }
        });
        self.pending_tasks.push(handle);
    }

    /// WT-018: Cancel / Esc in the discard confirmation.
    pub(crate) fn handle_cancel_discard_cards(&mut self) {
        let _ = self.compositor.remove(DISCARD_CARDS_DIALOG_ID);
    }
}

/// The notice naming one card the discard tried to move back.
fn move_back_notice(card: &CardMoveBack) -> String {
    match &card.outcome {
        CardMoveBackOutcome::MovedBack { status } => {
            format!("[discard] {} moved back to {status}", card.work_unit_id)
        }
        CardMoveBackOutcome::Failed { error } => {
            format!(
                "[discard] {} was not moved back: {error}",
                card.work_unit_id
            )
        }
    }
}
