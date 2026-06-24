//! PROV-107 — RPC-337 row projection + body-render tests
//! (badges, legend, placeholder, collapse, filter, current marker).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::rows_test_support::*;
use super::*;

/// Scenario: Model rows display capability badges
#[test]
fn custom_model_row_shows_badges_in_ts_order() {
    // @step Given the model selector lists a custom model supporting reasoning and vision with a 200k context window
    let providers = vec![provider(
        "openai",
        vec![model("gpt", true, true, 200_000, true)],
    )];

    // @step When the row is rendered while unselected
    let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");
    let model_row = rows.iter().find(|r| r.selectable).expect("model row");

    // @step Then it shows the badges "[C]", "[R]", "[V]" and "[200k]" in that order
    assert_eq!(model_row.badges, " [C] [R] [V] [200k]");

    // @step And the "[C]" badge is yellow, "[R]" magenta, "[V]" blue and "[200k]" gray
    assert_eq!(badge_token_style("[C]").fg, Some(Color::Yellow));
    assert_eq!(badge_token_style("[R]").fg, Some(Color::Magenta));
    assert_eq!(badge_token_style("[V]").fg, Some(Color::Blue));
    assert_eq!(badge_token_style("[200k]").fg, Some(Color::Gray));
}

/// Scenario: The body renders the capability legend
#[test]
fn body_renders_capability_legend_on_bottom_row() {
    // @step Given the model selector is open
    let providers = vec![provider(
        "openai",
        vec![model("gpt", false, false, 8_000, false)],
    )];
    let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");

    // @step When the body is rendered
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).expect("term");
    term.draw(|f| render_body(f.area(), f.buffer_mut(), &rows, true, 1, 0, None))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }

    // @step Then a legend line "[R] Reasoning | [V] Vision | [C] Custom" appears at the bottom of the body
    // (The 📁 segment is a wide-glyph; assert the badge prefix verbatim
    //  and the profile segment text separately — see RPC-338.)
    assert!(
        joined.contains("[R] Reasoning | [V] Vision | [C] Custom"),
        "legend missing: {joined}"
    );
}

/// Scenario: Providers still loading shows a placeholder
#[test]
fn empty_rows_render_placeholder() {
    // @step Given the model selector has opened but providers have not loaded
    let rows: Vec<ModelSelectorRow> = build_view_rows(&[], &expanded_set(&[]), "");

    // @step When the body is rendered
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).expect("term");
    // PROV-104: a not-yet-loaded body (`loaded == false`) now paints a
    // DISTINCT loading indicator rather than the no-models empty state.
    term.draw(|f| render_body(f.area(), f.buffer_mut(), &rows, false, 0, 0, None))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }

    // @step Then it shows the "No providers available" placeholder
    // PROV-104 contract update (rule [8]): the not-loaded state now shows a
    // distinct loading indicator, NOT the no-models empty placeholder.
    assert!(
        joined.contains(LOADING_PLACEHOLDER),
        "loading indicator missing: {joined}"
    );
    assert!(
        !joined.contains(EMPTY_PLACEHOLDER),
        "loading state must be distinct from the no-models empty state: {joined}"
    );
    // @step And the placeholder is replaced once the provider list arrives
    let loaded = build_view_rows(
        &[provider(
            "openai",
            vec![model("gpt", false, false, 8_000, false)],
        )],
        &expanded_set(&["openai"]),
        "",
    );
    assert!(loaded.iter().any(|r| r.selectable));
}

/// Scenario: Expanding and collapsing a provider group
#[test]
fn collapsed_provider_hides_models_expanded_shows_them() {
    // @step Given the model selector shows an expanded provider group
    let providers = vec![provider(
        "openai",
        vec![model("gpt", false, false, 8_000, false)],
    )];
    let expanded = build_view_rows(&providers, &expanded_set(&["openai"]), "");
    assert!(
        expanded.iter().any(|r| r.selectable),
        "expanded shows models"
    );
    assert!(
        expanded[0].label.starts_with('▼'),
        "expanded header arrow ▼"
    );

    // @step When I press the left arrow on the provider group
    // @step Then the group collapses and hides its model rows
    let collapsed = build_view_rows(&providers, &expanded_set(&[]), "");
    assert!(
        !collapsed.iter().any(|r| r.selectable),
        "collapsed hides models"
    );
    assert!(
        collapsed[0].label.starts_with('▶'),
        "collapsed header arrow ▶"
    );
}

/// Scenario: Filtering narrows the model list
#[test]
fn filter_narrows_models_and_clearing_restores() {
    // @step Given the model selector is showing all providers and models
    let providers = vec![provider(
        "openai",
        vec![
            model("gpt-4o", false, false, 8_000, false),
            model("o3-mini", true, false, 8_000, false),
        ],
    )];
    let all = build_view_rows(&providers, &expanded_set(&["openai"]), "");
    assert_eq!(all.iter().filter(|r| r.selectable).count(), 2);

    // @step When I press "/" and type filter text
    let filtered = build_view_rows(&providers, &expanded_set(&[]), "o3");

    // @step Then the list narrows to models matching the filter
    let model_rows: Vec<_> = filtered.iter().filter(|r| r.selectable).collect();
    assert_eq!(model_rows.len(), 1);
    assert!(model_rows[0].model_id.contains("o3"));

    // @step And clearing the filter restores the full list
    let restored = build_view_rows(&providers, &expanded_set(&["openai"]), "");
    assert_eq!(restored.iter().filter(|r| r.selectable).count(), 2);
}

/// Scenario: The active session model shows a current marker
#[test]
fn current_model_row_shows_green_current_marker() {
    // @step Given the model selector lists a model whose id matches the active session model
    let providers = vec![provider(
        "openai",
        vec![model("gpt-4o", false, false, 8_000, false)],
    )];
    let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");

    // @step When the list is rendered
    // (selection rests on the header so the current model row renders
    //  unselected — the green (current) marker only shows when the row
    //  is not inverse-highlighted, matching ModelSelectorView.tsx)
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).expect("term");
    term.draw(|f| render_body(f.area(), f.buffer_mut(), &rows, true, 0, 0, Some("gpt-4o")))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    let mut found_green_current = false;
    for y in 0..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            line.push_str(cell.symbol());
        }
        if line.contains("(current)") {
            // verify at least one cell on this row is green
            for x in 0..buf.area.width {
                if buf[(x, y)].fg == Color::Green {
                    found_green_current = true;
                }
            }
        }
        joined.push_str(&line);
        joined.push('\n');
    }

    // @step Then that model row shows a green "(current)" marker
    assert!(
        joined.contains("(current)"),
        "current marker text missing: {joined}"
    );
    assert!(found_green_current, "current marker not green");
}
