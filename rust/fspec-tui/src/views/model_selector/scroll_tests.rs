//! PROV-104 — model-view scroll/viewport parity buffer-render tests.
//!
//! Feature: spec/features/model-view-scroll-viewport.feature
//!
//! Renders the full `ModelSelectorView` into a ratatui `TestBackend` and
//! asserts the SELECTED row is actually PAINTED within the viewport at the
//! top edge, bottom edge, and mid-list — the gap that hid the original bug
//! (prior RPC-340 tests only checked `scroll_offset` arithmetic, never that
//! the selected row was drawn). Lives in its own file (via `#[path]`) so
//! `mod.rs` stays closer to the source-shape budget.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use codelet_rpc_types::ModelEntry;
use crossterm::event::{KeyEventKind, KeyEventState};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: 200_000,
        supports_reasoning: true,
        supports_vision: true,
        is_custom: false,
    }
}

fn provider(key: &str, ids: &[&str]) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: key.to_string(),
        models: ids.iter().map(|i| model(i)).collect(),
        profile_name: None,
        is_unreachable: false,
    }
}

/// One provider with 30 models → a single non-selectable header followed
/// by 30 selectable model rows. Far longer than any test viewport.
fn tall_view() -> ModelSelectorView {
    let ids: Vec<String> = (0..30).map(|i| format!("m{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![provider("openai", &refs)]);
    v.expanded = v.providers.iter().map(|p| p.key.clone()).collect();
    v.rebuild_rows();
    v.anchor_first_selectable();
    v.adjust_scroll();
    v
}

/// Render the whole view into a `w`x`h` TestBackend and return the buffer
/// as one `String` per row. Populates `v.visible_rows` as a side effect.
fn render_lines(v: &mut ModelSelectorView, w: u16, h: u16) -> Vec<String> {
    let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).expect("term");
    term.draw(|f| v.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut lines = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut s = String::new();
        for x in 0..buf.area.width {
            s.push_str(buf[(x, y)].symbol());
        }
        lines.push(s);
    }
    lines
}

/// Indices of rendered lines carrying the selected-row marker `> `
/// (RPC-351: solid cyan band + `> ` arrow replaced the old `▸`).
fn marker_lines(lines: &[String]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("> "))
        .map(|(i, _)| i)
        .collect()
}

/// Painted model rows (every model carries `[200k]`; legend/title never do).
fn model_row_count(lines: &[String]) -> usize {
    lines.iter().filter(|l| l.contains("[200k]")).count()
}

fn first_selectable(v: &ModelSelectorView) -> usize {
    crate::components::model_selector_dialog_rows::first_selectable(&v.rows)
}

/// Assert exactly one selected marker is painted and carries `sel_id`;
/// return the line index of the marker row.
fn assert_selected_painted(lines: &[String], sel_id: &str) -> usize {
    let markers = marker_lines(lines);
    assert_eq!(
        markers.len(),
        1,
        "exactly one selected row painted: {lines:?}"
    );
    let mi = markers[0];
    assert!(
        lines[mi].contains(&format!("> {sel_id} ")),
        "selected id {sel_id} painted on marker line: {}",
        lines[mi]
    );
    mi
}

/// Assert no inline up/down overflow arrow is painted anywhere (parity:
/// the indicator lives in a dedicated scrollbar column, never inline).
fn assert_no_inline_arrows(lines: &[String]) {
    for line in lines {
        assert!(
            !line.contains('↓') && !line.contains('↑'),
            "no inline overflow arrow may steal a content row: {line}"
        );
    }
}

/// Scenario: Down past the bottom paints the selected row on the last content row
#[test]
fn down_past_bottom_paints_selected_on_last_content_row() {
    // @step Given a model list of 30 models in a viewport 10 content rows tall
    let mut v = tall_view();
    render_lines(&mut v, 60, 14);
    let visible = v.visible_rows;
    assert!(visible > 0 && visible < v.rows.len());

    // @step When I press Down until the selection would fall below the visible window
    for _ in 0..(visible + 5) {
        v.handle_key(key(KeyCode::Down));
    }
    let lines = render_lines(&mut v, 60, 14);
    let sel_id = v.rows[v.selected_index].model_id.clone();

    // @step Then the selected model row is painted within the visible viewport
    let mi = assert_selected_painted(&lines, &sel_id);

    // @step And the selected model row is painted on the last visible content row
    for line in &lines[mi + 1..] {
        assert!(
            !line.contains("[200k]"),
            "no model row may be painted below the bottom-edge selection: {line}"
        );
    }

    // @step And no overflow indicator overwrites the selected row
    assert_no_inline_arrows(&lines);
}

/// Scenario: Returning to the first model paints it on the first content row
#[test]
fn returning_to_first_model_paints_it_with_header_above() {
    // @step Given a model list of 30 models in a viewport 10 content rows tall
    let mut v = tall_view();
    render_lines(&mut v, 60, 14);
    let visible = v.visible_rows;

    // @step And the viewport has been scrolled down away from the top
    for _ in 0..(visible + 5) {
        v.handle_key(key(KeyCode::Down));
    }
    assert!(v.scroll_offset > 0, "precondition: scrolled away from top");

    // @step When I press Up until the cursor reaches the first model
    let first = first_selectable(&v);
    while v.selected_index > first {
        v.handle_key(key(KeyCode::Up));
    }
    let lines = render_lines(&mut v, 60, 14);

    // @step Then the selected model row is painted within the visible viewport
    let mi = assert_selected_painted(&lines, "m0");

    // @step And the leading provider header is painted above the selected model row
    let header_above = lines[..mi].iter().any(|l| l.contains("openai"));
    assert!(
        header_above,
        "provider header must be painted above: {lines:?}"
    );

    // @step And the scroll offset returns to 0
    assert_eq!(v.scroll_offset, 0);
}

/// Scenario: End jumps to the last model and paints it at the bottom edge
#[test]
fn end_paints_last_model_at_bottom_edge() {
    // @step Given a model list of 30 models in a viewport 10 content rows tall
    let mut v = tall_view();
    render_lines(&mut v, 60, 14);

    // @step When I press End
    v.handle_key(key(KeyCode::End));
    let lines = render_lines(&mut v, 60, 14);
    let sel_id = v.rows[v.selected_index].model_id.clone();

    // @step Then the selected model row is the last model row
    assert_eq!(
        v.selected_index,
        crate::components::model_selector_dialog_rows::last_selectable(&v.rows)
    );

    // @step And the selected model row is painted within the visible viewport
    assert_selected_painted(&lines, &sel_id);

    // @step And no inline arrow overwrites the last content row
    assert_no_inline_arrows(&lines);
}

/// Scenario: A mid-list selection is painted within the viewport
#[test]
fn mid_list_selection_is_painted_and_not_stolen() {
    // @step Given a model list of 30 models in a viewport 10 content rows tall
    let mut v = tall_view();
    render_lines(&mut v, 60, 14);

    // @step When I move the selection to a model in the middle of the list
    for _ in 0..4 {
        v.handle_key(key(KeyCode::Down));
    }
    let lines = render_lines(&mut v, 60, 14);
    let sel_id = v.rows[v.selected_index].model_id.clone();

    // @step Then the selected model row is painted within the visible viewport
    assert_selected_painted(&lines, &sel_id);

    // @step And the selected model row is not stolen by an overflow indicator
    assert_no_inline_arrows(&lines);
}

/// Scenario: PageDown advances by one viewport height and keeps the selection painted
#[test]
fn page_down_advances_one_viewport_and_keeps_painted() {
    // @step Given a model list of 30 models in a viewport 10 content rows tall
    let mut v = tall_view();
    render_lines(&mut v, 60, 14);
    let visible = v.visible_rows;
    let before = v.selected_index;

    // @step When I press PageDown
    v.handle_key(key(KeyCode::PageDown));
    let after = v.selected_index;

    // @step Then the selection advances by approximately one viewport height
    assert!(
        after >= before + visible - 1,
        "PageDown advances ~one viewport: before={before} after={after} visible={visible}"
    );

    // @step And the selected model row is painted within the visible viewport
    let lines = render_lines(&mut v, 60, 14);
    let sel_id = v.rows[v.selected_index].model_id.clone();
    assert_selected_painted(&lines, &sel_id);
}

/// Scenario: An overflowing list paints a scrollbar column beside the content
#[test]
fn overflow_paints_scrollbar_column_beside_content() {
    // @step Given a model list of 30 models in a viewport 10 content rows tall
    let mut v = tall_view();
    render_lines(&mut v, 60, 14);
    let visible = v.visible_rows;
    for _ in 0..(visible + 5) {
        v.handle_key(key(KeyCode::Down));
    }

    // @step When the body is rendered
    let lines = render_lines(&mut v, 60, 14);

    // @step Then a scrollbar column is painted beside the list
    let bar = |l: &String| l.contains('■') || l.contains('│');
    assert!(lines.iter().any(bar), "scrollbar column painted: {lines:?}");

    // @step And the rightmost content column still shows model text
    assert_eq!(
        model_row_count(&lines),
        visible,
        "all {visible} visible rows must paint content (none stolen): {lines:?}"
    );
    assert!(
        lines.iter().any(|l| bar(l) && l.contains("[200k]")),
        "scrollbar must sit beside content, not replace a row: {lines:?}"
    );
}
