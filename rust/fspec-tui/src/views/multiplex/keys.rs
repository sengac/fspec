//! MUX-001/MUX-002 — mux keyboard routing: focus cycling and forwarding
//! to the focused pane only.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! The "trap": unfocused panes receive NO keyboard events.
//! Shift+Left / Shift+Right drive the mux focus + agent window (MUX-002:
//! rotation, stop-at-edges, new-agent prompt at the right edge). These
//! are the ONLY mux keybindings (2026-08-26 user directive): the 'm'
//! toggle, Tab pane/divider cycling, and keyboard divider resize were
//! removed — Tab is reserved for the agent view's turn-select mode and
//! the divider is mouse-drag-resizable only.
//!
//! MUX-002: in mux mode the App intercepts Shift+Left/Right BEFORE the
//! Navigator (`App::handle_event`, events.rs) so the new-agent dialog
//! opens synchronously; this classifier's FocusPrev/FocusNext arms are
//! the Navigator-level fallback.
//!
//! This module classifies keys into [`KeyDecision`]s; the Navigator
//! executes them (keeps the layout borrow and the pane-handler borrow
//! disjoint).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::MultiplexLayout;

/// What the mux layer wants the Navigator to do with a key.
pub enum KeyDecision {
    /// Shift+Left: rotate the agent window backward OR move focus one
    /// pane left (stop at the first pane — MUX-002, no wrap).
    FocusPrev,
    /// Shift+Right: rotate the agent window forward, move focus one
    /// pane right, or prompt for a new agent at the right edge
    /// (MUX-002 — no wrap).
    FocusNext,
    /// Forward the key to the focused pane only.
    Forward,
}

/// Classify a key event against the current mux layout.
pub fn classify_key(layout: &MultiplexLayout, key: &KeyEvent) -> KeyDecision {
    if !layout.config.enabled {
        return KeyDecision::Forward;
    }

    // Shift+Left / Shift+Right: cycle pane focus (R3, panes only).
    if key.modifiers.contains(KeyModifiers::SHIFT)
        && matches!(key.code, KeyCode::Left | KeyCode::Right)
    {
        return match key.code {
            KeyCode::Left => KeyDecision::FocusPrev,
            _ => KeyDecision::FocusNext,
        };
    }

    KeyDecision::Forward
}

/// Forward an event to the pane of `kind` (keyboard isolation helper).
///
/// BUG-183: the lazy panes' `Close` outcome (Esc on the focused Files /
/// Checkpoints pane) is translated onto the action bus — the same
/// `CloseChangedFilesView` / `CloseCheckpointsView` the single-view
/// Navigator path emits — so the App dispatch removes the pane kind
/// from the LIVE mux grid (transient, R1) instead of swallowing the key
/// (pre-fix every non-Ignored outcome was a plain `consumed()`, making
/// Esc a dead key in the lazy panes). `Emit` actions route straight to
/// the bus as well (diff reloads etc. — the view is the sole emitter of
/// those in mux mode too).
#[allow(clippy::too_many_arguments)]
pub fn forward_to_pane(
    event: &crossterm::event::Event,
    board: &crate::store::BoardStore,
    board_view: &crate::views::BoardView,
    agent_view: &mut crate::views::AgentView,
    changed_files: &mut crate::views::ChangedFilesView,
    checkpoints: &mut crate::views::CheckpointsView,
    kind: super::MuxPaneKind,
    action_tx: Option<&tokio::sync::mpsc::UnboundedSender<crate::components::Action>>,
) -> crate::components::EventResult {
    match kind {
        super::MuxPaneKind::Board => board_view.handle_event(event, board),
        super::MuxPaneKind::Agent => agent_view.handle_event(event),
        super::MuxPaneKind::ChangedFiles => match changed_files.handle_event(event) {
            crate::views::ChangedFilesEvent::Ignored => crate::components::EventResult::ignored(),
            crate::views::ChangedFilesEvent::Consumed => crate::components::EventResult::consumed(),
            crate::views::ChangedFilesEvent::Close => {
                if let Some(tx) = action_tx {
                    let _ = tx.send(crate::components::Action::CloseChangedFilesView);
                }
                crate::components::EventResult::consumed()
            }
            crate::views::ChangedFilesEvent::Emit(action) => {
                if let Some(tx) = action_tx {
                    let _ = tx.send(action);
                }
                crate::components::EventResult::consumed()
            }
        },
        super::MuxPaneKind::Checkpoints => match checkpoints.handle_event(event) {
            crate::views::CheckpointsEvent::Ignored => crate::components::EventResult::ignored(),
            crate::views::CheckpointsEvent::Consumed => crate::components::EventResult::consumed(),
            crate::views::CheckpointsEvent::Close => {
                if let Some(tx) = action_tx {
                    let _ = tx.send(crate::components::Action::CloseCheckpointsView);
                }
                crate::components::EventResult::consumed()
            }
            crate::views::CheckpointsEvent::Emit(action) => {
                if let Some(tx) = action_tx {
                    let _ = tx.send(action);
                }
                crate::components::EventResult::consumed()
            }
        },
    }
}
