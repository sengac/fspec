//! RPC-351 — selection-style + arrow parity tests for the full-screen
//! ModelSelector mode-view.
//!
//! Feature: spec/features/model-selector-selection-style.feature
//!
//! Pins the TS-parity contract: a selected row paints a solid cyan
//! background band (fg=Black) — NOT terminal reverse-video — across the
//! FULL row width, model rows carry a `> ` arrow, header rows prepend the
//! `> ` selection marker before the ▼/▶ expand icon, and every inline
//! coloured token flips to Black when its row is selected. Unselected
//! rows are unchanged.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::rows_test_support::*;
use super::*;
use ratatui::style::Modifier;

/// Render the body to a per-cell `(symbol, fg, bg, modifier)` grid so
/// tests can assert the band colours and the absence of REVERSED.
fn render_cells(
    rows: &[ModelSelectorRow],
    selected: usize,
    current: Option<&str>,
) -> Vec<(u16, u16, String, Color, Color, Modifier)> {
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).expect("term");
    term.draw(|f| {
        let _ = render_body(f.area(), f.buffer_mut(), rows, true, selected, 0, current);
    })
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut cells = Vec::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            cells.push((
                x,
                y,
                cell.symbol().to_string(),
                cell.fg,
                cell.bg,
                cell.modifier,
            ));
        }
    }
    cells
}

/// Find the y-row index containing the first occurrence of `needle`.
fn row_of(cells: &[(u16, u16, String, Color, Color, Modifier)], needle: &str) -> u16 {
    let mut by_row: std::collections::BTreeMap<u16, String> = std::collections::BTreeMap::new();
    for (_, y, sym, _, _, _) in cells {
        by_row.entry(*y).or_default().push_str(sym);
    }
    for (y, line) in by_row {
        if line.contains(needle) {
            return y;
        }
    }
    panic!("needle {needle:?} not found in any row");
}

/// Scenario: A selected model row paints a solid cyan band with a > arrow
#[test]
fn selected_model_row_paints_cyan_band_with_arrow() {
    // @step Given the model selector lists a model row
    let providers = vec![provider(
        "openai",
        vec![model("gpt-4o", false, false, 8_000, false)],
    )];
    let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");
    let sel = rows.iter().position(|r| r.selectable).expect("model row");

    // @step When that model row is rendered while selected
    let cells = render_cells(&rows, sel, None);
    let y = row_of(&cells, "gpt-4o");
    let row: Vec<_> = cells.iter().filter(|c| c.1 == y).collect();

    // @step Then the row paints a solid cyan background with black foreground
    assert!(
        row.iter().all(|c| c.4 == Color::Cyan),
        "every cell on the selected model row must have a cyan bg"
    );
    let label_cells: Vec<_> = row.iter().filter(|c| c.2 != " ").collect();
    assert!(
        label_cells.iter().all(|c| c.3 == Color::Black),
        "foreground text on the selected row must be black"
    );

    // @step And the row is not styled with reverse video
    assert!(
        row.iter().all(|c| !c.5.contains(Modifier::REVERSED)),
        "the selected row must not use terminal reverse video"
    );

    // @step And the row shows a "> " arrow marker
    let line: String = row.iter().map(|c| c.2.clone()).collect();
    assert!(line.contains("> gpt-4o"), "missing '> ' arrow: {line}");
}

/// Scenario: The selection band fills the full row width
#[test]
fn selection_band_fills_full_row_width() {
    // @step Given the model selector lists a short model row
    let providers = vec![provider("o", vec![model("x", false, false, 0, false)])];
    let rows = build_view_rows(&providers, &expanded_set(&["o"]), "");
    let sel = rows.iter().position(|r| r.selectable).expect("model row");

    // @step When that model row is rendered while selected
    let cells = render_cells(&rows, sel, None);
    let y = row_of(&cells, "> x");

    // @step Then the cyan background extends to the right edge of the row
    let last = cells
        .iter()
        .filter(|c| c.1 == y)
        .max_by_key(|c| c.0)
        .expect("rightmost cell");
    assert_eq!(
        last.4,
        Color::Cyan,
        "the cyan band must extend to the right edge of the row"
    );
}

/// Scenario: A selected header row prepends the selection marker before the expand icon
#[test]
fn selected_header_prepends_marker_before_expand_icon() {
    // @step Given the model selector shows an expanded provider header row
    let providers = vec![provider(
        "openai",
        vec![model("gpt-4o", false, false, 8_000, false)],
    )];
    let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");
    assert!(!rows[0].selectable, "row 0 is the header");

    // @step When that header row is rendered while selected
    let cells = render_cells(&rows, 0, None);
    let row: Vec<_> = cells.iter().filter(|c| c.1 == 0).collect();
    let line: String = row.iter().map(|c| c.2.clone()).collect();

    // @step Then the header prepends "> " before the expand icon
    let marker = line.find("> ").expect("selection marker");
    let icon = line.find('▼').expect("expand icon");
    assert!(marker < icon, "'> ' must come before ▼: {line}");

    // @step And the header paints a solid cyan background with black foreground
    assert!(
        row.iter().all(|c| c.4 == Color::Cyan),
        "selected header must have cyan bg"
    );
    let glyph_cells: Vec<_> = row.iter().filter(|c| c.2 != " ").collect();
    assert!(
        glyph_cells.iter().all(|c| c.3 == Color::Black),
        "selected header text must be black"
    );
}

/// Scenario: An unselected header row prepends padding before the expand icon
#[test]
fn unselected_header_prepends_padding() {
    // @step Given the model selector shows an expanded provider header row
    let providers = vec![provider(
        "openai",
        vec![model("gpt-4o", false, false, 8_000, false)],
    )];
    let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");

    // @step When that header row is rendered while not selected
    // (selection parked on the model row so the header renders unselected)
    let sel = rows.iter().position(|r| r.selectable).expect("model row");
    let cells = render_cells(&rows, sel, None);
    let row: Vec<_> = cells.iter().filter(|c| c.1 == 0).collect();
    let line: String = row.iter().map(|c| c.2.clone()).collect();

    // @step Then the header prepends two spaces before the expand icon
    let icon = line.find('▼').expect("expand icon");
    assert!(
        line[..icon].starts_with("  "),
        "unselected header must prepend two spaces before ▼: {line:?}"
    );
    assert!(
        !line[..icon].contains('>'),
        "unselected header must not show the selection marker: {line:?}"
    );

    // @step And the header shows no cyan background
    assert!(
        row.iter().all(|c| c.4 != Color::Cyan),
        "unselected header must not have a cyan band"
    );
}

/// Scenario: A selected model row flips every inline token to black
#[test]
fn selected_model_row_flips_tokens_black() {
    // @step Given the model selector lists a model row with custom, reasoning, vision and context badges that is the current model
    let providers = vec![provider(
        "openai",
        vec![model("gpt-4o", true, true, 200_000, true)],
    )];
    let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");
    let sel = rows.iter().position(|r| r.selectable).expect("model row");

    // @step When that model row is rendered while selected
    let cells = render_cells(&rows, sel, Some("gpt-4o"));
    let y = row_of(&cells, "(current)");
    let glyphs: Vec<_> = cells.iter().filter(|c| c.1 == y && c.2 != " ").collect();

    // @step Then the badges are rendered black
    for token in ["[C]", "[R]", "[V]", "[200k]"] {
        let first = token.chars().next().unwrap().to_string();
        assert!(
            glyphs.iter().any(|c| c.2 == first && c.3 == Color::Black),
            "badge {token} must render black on the selected row"
        );
    }
    assert!(
        glyphs.iter().all(|c| c.3 == Color::Black),
        "no inline token on the selected row may keep an accent colour"
    );

    // @step And the "(current)" marker is rendered black
    assert!(
        glyphs.iter().all(|c| c.3 != Color::Green),
        "(current) must flip to black, not stay green, on the selected row"
    );
}

/// Scenario: A selected profile header flips the folder and unreachable markers to black
#[test]
fn selected_profile_header_flips_markers_black() {
    // @step Given the model selector shows an unreachable profile header row
    let providers = vec![profile_provider(
        "openai:down-profile",
        "openai: down-profile",
        "down-profile",
        true,
        Vec::new(),
    )];
    let rows = build_view_rows(&providers, &expanded_set(&["openai:down-profile"]), "");

    // @step When that header row is rendered while selected
    let cells = render_cells(&rows, 0, None);
    let row: Vec<_> = cells.iter().filter(|c| c.1 == 0).collect();

    // @step Then the folder icon is rendered black rather than magenta
    assert!(
        row.iter().any(|c| c.2 == "📁" && c.3 == Color::Black),
        "selected 📁 must render black"
    );
    assert!(
        row.iter().all(|c| !(c.2 == "📁" && c.3 == Color::Magenta)),
        "selected 📁 must not stay magenta"
    );

    // @step And the "(unreachable)" marker is rendered black rather than red
    assert!(
        row.iter().all(|c| c.3 != Color::Red),
        "selected (unreachable) must flip to black, not stay red"
    );
    let line: String = row.iter().map(|c| c.2.clone()).collect();
    assert!(line.contains("(unreachable)"), "marker present: {line}");
}

/// Scenario: An unselected model row is unchanged
#[test]
fn unselected_model_row_unchanged() {
    // @step Given the model selector lists a model row with badges
    let providers = vec![provider(
        "openai",
        vec![model("gpt-4o", true, true, 200_000, true)],
    )];
    let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");

    // @step When that model row is rendered while not selected
    // (selection parked on the header so the model row renders unselected)
    let cells = render_cells(&rows, 0, None);
    let y = row_of(&cells, "gpt-4o");
    let row: Vec<_> = cells.iter().filter(|c| c.1 == y).collect();

    // @step Then the model label keeps its white foreground
    assert!(
        row.iter().any(|c| c.2 == "g" && c.3 == Color::White),
        "unselected model label must stay white"
    );

    // @step And the badges keep their accent colours and are dimmed
    assert!(
        row.iter()
            .any(|c| c.2 == "[" && c.3 == Color::Yellow && c.5.contains(Modifier::DIM)),
        "[C] badge must stay yellow + dim when unselected"
    );

    // @step And the row shows no cyan background
    assert!(
        row.iter().all(|c| c.4 != Color::Cyan),
        "unselected model row must not have a cyan band"
    );
}
