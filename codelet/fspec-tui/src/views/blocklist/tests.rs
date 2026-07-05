//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature
//! Feature: spec/features/blocklist-view-scrolling.feature
//!
//! BlocklistView unit + render tests. The RPC-056 tests pin the
//! category heuristic and set_rules clamping; the BLOCK-008 tests pin
//! viewport scrolling parity (nav + render windowing + scrollbar gutter).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_rpc_types::BlocklistRuleInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;

fn rule(id: &str, pattern: &str, action: &str, source: &str) -> BlocklistRuleInfo {
    BlocklistRuleInfo {
        id: id.to_string(),
        pattern: pattern.to_string(),
        action: action.to_string(),
        reason: String::new(),
        guidance: None,
        source: source.to_string(),
    }
}

/// Build `n` block rules with ids `rule000..`.
fn rules(n: usize) -> Vec<BlocklistRuleInfo> {
    (0..n)
        .map(|i| rule(&format!("rule{i:03}"), &format!("^cmd{i}"), "block", "system"))
        .collect()
}

fn press(view: &mut BlocklistView, code: KeyCode) {
    view.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

/// Render the view into a `TestBackend` buffer and return the joined
/// text of every row (used for buffer-text assertions).
fn render_text(view: &mut BlocklistView, w: u16, h: u16) -> String {
    let disabled: HashSet<String> = HashSet::new();
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut(), &disabled))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

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

/// Count scrollbar glyphs (`■`/`│`) painted inside the LEFT list pane's
/// interior. BLOCK-009 adds a `render_vertical_divider` `│` column at the
/// pane boundary (~x = w/2 - 1); we stop BEFORE it so only the left
/// pane's own scrollbar gutter contributes — never the divider.
fn scrollbar_glyphs(buf: &ratatui::buffer::Buffer, w: u16, h: u16) -> usize {
    let mut count = 0;
    // Left list pane only: stop one column short of the divider gutter.
    let x_start = 0u16;
    let x_end = (w / 2).saturating_sub(1);
    for y in 1..h.saturating_sub(1) {
        for x in x_start..x_end {
            let sym = buf[(x, y)].symbol();
            if sym == "■" || sym == "│" {
                count += 1;
            }
        }
    }
    count
}

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
        BlocklistEvent::Emit(Action::ToggleBlocklistRule(id)) => {
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

// Feature: spec/features/blocklist-view-framing.feature
#[test]
fn header_shows_the_rules_count() {
    // @step Given a BlocklistView seeded with 2 rules
    let mut view = BlocklistView::new();
    view.set_rules(rules(2));
    // @step When the view is rendered into a 120x24 buffer
    let text = render_text(&mut view, 120, 24);
    // @step Then the rendered header contains "Blocklist Rules (2 rules)"
    assert!(
        text.contains("Blocklist Rules (2 rules)"),
        "missing count header: {text}"
    );
}

// Feature: spec/features/blocklist-view-framing.feature
#[test]
fn a_vertical_divider_separates_the_list_and_details_panes() {
    // @step Given a BlocklistView seeded with rules
    let mut view = BlocklistView::new();
    view.set_rules(rules(3));
    // @step When the view is rendered into a 120x24 buffer
    let disabled: HashSet<String> = HashSet::new();
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut(), &disabled))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    // @step Then a vertical divider column separates the list pane from the details pane
    // The [Percentage(50), Length(1), Percentage(50)] split puts the
    // divider gutter at the horizontal midpoint (~x = width/2 = 60).
    // Assert a "│" runs down that column across the body rows, distinct
    // from any left-pane scrollbar gutter (which sits left of midpoint).
    let mid = 120u16 / 2;
    let mut divider_col = None;
    for col in mid.saturating_sub(1)..=mid + 1 {
        let mut runs = 0;
        for y in 2..22 {
            if buf[(col, y)].symbol() == "│" {
                runs += 1;
            }
        }
        if runs >= 10 {
            divider_col = Some(col);
            break;
        }
    }
    assert!(
        divider_col.is_some(),
        "expected a vertical divider column near the midpoint"
    );
}

// Feature: spec/features/blocklist-view-framing.feature
#[test]
fn footer_shows_the_reference_parity_hint() {
    // @step Given a BlocklistView seeded with rules
    let mut view = BlocklistView::new();
    view.set_rules(rules(3));
    // @step When the view is rendered into a 120x24 buffer
    let text = render_text(&mut view, 120, 24);
    // @step Then the rendered footer contains "Enter/Space: Toggle Rule"
    assert!(
        text.contains("Enter/Space: Toggle Rule"),
        "missing toggle hint: {text}"
    );
    // @step And the rendered footer contains "Esc: Close"
    assert!(text.contains("Esc: Close"), "missing esc hint: {text}");
}

// -------------------------------------------------------------------
// BLOCK-011 — mouse-wheel scroll support
// Feature: spec/features/blocklist-view-mouse-scroll.feature
//
// Level choice: the first four scenarios are exercised at the view
// level (in-module so we can read the private `selected_index` +
// `scroll_offset()` and drive `set_visible_rows`). The fifth
// (navigator-routing) scenario is exercised through the real
// `Navigator::handle_blocklist_event` via `crate::views::navigator`
// (mirroring the model_selector RPC-345 `switch_to_providers_flips_...`
// navigator test) — the strongest reliable assertion, since the
// navigator branch is the thin passthrough under test.
//
// WheelVelocity caveat: `WheelVelocity::step` accelerates on rapid
// consecutive events (1×–5× ramp). For a SINGLE event from a fresh
// view the step is exactly 1, so we assert exact index 1 deterministically.
// -------------------------------------------------------------------

/// Build a synthetic wheel/move MouseEvent. Column/row are irrelevant to
/// the wheel handler (it does no hit-testing), so we pin them at (1, 1).
fn mouse_event(kind: crossterm::event::MouseEventKind) -> crossterm::event::MouseEvent {
    crossterm::event::MouseEvent {
        kind,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    }
}

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn wheel_down_moves_the_selection_down() {
    use crossterm::event::MouseEventKind;
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And the selection is at index 0
    view.selected_index = 0;
    view.adjust_scroll();
    // @step When I scroll the mouse wheel down once
    view.handle_mouse(mouse_event(MouseEventKind::ScrollDown));
    // @step Then the selection moves to index 1
    // (fresh WheelVelocity → step is exactly 1× on the first event)
    assert_eq!(view.selected_index, 1);
    // @step And the visible window scrolls to keep the selection visible
    let off = view.scroll_offset();
    assert!(
        view.selected_index >= off && view.selected_index < off + 8,
        "selection {} not inside window [{}, {})",
        view.selected_index,
        off,
        off + 8
    );
}

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn wheel_down_clamps_at_the_last_rule() {
    use crossterm::event::MouseEventKind;
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And the selection is at the last rule index 19
    view.selected_index = 19;
    view.adjust_scroll();
    // @step When I scroll the mouse wheel down once
    view.handle_mouse(mouse_event(MouseEventKind::ScrollDown));
    // @step Then the selection stays at index 19
    assert_eq!(view.selected_index, 19);
}

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn wheel_up_clamps_at_the_first_rule() {
    use crossterm::event::MouseEventKind;
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And the selection is at index 0
    view.selected_index = 0;
    view.adjust_scroll();
    // @step When I scroll the mouse wheel up once
    view.handle_mouse(mouse_event(MouseEventKind::ScrollUp));
    // @step Then the selection stays at index 0
    assert_eq!(view.selected_index, 0);
}

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn non_wheel_mouse_events_are_ignored() {
    use crossterm::event::MouseEventKind;
    // @step Given the blocklist view has 20 rules loaded
    let mut view = BlocklistView::new();
    view.set_rules(rules(20));
    view.set_visible_rows(8);
    // @step And the selection is at index 5
    view.selected_index = 5;
    view.adjust_scroll();
    // @step When I move the mouse over the view
    let outcome = view.handle_mouse(mouse_event(MouseEventKind::Moved));
    // @step Then the selection stays at index 5
    assert_eq!(view.selected_index, 5);
    // @step And the mouse event is reported as ignored
    assert!(
        matches!(outcome, BlocklistEvent::Ignored),
        "expected Ignored, got {outcome:?}"
    );
}

// Feature: spec/features/blocklist-view-mouse-scroll.feature
#[test]
fn the_navigator_routes_wheel_events_to_the_view() {
    use crate::views::navigator::{Navigator, ViewMode};
    use crate::theme::Theme;
    use crossterm::event::{Event, MouseEventKind};
    use std::sync::Arc;
    use tokio::sync::mpsc::unbounded_channel;

    // @step Given the blocklist view has 20 rules loaded
    let (tx, _rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    nav.active_view = ViewMode::Blocklist;
    nav.blocklist.set_rules(rules(20));
    nav.blocklist.set_visible_rows(8);
    // @step And the selection is at index 0
    nav.blocklist.selected_index = 0;
    nav.blocklist.adjust_scroll();
    // @step When a mouse wheel down event is delivered through the navigator
    let event = Event::Mouse(mouse_event(MouseEventKind::ScrollDown));
    let result = nav.handle_blocklist_event(&event);
    // @step Then the navigator reports the event as consumed
    assert!(result.is_consumed(), "navigator should consume wheel event");
    // @step And the selection moves down
    assert!(
        nav.blocklist.selected_index > 0,
        "selection should move down, got {}",
        nav.blocklist.selected_index
    );
}

// Feature: spec/features/blocklist-view-framing.feature
#[test]
fn rpc056_rendering_behaviour_is_preserved_after_the_framing_change() {
    // @step Given a BlocklistView seeded with rules [git-checkout-block(system, block), cat-block(project, block)]
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule("git-checkout-block", "^git\\s+checkout\\b", "block", "system"),
        rule("cat-block", "^cat\\s+", "block", "project"),
    ]);
    // @step And the focused session's blocklist_disabled set contains "git-checkout-block"
    let mut disabled: HashSet<String> = HashSet::new();
    disabled.insert("git-checkout-block".to_string());
    // @step When the view is rendered into a 120x24 buffer
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut(), &disabled))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut text = String::new();
    for y in 0..24 {
        for x in 0..120 {
            text.push_str(buf[(x, y)].symbol());
        }
        text.push('\n');
    }
    // @step Then the rendered text contains "git-checkout-block"
    assert!(text.contains("git-checkout-block"), "missing id: {text}");
    // @step And the rendered text contains "system"
    assert!(text.contains("system"), "missing system tag");
    // @step And the rendered text contains "project"
    assert!(text.contains("project"), "missing project tag");
    // @step And the rendered text contains "○ git-checkout-block"
    assert!(
        text.contains("○ git-checkout-block"),
        "missing disabled glyph: {text}"
    );
    // @step And the rendered text contains "(disabled)"
    assert!(text.contains("(disabled)"), "missing (disabled) suffix");

    // @step When the view is re-seeded with an empty rule list
    view.set_rules(Vec::new());
    // @step And the view is rendered again
    let text2 = render_text(&mut view, 120, 24);
    // @step Then the rendered text contains "No blocklist rules configured"
    assert!(
        text2.contains("No blocklist rules configured"),
        "missing empty-state: {text2}"
    );
}
