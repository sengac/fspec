//! BOARD-024 — board card Enter chooser and complete canonical details.
//!
//! Features:
//!   - spec/features/board-card-enter-chooser-and-complete-details.feature
//!   - spec/features/board-card-details-loading-generations.feature
//!   - spec/features/board-card-details-scrolling-and-links.feature
//!   - spec/features/board-card-details-attachments.feature
//!   - spec/features/board-card-details-planning-sections.feature
//!
//! One Foreground modal owns the captured card id and its Board/Mux
//! origin. Chooser mode offers `Open in agent` (delegating to the existing
//! intent-safe `EnterWorkUnit` / `MuxEnterWorkUnit` entry) and
//! `Show full details`. Details mode keeps a back-history of canonical
//! pages; every canonical read carries a request generation so late,
//! stale or dismissed completions can never replace the current view.

mod chooser;
pub mod document;
mod document_storm;
mod events;
mod frame;
mod labels;
mod page;
pub mod sanitize;
mod wrap;

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc::UnboundedSender;

use super::Action;
use page::Page;

/// Stable compositor id of the card dialog.
pub const BOARD_CARD_DIALOG_ID: &str = "board-card-dialog";

/// Monotonic request generations shared by every card dialog instance so a
/// reopened dialog for the same card never reuses an earlier generation.
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Where the card Enter came from; agent entry preserves the origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardCardOrigin {
    /// The single Board view.
    Board,
    /// The focused board pane of the mux grid.
    Mux,
}

enum Mode {
    Chooser,
    Details,
}

struct Pending {
    generation: u64,
    id: String,
}

/// Visible one-line status inside the details frame.
pub(crate) enum Status {
    Loading(String),
    Error(String),
}

/// The modal card chooser + details dialog.
pub struct BoardCardDialog {
    card_id: String,
    origin: BoardCardOrigin,
    chooser_selected: usize,
    mode: Mode,
    pages: Vec<Page>,
    pending: Option<Pending>,
    status: Option<Status>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl BoardCardDialog {
    /// Open the chooser for the captured card and origin.
    pub fn new(card_id: String, origin: BoardCardOrigin) -> Self {
        Self {
            card_id,
            origin,
            chooser_selected: 0,
            mode: Mode::Chooser,
            pages: Vec::new(),
            pending: None,
            status: None,
            action_tx: None,
        }
    }

    /// Attach the App's action bus.
    pub fn with_action_tx(mut self, tx: UnboundedSender<Action>) -> Self {
        self.action_tx = Some(tx);
        self
    }

    /// The captured card id.
    pub fn card_id(&self) -> &str {
        &self.card_id
    }

    /// The captured origin.
    pub fn origin(&self) -> BoardCardOrigin {
        self.origin
    }

    fn emit(&self, action: Action) {
        if let Some(tx) = self.action_tx.as_ref() {
            if tx.send(action).is_err() {
                tracing::warn!("board card dialog: action bus closed");
            }
        }
    }

    /// Request canonical details for `id` with a fresh generation.
    fn request(&mut self, id: String) {
        let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        self.status = Some(Status::Loading(id.clone()));
        self.pending = Some(Pending {
            generation,
            id: id.clone(),
        });
        self.emit(Action::RequestBoardCardDetails {
            request_generation: generation,
            work_unit_id: id,
        });
    }

    /// Accept a completion only when it matches the current request.
    fn complete(&mut self, generation: u64, id: &str, result: &Result<String, String>) {
        let current = self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.generation == generation && pending.id == id);
        if !current || !matches!(self.mode, Mode::Details) {
            return;
        }
        self.pending = None;
        let page = result
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|payload| Page::from_envelope(id, payload));
        match page {
            Ok(page) => {
                self.pages.push(page);
                self.status = None;
            }
            Err(message) => {
                self.status = Some(Status::Error(format!("Error: {id}: {message}")));
            }
        }
    }

    fn back(&mut self) {
        self.pending = None;
        self.status = None;
        if self.pages.len() > 1 {
            self.pages.pop();
        } else {
            self.pages.clear();
            self.mode = Mode::Chooser;
        }
    }
}
