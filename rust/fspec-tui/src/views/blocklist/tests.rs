//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature
//! Feature: spec/features/blocklist-view-scrolling.feature
//!
//! BlocklistView unit + render tests. The RPC-056 tests pin the
//! category heuristic and set_rules clamping; the BLOCK-008 tests pin
//! viewport scrolling parity (nav + render windowing + scrollbar gutter).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use crossterm::event::KeyCode;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::test_support::*;
use super::*;

// -------------------------------------------------------------------
// RPC-056 preserved tests
// -------------------------------------------------------------------

#[test]
fn set_rules_resets_selection_when_index_exceeds_new_len() {
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule("a", "a", "block", "system"),
        rule("b", "b", "block", "system"),
    ]);
    view.selected_index = 1;
    view.set_rules(vec![rule("c", "c", "block", "system")]);
    assert_eq!(view.selected_index, 0);
}

#[test]
fn derive_category_classifies_bash_and_file_path() {
    assert_eq!(derive_category("^cat\\s+"), "bash");
    assert_eq!(derive_category("/etc/passwd"), "file_path");
    assert_eq!(derive_category("~/.aws/.*"), "file_path");
    assert_eq!(derive_category("./scripts/deploy.sh"), "file_path");
    assert_eq!(derive_category("git checkout"), "bash");
}

// -------------------------------------------------------------------
// BLOCK-008 — viewport scrolling
// -------------------------------------------------------------------

// Feature: spec/features/blocklist-view-scrolling.feature
#[test]
fn scrolling_down_keeps_focused_row_inside_window() {
    // @step Given a BlocklistView seeded with 20 rules and a visible window of 8 rows
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And selected_index is 0 and scroll_offset is 0
    assert_eq!(view.selected_index, 0);
    assert_eq!(view.scroll_offset(), 0);
    // @step When the user presses Down 10 times
    for _ in 0..10 {
        press(&mut view, KeyCode::Down);
    }
    // @step Then selected_index equals 10
    assert_eq!(view.selected_index, 10);
    // @step And scroll_offset equals 3 so the focused row stays inside the window
    assert_eq!(view.scroll_offset(), 3);
}

// Feature: spec/features/blocklist-view-scrolling.feature
#[test]
fn scrolling_back_up_above_window_scrolls_offset_back() {
    // @step Given a BlocklistView seeded with 20 rules and a visible window of 8 rows
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And selected_index is 10 and scroll_offset is 3
    for _ in 0..10 {
        press(&mut view, KeyCode::Down);
    }
    assert_eq!(view.selected_index, 10);
    assert_eq!(view.scroll_offset(), 3);
    // @step When the user presses Up 8 times
    for _ in 0..8 {
        press(&mut view, KeyCode::Up);
    }
    // @step Then selected_index equals 2
    assert_eq!(view.selected_index, 2);
    // @step And scroll_offset equals 2 so the focused row stays inside the window
    assert_eq!(view.scroll_offset(), 2);
}

// Feature: spec/features/blocklist-view-scrolling.feature
#[test]
fn scroll_offset_clamps_at_total_minus_visible_on_last_row() {
    // @step Given a BlocklistView seeded with 20 rules and a visible window of 8 rows
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step When the focused row is moved to the last rule
    for _ in 0..25 {
        press(&mut view, KeyCode::Down);
    }
    assert_eq!(view.selected_index, 19);
    // @step Then scroll_offset equals 12
    assert_eq!(view.scroll_offset(), 12);
    // @step And scroll_offset never exceeds total minus visible rows
    assert!(view.scroll_offset() <= 20 - 8);
}

// Feature: spec/features/blocklist-view-scrolling.feature
#[test]
fn rendering_overflowing_list_windows_rows_and_shows_indicator() {
    // @step Given a BlocklistView seeded with 30 rules
    let mut view = BlocklistView::new();
    view.set_rules(rules(30));
    // @step When the view is rendered into a buffer shorter than the rule list
    let text = render_text(&mut view, 80, 14);
    // @step Then only the windowed slice of rows is painted
    assert!(text.contains("rule000"), "first windowed row should paint");
    assert!(
        !text.contains("rule029"),
        "off-window last row should NOT paint: {text}"
    );
    // @step And the rendered text contains a "Showing" scroll indicator reflecting the visible range
    assert!(
        text.contains("Showing"),
        "expected Showing indicator: {text}"
    );
    assert!(text.contains("of 30"), "indicator reflects total: {text}");
}

// Feature: spec/features/blocklist-view-scrolling.feature
#[test]
fn re_seeding_with_shorter_list_resets_scroll_offset() {
    // @step Given a BlocklistView holding 20 rules with scroll_offset 12
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    for _ in 0..25 {
        press(&mut view, KeyCode::Down);
    }
    assert_eq!(view.scroll_offset(), 12);
    assert_eq!(view.selected_index, 19);
    // @step When set_rules replaces the list with 3 rules
    view.set_rules(rules(3));
    // @step Then scroll_offset resets to 0
    assert_eq!(view.scroll_offset(), 0);
    // @step And selected_index is clamped inside the new list
    assert_eq!(view.selected_index, 2);
}

// Feature: spec/features/blocklist-view-scrolling.feature
#[test]
fn overflowing_list_renders_scrollbar_gutter_and_fitting_list_does_not() {
    let disabled: HashSet<String> = HashSet::new();
    // @step Given a BlocklistView seeded with more rules than fit the pane
    let mut view = BlocklistView::new();
    view.set_rules(rules(30));
    // @step When the view is rendered
    let (w, h) = (80u16, 14u16);
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut(), &disabled))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    // @step Then a scrollbar gutter column is drawn in the left pane
    assert!(
        scrollbar_glyphs(&buf, w, h) > 0,
        "expected scrollbar glyphs for overflowing list"
    );

    // @step When the view is re-seeded with a list that fits entirely
    view.set_rules(rules(2));
    // @step And the view is rendered again
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut(), &disabled))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    // @step Then no scrollbar gutter column is drawn
    assert_eq!(
        scrollbar_glyphs(&buf, w, h),
        0,
        "no scrollbar glyphs expected for a fitting list"
    );
}
