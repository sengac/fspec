//! MENU-004 — the mux bar's keyboard classification (R-KEYS, R-ZONEB).
//!
//! Feature: spec/features/mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick.feature
//!
//! The classification is a PURE read over the layout + focused panes: it
//! returns a [`MuxBarKeyOutcome`] the Navigator applies. The bar's state
//! (ring focus, open dropdown) lives on `MultiplexLayout` (RED CARD 1);
//! the emitted `Action`s are mutated by `App::dispatch_menu`'s Mux branch
//! (the single-mutation-surface pattern — the board `MenuMove`/`store`
//! parity, with the ring walk computed from a per-frame snapshot in the
//! App).
//!
//! Gating, in order (mirrors the board/agent surfaces):
//! 1. **Open dropdown (true-modal)** — Up/Down move the cursor,
//!    Left/Right walk the ring + re-anchor, Enter executes + closes,
//!    Esc closes; every other key (including characters) is swallowed.
//! 2. **Bar focus (item or Zone B cell, closed)** — Enter opens the
//!    item's dropdown / activates the cell (chip → session, view label →
//!    pane); Up/Down drop back into the focused pane input (the draft is
//!    preserved); Left/Right walk the ring; a bare character key dismisses
//!    the bar AND is typed into the focused pane input on the same event
//!    (the Navigator forwards it after clearing).
//! 3. **No bar focus** — Board pane: Right off the last column enters the
//!    bar at the first item (the column⇄menu⇄chip ring continues); Agent
//!    pane: bare Left on an EMPTY input enters at the first item
//!    (MENU-003 parity). Files/Checkpoints panes do NOT feed the bar
//!    (RED CARD 2). Every other key is forwarded to the focused pane.
//!
//! Shift+arrows are NEVER claimed here (the App intercepts them before the
//! Navigator — pane focus cycling, R-KEYS: "Shift+arrows never enter or
//! move the bar").

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::components::Action;
use crate::store::BoardStore;

use super::{MultiplexLayout, MuxPaneKind};

/// True iff a modifier-free key is pressed.
fn plain(code: KeyCode, key: &KeyEvent) -> bool {
    code == key.code && key.modifiers == KeyModifiers::NONE
}

/// True iff a bare (modifier-free) printable character is pressed.
fn bare_char(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char(_)) && key.modifiers == KeyModifiers::NONE
}

/// What the Navigator must do with a mux bar key.
pub(crate) enum MuxBarKeyOutcome {
    /// The bar claimed the key and the Navigator emits this `Action`
    /// (the App's Mux branch mutates the layout's bar state). Boxed —
    /// see `BlocklistEvent::Emit` for the `large_enum_variant` rationale.
    Emit(Box<Action>),
    /// The bar claimed the key and swallowed it (true-modal dropdown, or
    /// an unrecognized key while bar-focused) — no action, consumed.
    Swallow,
    /// A bare character key with the bar focused: clear the bar focus AND
    /// forward the SAME key to the focused pane's input (it types there).
    DismissAndForward,
    /// The bar does not own this key — forward it to the focused pane.
    Forward,
}

/// Box the action for the `Emit` variant (the `large_enum_variant`
/// shape — `Action` is ≥ 216 bytes, see `BlocklistEvent::Emit`).
fn emit(action: Action) -> MuxBarKeyOutcome {
    MuxBarKeyOutcome::Emit(Box::new(action))
}

/// A resolved Zone B cell target (Enter on a cell, R-ZONEB).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ZoneBTarget {
    /// A chip — activate session `i` (the global painted-chip index).
    Chip(usize),
    /// A pane view label — focus effective pane `i`.
    Pane(usize),
    /// The cell index no longer addresses a cell (stale focus).
    None,
}

impl MultiplexLayout {
    /// Resolve a Zone B display-cell index (the painted order: the
    /// non-agent effective panes as view labels, then the global chips)
    /// to its target. The ring addresses PAINTED cells, so this reads
    /// the SAME derivation `menu_snapshot::build_snapshot` uses.
    pub(crate) fn zone_b_target(&self, cell: usize) -> ZoneBTarget {
        let labels = self.view_label_count();
        if cell < labels {
            // The `cell`-th non-agent effective pane.
            let pane = self
                .effective_panes()
                .iter()
                .take_while(|_| true)
                .enumerate()
                .filter(|(_, kind)| **kind != MuxPaneKind::Agent)
                .nth(cell)
                .map(|(i, _)| i)
                .unwrap_or(0);
            ZoneBTarget::Pane(pane)
        } else {
            let chip = cell - labels;
            if chip < self.menu_chips {
                ZoneBTarget::Chip(chip)
            } else {
                ZoneBTarget::None
            }
        }
    }
}

/// The focused pane's kind (`None` when the focus index is stale).
fn focused_kind(layout: &MultiplexLayout) -> Option<MuxPaneKind> {
    layout.effective_panes().get(layout.focus()).copied()
}

/// Classify a key against the mux bar. Pure read — the Navigator applies
/// the outcome (emitting / dismissing / forwarding).
pub(crate) fn classify_bar_key(
    layout: &MultiplexLayout,
    board_store: &BoardStore,
    agent_view: &crate::views::AgentView,
    key: &KeyEvent,
) -> MuxBarKeyOutcome {
    // Shift+arrows are NEVER claimed (pane focus cycling — the App
    // intercepts them before the Navigator; R-KEYS: "never enter or move
    // the bar").
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        return MuxBarKeyOutcome::Forward;
    }

    // ── 1. an open dropdown is true-modal ───────────────────────────────
    if let Some((category, cursor)) = layout.open_menu() {
        match key.code {
            KeyCode::Up if plain(KeyCode::Up, key) => {
                return emit(Action::MenuDropdownCursor(-1));
            }
            KeyCode::Down if plain(KeyCode::Down, key) => {
                return emit(Action::MenuDropdownCursor(1));
            }
            KeyCode::Enter if plain(KeyCode::Enter, key) => {
                return emit(Action::MenuExecuteItem {
                    category,
                    row: cursor,
                });
            }
            KeyCode::Esc if plain(KeyCode::Esc, key) => {
                return emit(Action::MenuCloseDropdown);
            }
            KeyCode::Left if plain(KeyCode::Left, key) => {
                return emit(Action::MenuMove(-1));
            }
            KeyCode::Right if plain(KeyCode::Right, key) => {
                return emit(Action::MenuMove(1));
            }
            // R-KEYS: every other key (including characters) is swallowed.
            _ => return MuxBarKeyOutcome::Swallow,
        }
    }

    // ── 2. a bar item or Zone B cell has the focus (dropdown closed) ─────
    if let Some(focus) = layout.menu_focus() {
        match focus {
            crate::components::menu_bar::MenuFocus::Item(category) => {
                if plain(KeyCode::Up, key) || plain(KeyCode::Down, key) {
                    // Drop back into the focused pane input (draft kept).
                    return emit(Action::MenuFocusToColumns);
                }
                if plain(KeyCode::Enter, key) {
                    return emit(Action::MenuOpenDropdown(category));
                }
                if plain(KeyCode::Left, key) {
                    return emit(Action::MenuMove(-1));
                }
                if plain(KeyCode::Right, key) {
                    return emit(Action::MenuMove(1));
                }
                if bare_char(key) {
                    return MuxBarKeyOutcome::DismissAndForward;
                }
                return MuxBarKeyOutcome::Swallow;
            }
            crate::components::menu_bar::MenuFocus::ZoneB(cell) => {
                if plain(KeyCode::Up, key) || plain(KeyCode::Down, key) {
                    return emit(Action::MenuFocusToColumns);
                }
                if plain(KeyCode::Enter, key) {
                    // Resolve the cell: a chip → activate the session; a
                    // view label → focus that pane (R-ZONEB).
                    return match layout.zone_b_target(cell) {
                        ZoneBTarget::Chip(i) => emit(Action::MenuChipActivate(i)),
                        ZoneBTarget::Pane(pane) => emit(Action::MenuFocusPane(pane)),
                        ZoneBTarget::None => MuxBarKeyOutcome::Swallow,
                    };
                }
                if plain(KeyCode::Left, key) {
                    return emit(Action::MenuMove(-1));
                }
                if plain(KeyCode::Right, key) {
                    return emit(Action::MenuMove(1));
                }
                if bare_char(key) {
                    return MuxBarKeyOutcome::DismissAndForward;
                }
                return MuxBarKeyOutcome::Swallow;
            }
            // MENU-009 Q1: the mux bar paints NO Zone C (empty `zone_c`
            // slice) — a `MenuFocus::ZoneC` is unreachable here, but the
            // match must stay exhaustive. Swallow it defensively (a
            // stale focus should not leak into the pane input).
            crate::components::menu_bar::MenuFocus::ZoneC(_) => {
                return MuxBarKeyOutcome::Swallow;
            }
        }
    }

    // ── 3. no bar focus — entry from the focused pane's edge rule ────────
    match focused_kind(layout) {
        Some(MuxPaneKind::Board) => {
            // The column⇄menu⇄chip ring continues into the mux bar: Right
            // off the last column lands on the first item.
            if plain(KeyCode::Right, key)
                && board_store.focused_column_index() == crate::store::COLUMN_ORDER.len() - 1
            {
                return emit(Action::MenuMoveToItem(0));
            }
            MuxBarKeyOutcome::Forward
        }
        Some(MuxPaneKind::Agent) => {
            // MENU-003 parity: bare Left on an EMPTY input enters the bar
            // at the first item.
            if plain(KeyCode::Left, key) && agent_view.input.is_empty() {
                return emit(Action::MenuMoveToItem(0));
            }
            MuxBarKeyOutcome::Forward
        }
        // Files/Checkpoints panes do NOT feed the bar (RED CARD 2).
        _ => MuxBarKeyOutcome::Forward,
    }
}
