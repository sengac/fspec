//! MENU-002 — the board's 2-zone menu bar + ring keyboard arms.
//!
//! Feature: spec/features/board-surface-header-row-replaced-by-the-2-zone-bar-column-menu-chip-continuous-ring-keys-wheel-mouse.feature
//! Card: MENU-002 (R2, R3, R4, R6, R9).
//!
//! Extracted from `BoardView::handle_event` (board.rs) so the
//! orchestrator stays under the 300-LoC ceiling. Gating, in order:
//!
//! 1. **Open dropdown (R6 true-modal)** — ONLY the ring keys move:
//!    Up/Down (j/k) walk the cursor (wrapping), Enter executes the row,
//!    Esc closes the dropdown (NO app-level exit cascade), Left/Right
//!    walk the ring (the open item follows the focus — GUI parity);
//!    every other key is swallowed.
//! 2. **Bar focus (item or chip)** — Up/Down drop focus back into the
//!    focused column (R3); Enter opens the item's dropdown / activates
//!    the chip (R4/R5); Left/Right walk the ring (R2).
//! 3. **No bar focus** — Left/Right walk the ring (R2: columns are the
//!    ring's front half; this supersedes `FocusPrev/NextColumn`),
//!    Up/Down keep their column-selection meaning, Enter keeps
//!    `EnterWorkUnit`.
//!
//! The store owns the ring math (`BoardStore::menu_move` — R8); the
//! view only emits the Actions.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::components::{Action, EventResult};
use crate::store::BoardStore;

use super::BoardView;

/// True iff a modifier-free ring/confirm key is pressed.
fn plain(code: KeyCode, key: &KeyEvent) -> bool {
    code == key.code && key.modifiers == KeyModifiers::NONE
}

/// Route the board's menu-bar keys against `store`. `Some(result)` when
/// a menu arm claimed the event (the caller returns it verbatim),
/// `None` when the event is a non-menu key on an unfocused bar (the
/// caller keeps the navigation-key cascade running).
pub(super) fn handle_menu_keys(
    view: &BoardView,
    key: &KeyEvent,
    store: &BoardStore,
) -> Option<EventResult> {
    // ── R6: an open dropdown is true-modal ──────────────────────────────
    if let Some((category, cursor)) = store.open_menu() {
        if plain(KeyCode::Up, key)
            || plain(KeyCode::Char('k'), key)
            || plain(KeyCode::Down, key)
            || plain(KeyCode::Char('j'), key)
        {
            let delta = if plain(KeyCode::Down, key) || plain(KeyCode::Char('j'), key) {
                1
            } else {
                -1
            };
            view.emit(Action::MenuDropdownCursor(delta));
            return Some(EventResult::consumed());
        }
        if plain(KeyCode::Enter, key) {
            view.emit(Action::MenuExecuteItem {
                category,
                row: cursor,
            });
            return Some(EventResult::consumed());
        }
        if plain(KeyCode::Esc, key) {
            // R9: close the dropdown — consumed, so the app-level exit
            // cascade (RPC-102) must NOT run.
            view.emit(Action::MenuCloseDropdown);
            return Some(EventResult::consumed());
        }
        if plain(KeyCode::Left, key) || plain(KeyCode::Char('h'), key) {
            view.emit(Action::MenuMove(-1));
            return Some(EventResult::consumed());
        }
        if plain(KeyCode::Right, key) || plain(KeyCode::Char('l'), key) {
            view.emit(Action::MenuMove(1));
            return Some(EventResult::consumed());
        }
        // R6: every other key is swallowed while the dropdown is open.
        return Some(EventResult::ignored());
    }

    // ── R3/R2: a Zone A item or Zone B chip has the bar focus ───────────
    if let Some(focus) = store.menu_focus() {
        match focus {
            crate::components::menu_bar::MenuFocus::Item(category) => {
                if plain(KeyCode::Up, key)
                    || plain(KeyCode::Char('k'), key)
                    || plain(KeyCode::Down, key)
                    || plain(KeyCode::Char('j'), key)
                {
                    view.emit(Action::MenuFocusToColumns);
                    return Some(EventResult::consumed());
                }
                if plain(KeyCode::Enter, key) {
                    // R4: closed → open at row 0 (a second open-tick on
                    // the open item closes it — dispatch routes through
                    // `open_or_close_menu`). While the dropdown is OPEN
                    // Enter is claimed by the modal gate above (it
                    // executes the row), so this arm only sees a
                    // CLOSED dropdown and re-arms it at row 0.
                    view.emit(Action::MenuOpenDropdown(category));
                    return Some(EventResult::consumed());
                }
                if plain(KeyCode::Left, key) || plain(KeyCode::Char('h'), key) {
                    view.emit(Action::MenuMove(-1));
                    return Some(EventResult::consumed());
                }
                if plain(KeyCode::Right, key) || plain(KeyCode::Char('l'), key) {
                    view.emit(Action::MenuMove(1));
                    return Some(EventResult::consumed());
                }
                return Some(EventResult::ignored());
            }
            crate::components::menu_bar::MenuFocus::ZoneB(index) => {
                if plain(KeyCode::Up, key)
                    || plain(KeyCode::Char('k'), key)
                    || plain(KeyCode::Down, key)
                    || plain(KeyCode::Char('j'), key)
                {
                    view.emit(Action::MenuFocusToColumns);
                    return Some(EventResult::consumed());
                }
                if plain(KeyCode::Enter, key) {
                    // R5: Enter on a chip jumps to that session's
                    // Agent view.
                    view.emit(Action::MenuChipActivate(index));
                    return Some(EventResult::consumed());
                }
                if plain(KeyCode::Left, key) || plain(KeyCode::Char('h'), key) {
                    view.emit(Action::MenuMove(-1));
                    return Some(EventResult::consumed());
                }
                if plain(KeyCode::Right, key) || plain(KeyCode::Char('l'), key) {
                    view.emit(Action::MenuMove(1));
                    return Some(EventResult::consumed());
                }
                return Some(EventResult::ignored());
            }
        }
    }

    // ── R2: no bar focus — Left/Right walk the ring from the column ────
    if plain(KeyCode::Left, key) || plain(KeyCode::Char('h'), key) {
        // COPY-009: a column walk changes the focused column's strip
        // content — clear any active strip selection (old parity).
        view.clear_details_selection();
        view.emit(Action::MenuMove(-1));
        return Some(EventResult::consumed());
    }
    if plain(KeyCode::Right, key) || plain(KeyCode::Char('l'), key) {
        view.clear_details_selection();
        view.emit(Action::MenuMove(1));
        return Some(EventResult::consumed());
    }
    None
}
