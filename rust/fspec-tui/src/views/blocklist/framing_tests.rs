//! Feature: spec/features/blocklist-view-framing.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::collections::HashSet;

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

// Feature: spec/features/blocklist-view-framing.feature
#[test]
fn rpc056_rendering_behaviour_is_preserved_after_the_framing_change() {
    // @step Given a BlocklistView seeded with rules [git-checkout-block(system, block), cat-block(project, block)]
    let mut view = BlocklistView::new();
    view.set_rules(vec![
        rule(
            "git-checkout-block",
            "^git\\s+checkout\\b",
            "block",
            "system",
        ),
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
