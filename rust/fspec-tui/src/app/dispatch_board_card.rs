//! BOARD-024 — App routing for the board card chooser and canonical details.
//!
//! Feature: spec/features/board-card-enter-chooser-and-complete-details.feature
//!
//! The single `try_dispatch_board_card` capability branch: opens the modal
//! chooser for the captured card/origin, performs canonical detail reads
//! through the shared `FspecBackend` (embedded or WebSocket) on a pending
//! task tagged with the dialog's request generation, records the daemon
//! workspace from successful envelopes, and routes `OpenAttachment` to the
//! details-aware attachment flow while the card dialog is open. Agent
//! entry stays with the existing `EnterWorkUnit` / `MuxEnterWorkUnit` arms.

use std::path::PathBuf;
use std::sync::Arc;

use crate::components::board_card::{BoardCardDialog, BOARD_CARD_DIALOG_ID};
use crate::components::Action;

use super::state::App;

impl App {
    fn open_board_card_chooser(
        &mut self,
        id: &str,
        origin: crate::components::board_card::BoardCardOrigin,
    ) {
        if self.compositor.contains(BOARD_CARD_DIALOG_ID) {
            return;
        }
        let dialog =
            BoardCardDialog::new(id.to_owned(), origin).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    fn request_board_card_details(&mut self, generation: u64, id: &str) {
        let backend = Arc::clone(&self.backend);
        let tx = self.action_tx.clone();
        let id = id.to_owned();
        if tokio::runtime::Handle::try_current().is_err() {
            let action = Action::BoardCardDetailsLoaded {
                request_generation: generation,
                work_unit_id: id,
                result: Err("details unavailable: no async runtime".to_owned()),
            };
            if tx.send(action).is_err() {
                tracing::warn!("board card details: action bus closed");
            }
            return;
        }
        // RPC-450: details come from the workspace the board is showing.
        let workspace = self
            .board_store
            .selected_workspace()
            .map(|entry| entry.path.clone());
        let handle = tokio::spawn(async move {
            let result = match workspace {
                Some(workspace) => backend
                    .get_work_unit_details(workspace, id.clone())
                    .await
                    .map_err(|error| error.to_string()),
                None => Err(format!(
                    "details unavailable for {id}: the board's workspace is not known yet"
                )),
            };
            let action = Action::BoardCardDetailsLoaded {
                request_generation: generation,
                work_unit_id: id,
                result,
            };
            if tx.send(action).is_err() {
                tracing::warn!("board card details: action bus closed");
            }
        });
        self.pending_tasks.push(handle);
    }

    fn record_board_card_workspace(&mut self, payload: &str) {
        let workspace = serde_json::from_str::<serde_json::Value>(payload)
            .ok()
            .and_then(|envelope| envelope["workspace"].as_str().map(PathBuf::from));
        if let Some(workspace) = workspace {
            self.board_card.workspace = Some(workspace);
        }
    }

    /// Route BOARD-024 actions. Returns true when the action was claimed.
    pub(crate) fn try_dispatch_board_card(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenBoardCardChooser {
                work_unit_id,
                origin,
            } => self.open_board_card_chooser(work_unit_id, *origin),
            Action::RequestBoardCardDetails {
                request_generation,
                work_unit_id,
            } => self.request_board_card_details(*request_generation, work_unit_id),
            Action::BoardCardDetailsLoaded {
                result: Ok(payload),
                ..
            } => self.record_board_card_workspace(payload),
            Action::BoardCardDetailsLoaded { .. } | Action::BoardCardAttachmentFailed { .. } => {}
            Action::OpenAttachment(path) if self.compositor.contains(BOARD_CARD_DIALOG_ID) => {
                self.open_board_card_attachment(path);
            }
            _ => return false,
        }
        true
    }
}
