//! Feature: spec/features/blocklist-view-keybindings.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// -------------------------------------------------------------------
// BLOCK-009 — full-screen shell framing / chrome parity
// -------------------------------------------------------------------

// -------------------------------------------------------------------
// BLOCK-010 — keyboard parity (remove vim j/k; add PageUp/PageDown/Home/End)
// Feature: spec/features/blocklist-view-keybindings.feature
// -------------------------------------------------------------------

// Feature: spec/features/blocklist-view-keybindings.feature
#[test]
fn vim_keys_do_not_move_the_selection() {
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    // @step And the selection is at index 5
    view.selected_index = 5;
    view.adjust_scroll();
    // @step When I press the "j" key
    press(&mut view, KeyCode::Char('j'));
    // @step Then the selection stays at index 5
    assert_eq!(view.selected_index, 5);
    // @step When I press the "k" key
    press(&mut view, KeyCode::Char('k'));
    // @step Then the selection stays at index 5
    assert_eq!(view.selected_index, 5);
}

// Feature: spec/features/blocklist-view-keybindings.feature
#[test]
fn arrow_keys_move_the_selection_one_rule_at_a_time() {
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    // @step And the selection is at index 5
    view.selected_index = 5;
    view.adjust_scroll();
    // @step When I press the Down arrow key
    press(&mut view, KeyCode::Down);
    // @step Then the selection moves to index 6
    assert_eq!(view.selected_index, 6);
    // @step When I press the Up arrow key
    press(&mut view, KeyCode::Up);
    // @step Then the selection moves to index 5
    assert_eq!(view.selected_index, 5);
}

// Feature: spec/features/blocklist-view-keybindings.feature
#[test]
fn page_down_advances_the_selection_by_one_viewport() {
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    // @step And the visible window shows 8 rows
    view.set_visible_rows(8);
    // @step And the selection is at index 0
    view.selected_index = 0;
    view.adjust_scroll();
    // @step When I press the PageDown key
    press(&mut view, KeyCode::PageDown);
    // @step Then the selection moves to index 8
    assert_eq!(view.selected_index, 8);
    // @step And the visible window scrolls so the selection stays visible
    let off = view.scroll_offset();
    assert!(
        view.selected_index >= off && view.selected_index < off + 8,
        "selection {} not inside window [{}, {})",
        view.selected_index,
        off,
        off + 8
    );
}

// Feature: spec/features/blocklist-view-keybindings.feature
#[test]
fn page_down_clamps_at_the_last_rule() {
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    // @step And the visible window shows 8 rows
    view.set_visible_rows(8);
    // @step And the selection is at the last rule index 19
    view.selected_index = 19;
    view.adjust_scroll();
    // @step When I press the PageDown key
    press(&mut view, KeyCode::PageDown);
    // @step Then the selection stays at index 19
    assert_eq!(view.selected_index, 19);
}

// Feature: spec/features/blocklist-view-keybindings.feature
#[test]
fn page_up_retreats_the_selection_by_one_viewport() {
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    // @step And the visible window shows 8 rows
    view.set_visible_rows(8);
    // @step And the selection is at index 8
    view.selected_index = 8;
    view.adjust_scroll();
    // @step When I press the PageUp key
    press(&mut view, KeyCode::PageUp);
    // @step Then the selection moves to index 0
    assert_eq!(view.selected_index, 0);
}

// Feature: spec/features/blocklist-view-keybindings.feature
#[test]
fn end_selects_the_last_rule_and_home_selects_the_first_rule() {
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    // @step And the visible window shows 8 rows
    view.set_visible_rows(8);
    // @step And the selection is at index 5
    view.selected_index = 5;
    view.adjust_scroll();
    // @step When I press the End key
    press(&mut view, KeyCode::End);
    // @step Then the selection moves to the last rule index 19
    assert_eq!(view.selected_index, 19);
    // @step And the visible window scrolls so the selection stays visible
    let off = view.scroll_offset();
    assert!(
        view.selected_index >= off && view.selected_index < off + 8,
        "selection {} not inside window [{}, {})",
        view.selected_index,
        off,
        off + 8
    );
    // @step When I press the Home key
    press(&mut view, KeyCode::Home);
    // @step Then the selection moves to the first rule index 0
    assert_eq!(view.selected_index, 0);
}

// Feature: spec/features/blocklist-view-keybindings.feature
#[test]
fn space_toggles_the_focused_rule() {
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    // @step And the selection is at index 3
    view.selected_index = 3;
    view.adjust_scroll();
    // @step When I press the Space key
    let event = view.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    // @step Then a toggle action is emitted for the focused rule's id
    match event {
        BlocklistEvent::Emit(action) => {
            let Action::ToggleBlocklistRule(id) = *action else {
                panic!("expected Emit(ToggleBlocklistRule), got {action:?}");
            };
            assert_eq!(id, "rule003");
        }
        other => panic!("expected Emit(ToggleBlocklistRule), got {other:?}"),
    }
}

// Feature: spec/features/blocklist-view-keybindings.feature
#[test]
fn footer_hint_no_longer_advertises_vim_keys() {
    // @step Given the blocklist view has rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(3));
    // @step When the view is rendered
    let text = render_text(&mut view, 120, 24);
    let footer = text
        .lines()
        .rev()
        .find(|line| line.contains("Toggle Rule") || line.contains("Navigate"))
        .unwrap_or("");
    // @step Then the footer hint does not contain "jk"
    assert!(
        !footer.contains("jk"),
        "footer should not advertise vim keys: {footer:?}"
    );
    // @step And the footer hint lists the arrow and Page/Home/End keys
    assert!(
        footer.contains("Home") && footer.contains("End"),
        "footer should list Home/End keys: {footer:?}"
    );
    assert!(
        footer.contains("PgUp") || footer.contains("PgDn") || footer.contains("Page"),
        "footer should list Page keys: {footer:?}"
    );
}
