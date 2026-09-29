//! BOARD-023 — single source of truth for the board's top-row shortcuts.
//!
//! Feature: spec/features/board-actions-popup-dialog-triggered-by-u-key.feature
//! Card: BOARD-023 (R11).
//!
//! This table is consumed by BOTH the new `BoardKeybindingDialog` row
//! builder AND the board key-arm documentation (views/board/keys.rs),
//! so the dialog can never drift from the actual key handling again.
//!
//! The `chord_hint` constant is what the header row now paints (R10) —
//! a single short hint that replaces the old six-action chord.

use codelet_rpc_types::SessionId;
use crossterm::event::KeyCode;

use crate::components::Action;

/// One board shortcut entry: the key display, a label, a one-line
/// description, and the `Action` the bare key emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardShortcut {
    /// The key glyph as painted in the dialog label (e.g. "C", ".", "/").
    pub key: &'static str,
    /// The label shown next to the key (e.g. "Checkpoints").
    pub label: &'static str,
    /// The one-line description shown in the dialog body (dimmed).
    pub description: &'static str,
    /// The `Action` the bare board key emits.
    pub action: BoardShortcutAction,
    /// The crossterm `KeyCode` of the bare key (documentary — the
    /// actual key arms in `views/board/keys.rs` remain authoritative;
    /// this field exists so the table is self-describing and testable).
    pub key_code: KeyCode,
}

/// Discriminates the action the bare board key would emit. `OpenAgentView`
/// carries the session target snapshotted at dialog-open time (R5) — the
/// dialog is built with that snapshot, so the table stays `Copy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardShortcutAction {
    OpenCheckpointsView,
    OpenChangedFilesView,
    OpenFoundation,
    /// Open/start an agent for the focused work unit. `Option<SessionId>`
    /// mirrors the `BoardView::selected_session` snapshot taken at open.
    OpenAgentView(Option<SessionId>),
    OpenWorkUnitSearch,
    OpenMuxConfigDialog,
}

impl BoardShortcutAction {
    /// Resolve to the concrete [`Action`] to emit on Enter (when `Some`).
    pub fn to_action(self) -> Action {
        match self {
            BoardShortcutAction::OpenCheckpointsView => Action::OpenCheckpointsView,
            BoardShortcutAction::OpenChangedFilesView => Action::OpenChangedFilesView,
            BoardShortcutAction::OpenFoundation => Action::OpenFoundation,
            BoardShortcutAction::OpenAgentView(target) => Action::OpenAgentView(target),
            BoardShortcutAction::OpenWorkUnitSearch => Action::OpenWorkUnitSearch,
            BoardShortcutAction::OpenMuxConfigDialog => Action::OpenMuxConfigDialog,
        }
    }
}

/// The R3 row order — exactly the six bare board shortcuts. The dialog
/// appends `? Help` and `Esc Exit` after these (they emit the new
/// `OpenBoardHelp` / `OpenBoardExitConfirmation` actions — carried as
/// constants in the dialog rather than in `BoardShortcutAction`
/// because they are not "bare board keys" the way C/F/D/./ /M are).
/// There is NO trigger entry: the 'u' trigger is advertised by the
/// header hint (CHORD_HINT) and the board help lines instead.
pub const SHORTCUTS: [BoardShortcut; 6] = [
    BoardShortcut {
        key: "C",
        label: "Checkpoints",
        description: "Open the checkpoints view",
        action: BoardShortcutAction::OpenCheckpointsView,
        key_code: KeyCode::Char('c'),
    },
    BoardShortcut {
        key: "F",
        label: "Changed Files",
        description: "Open the changed-files view",
        action: BoardShortcutAction::OpenChangedFilesView,
        key_code: KeyCode::Char('f'),
    },
    BoardShortcut {
        key: "D",
        label: "FOUNDATION.md",
        description: "Open FOUNDATION.md in the browser",
        action: BoardShortcutAction::OpenFoundation,
        key_code: KeyCode::Char('d'),
    },
    BoardShortcut {
        key: ".",
        label: "New Agent",
        description: "Open/start an agent for the focused unit",
        action: BoardShortcutAction::OpenAgentView(None),
        key_code: KeyCode::Char('.'),
    },
    BoardShortcut {
        key: "/",
        label: "Search",
        description: "Search work units",
        action: BoardShortcutAction::OpenWorkUnitSearch,
        key_code: KeyCode::Char('/'),
    },
    BoardShortcut {
        key: "M",
        label: "Mux",
        description: "Open the mux layout config",
        action: BoardShortcutAction::OpenMuxConfigDialog,
        key_code: KeyCode::Char('m'),
    },
];

/// The R10 short hint painted on row 3 of the board header — replaces
/// the old six-action chord. Single plain primary-fg span (no styling
/// change from the pre-BOARD-023 chord row).
pub const CHORD_HINT: &str = "u Actions";
