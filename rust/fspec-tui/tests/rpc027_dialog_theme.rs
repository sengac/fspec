//! RPC-027 — Tests for the canonical fspec dialog theme.
//!
//! Feature: spec/features/rpc027-dialog-theme.feature
//!
//! These tests pin Section A (shared dialog_theme renderer
//! fundamentals) of the RPC-027 acceptance criteria.
//!
//! Scenario coverage:
//!   1. dialog_theme module exposes the canonical public API
//!   2. render_dialog paints a rounded border in the accent color
//!   3. render_dialog paints an opaque black background over the dialog rect
//!   4. render_dialog paints the inner title as a bold accent-colored row
//!   5. render_dialog inserts one blank row between title and body
//!   6. render_dialog paints the selected row with the full-width inverse highlight
//!   7. label_description_row prefixes selected rows with "▸ " and others with "  "
//!   8. label_description_row dims the description text for unselected rows only
//!   9. render_dialog paints the footer with Modifier::DIM and centered alignment
//!  10. dialog_rect centers the dialog within the available area
//!  11. dialog_rect clamps to the parent area when content exceeds it

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::text::Span;
use ratatui::Terminal;

use codelet_fspec_tui::components::dialog_theme::{
    dialog_rect, render_dialog, Accent, DialogRow, FspecDialog, FOOTER_SEPARATOR, MARKER_SELECTED,
    MARKER_UNSELECTED,
};
use codelet_fspec_tui::components::dialog_theme_rows::label_description_row;

/// Helper: draw the dialog into an 80x24 TestBackend and return the
/// resulting Buffer for cell-level assertions.
fn render_to_buffer(dialog: &FspecDialog<'_>) -> Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Terminal::new(TestBackend)");
    terminal
        .draw(|frame| {
            render_dialog(frame.area(), frame.buffer_mut(), dialog);
        })
        .expect("draw");
    terminal.backend().buffer().clone()
}

/// Helper: scan the buffer for the first cell containing `needle` and
/// return its (x, y) position in COLUMN coordinates (not bytes).
fn find_text(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    let needle_chars: Vec<char> = needle.chars().collect();
    if needle_chars.is_empty() {
        return None;
    }
    for y in 0..buf.area.height {
        let row: Vec<char> = (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
            .collect();
        for start in 0..row.len() {
            if start + needle_chars.len() > row.len() {
                break;
            }
            if row[start..start + needle_chars.len()] == needle_chars[..] {
                return Some((start as u16, y));
            }
        }
    }
    None
}

/// Helper: collect the entire buffer into a single newline-joined
/// string for substring assertions.
fn buffer_text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Scenario: dialog_theme module exposes the canonical public API
#[test]
fn dialog_theme_module_exposes_the_canonical_public_api() {
    // @step Given the file rust/fspec-tui/src/components/dialog_theme.rs exists
    // @step When I inspect its public exports
    // @step Then it exposes the Accent enum with variants Cyan, Yellow and Red
    let _ = Accent::Cyan;
    let _ = Accent::Yellow;
    let _ = Accent::Red;

    // @step And it exposes the DialogRow struct with spans, selectable, and selected fields
    let row = DialogRow {
        spans: vec![Span::raw("x")],
        selectable: true,
        selected: false,
    };
    assert!(!row.selected);

    // @step And it exposes the FspecDialog struct with accent, title, rows, footer, and min_width fields
    let dialog = FspecDialog {
        accent: Accent::Cyan,
        title: "T",
        rows: vec![],
        footer: "",
        min_width: 40,
    };
    assert_eq!(dialog.title, "T");

    // @step And it exposes the render_dialog function
    // @step And it exposes the dialog_rect function
    // @step And it exposes the label_description_row helper
    // (compile-time presence — references above already validate this)

    // @step And it exposes the MARKER_SELECTED constant equal to "▸ "
    assert_eq!(MARKER_SELECTED, "▸ ");

    // @step And it exposes the MARKER_UNSELECTED constant equal to "  "
    assert_eq!(MARKER_UNSELECTED, "  ");

    // @step And it exposes the FOOTER_SEPARATOR constant equal to " │ "
    assert_eq!(FOOTER_SEPARATOR, " │ ");
}

/// Scenario: render_dialog paints a rounded border in the accent color
#[test]
fn render_dialog_paints_a_rounded_border_in_the_accent_color() {
    // @step Given an FspecDialog with accent Yellow, title "Test", a single body row "hello", and footer "esc"
    let dialog = FspecDialog {
        accent: Accent::Yellow,
        title: "Test",
        rows: vec![DialogRow {
            spans: vec![Span::raw("hello".to_string())],
            selectable: false,
            selected: false,
        }],
        footer: "esc",
        min_width: 40,
    };

    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_to_buffer(&dialog);
    let rect = dialog_rect(Rect::new(0, 0, 80, 24), &dialog);

    // @step Then the top-left cell of the dialog rect contains "╭"
    assert_eq!(buf[(rect.x, rect.y)].symbol(), "╭");

    // @step And the top-right cell contains "╮"
    assert_eq!(buf[(rect.x + rect.width - 1, rect.y)].symbol(), "╮");

    // @step And the bottom-left cell contains "╰"
    assert_eq!(buf[(rect.x, rect.y + rect.height - 1)].symbol(), "╰");

    // @step And the bottom-right cell contains "╯"
    assert_eq!(
        buf[(rect.x + rect.width - 1, rect.y + rect.height - 1)].symbol(),
        "╯"
    );

    // @step And the border cells have foreground color Color::Yellow
    assert_eq!(buf[(rect.x, rect.y)].fg, Color::Yellow);
    assert_eq!(buf[(rect.x + rect.width - 1, rect.y)].fg, Color::Yellow);
}

/// Scenario: render_dialog paints an opaque black background over the dialog rect
#[test]
fn render_dialog_paints_an_opaque_black_background_over_the_dialog_rect() {
    // @step Given an FspecDialog with accent Cyan, title "Test", a single body row "hello", and footer ""
    let dialog = FspecDialog {
        accent: Accent::Cyan,
        title: "Test",
        rows: vec![DialogRow {
            spans: vec![Span::raw("hello".to_string())],
            selectable: false,
            selected: false,
        }],
        footer: "",
        min_width: 40,
    };

    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_to_buffer(&dialog);
    let rect = dialog_rect(Rect::new(0, 0, 80, 24), &dialog);

    // @step Then every cell inside the dialog rect has background color Color::Black
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            assert_eq!(
                buf[(x, y)].bg,
                Color::Black,
                "cell ({x},{y}) should have bg=Black"
            );
        }
    }
}

/// Scenario: render_dialog paints the inner title as a bold accent-colored row
#[test]
fn render_dialog_paints_the_inner_title_as_a_bold_accent_colored_row() {
    // @step Given an FspecDialog with accent Yellow, title "Thinking Level", an empty rows vec, and footer ""
    let dialog = FspecDialog {
        accent: Accent::Yellow,
        title: "Thinking Level",
        rows: vec![],
        footer: "",
        min_width: 40,
    };

    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_to_buffer(&dialog);

    // @step Then the first non-padding body row contains the text "Thinking Level"
    let (title_x, title_y) =
        find_text(&buf, "Thinking Level").expect("title text must appear in body");

    // @step And every cell of that text has foreground color Color::Yellow
    for i in 0..("Thinking Level".chars().count() as u16) {
        let cell = &buf[(title_x + i, title_y)];
        assert_eq!(
            cell.fg,
            Color::Yellow,
            "title cell ({},{}) fg should be Yellow",
            title_x + i,
            title_y
        );
        // @step And every cell of that text carries the BOLD modifier
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "title cell ({},{}) should be BOLD",
            title_x + i,
            title_y
        );
    }

    // @step And the title text is NOT rendered into the top border row
    let rect = dialog_rect(Rect::new(0, 0, 80, 24), &dialog);
    assert_ne!(title_y, rect.y, "title must not be on the border row");
}

/// Scenario: render_dialog inserts one blank row between title and body
#[test]
fn render_dialog_inserts_one_blank_row_between_title_and_body() {
    // @step Given an FspecDialog with title "T" and a single body row containing "row0"
    let dialog = FspecDialog {
        accent: Accent::Cyan,
        title: "T",
        rows: vec![DialogRow {
            spans: vec![Span::raw("row0".to_string())],
            selectable: false,
            selected: false,
        }],
        footer: "",
        min_width: 40,
    };

    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_to_buffer(&dialog);

    let (_, title_y) = find_text(&buf, "T").expect("title row exists");
    let (_, row0_y) = find_text(&buf, "row0").expect("body row exists");

    // @step Then the body row immediately after the title row contains only spaces
    // (i.e. row0 is two positions below the title — one blank row between them)
    // @step And the body row two positions after the title contains the text "row0"
    assert_eq!(
        row0_y,
        title_y + 2,
        "row0 must be exactly two rows below the title (1 blank gap)"
    );
}

/// Scenario: render_dialog paints the selected row with the full-width inverse highlight
#[test]
fn render_dialog_paints_the_selected_row_with_the_full_width_inverse_highlight() {
    // @step Given an FspecDialog with accent Yellow and three body rows where index 1 has selected = true
    let rows = vec![
        DialogRow {
            spans: vec![Span::raw("alpha".to_string())],
            selectable: true,
            selected: false,
        },
        DialogRow {
            spans: vec![Span::raw("bravo".to_string())],
            selectable: true,
            selected: true,
        },
        DialogRow {
            spans: vec![Span::raw("charlie".to_string())],
            selectable: true,
            selected: false,
        },
    ];
    let dialog = FspecDialog {
        accent: Accent::Yellow,
        title: "T",
        rows,
        footer: "",
        min_width: 40,
    };

    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_to_buffer(&dialog);

    let (_, alpha_y) = find_text(&buf, "alpha").expect("alpha row exists");
    let (_, bravo_y) = find_text(&buf, "bravo").expect("bravo row exists");
    let (_, charlie_y) = find_text(&buf, "charlie").expect("charlie row exists");

    let rect = dialog_rect(Rect::new(0, 0, 80, 24), &dialog);
    let inner_x_start = rect.x + 1; // inside the border
    let inner_x_end = rect.x + rect.width - 1;

    // @step Then every cell of the row at index 1 has background color Color::Yellow
    for x in inner_x_start..inner_x_end {
        assert_eq!(
            buf[(x, bravo_y)].bg,
            Color::Yellow,
            "selected row cell ({x},{bravo_y}) should have bg=Yellow"
        );
        // @step And every cell of that row has foreground color Color::Black
        assert_eq!(
            buf[(x, bravo_y)].fg,
            Color::Black,
            "selected row cell ({x},{bravo_y}) should have fg=Black"
        );
        // @step And every cell of that row carries the BOLD modifier
        assert!(
            buf[(x, bravo_y)].modifier.contains(Modifier::BOLD),
            "selected row cell ({x},{bravo_y}) should be BOLD"
        );
    }

    // @step And the row at index 0 has the default background
    // (interior cells of unselected rows should NOT have Yellow bg)
    let mid = (inner_x_start + inner_x_end) / 2;
    assert_ne!(
        buf[(mid, alpha_y)].bg,
        Color::Yellow,
        "unselected row index 0 should not carry Yellow bg"
    );

    // @step And the row at index 2 has the default background
    assert_ne!(
        buf[(mid, charlie_y)].bg,
        Color::Yellow,
        "unselected row index 2 should not carry Yellow bg"
    );
}

/// Scenario: label_description_row prefixes selected rows with "▸ " and others with "  "
#[test]
fn label_description_row_prefixes_selected_rows_with_marker_and_others_with_spaces() {
    // @step Given the helper label_description_row("Low", "fast", true)
    let row_sel = label_description_row("Low", "fast", true);
    // @step Then the resulting DialogRow's first span content is "▸ "
    assert_eq!(row_sel.spans[0].content, "▸ ");

    // @step Given the helper label_description_row("Low", "fast", false)
    let row_unsel = label_description_row("Low", "fast", false);
    // @step Then the resulting DialogRow's first span content is "  "
    assert_eq!(row_unsel.spans[0].content, "  ");
}

/// Scenario: label_description_row dims the description text for unselected rows only
#[test]
fn label_description_row_dims_the_description_text_for_unselected_rows_only() {
    // @step Given the helper label_description_row("Low", "fast", false)
    let row_unsel = label_description_row("Low", "fast", false);
    // @step Then the description span carries Modifier::DIM
    // The description is the LAST span (after marker + label + " - ")
    let desc = row_unsel.spans.last().expect("description span exists");
    assert!(
        desc.style.add_modifier.contains(Modifier::DIM),
        "unselected description must carry DIM modifier"
    );

    // @step Given the helper label_description_row("Low", "fast", true)
    let row_sel = label_description_row("Low", "fast", true);
    // @step Then the description span has the default Style (no DIM)
    let desc = row_sel.spans.last().expect("description span exists");
    assert!(
        !desc.style.add_modifier.contains(Modifier::DIM),
        "selected description must NOT carry DIM modifier"
    );
}

/// Scenario: render_dialog paints the footer with Modifier::DIM and centered alignment
#[test]
fn render_dialog_paints_the_footer_with_dim_modifier_and_centered_alignment() {
    // @step Given an FspecDialog with footer "↑↓ Navigate │ Enter Select │ Esc Close"
    let footer = "↑↓ Navigate │ Enter Select │ Esc Close";
    let dialog = FspecDialog {
        accent: Accent::Yellow,
        title: "T",
        rows: vec![DialogRow {
            spans: vec![Span::raw("body".to_string())],
            selectable: false,
            selected: false,
        }],
        footer,
        min_width: 50,
    };

    // @step When I render it onto an 80x24 TestBackend buffer
    let buf = render_to_buffer(&dialog);
    let text = buffer_text(&buf);

    // @step Then the last body row contains the substring "↑↓ Navigate"
    assert!(text.contains("↑↓ Navigate"), "footer text must render");

    // @step And every non-blank cell in that row carries the DIM modifier
    let (footer_x, footer_y) = find_text(&buf, "↑↓ Navigate").expect("footer row found");
    let cell = &buf[(footer_x, footer_y)];
    assert!(
        cell.modifier.contains(Modifier::DIM),
        "footer cell must carry DIM modifier"
    );

    // @step And the text is horizontally centered within the inner body width
    let rect = dialog_rect(Rect::new(0, 0, 80, 24), &dialog);
    let inner_x = rect.x + 1;
    let inner_w = rect.width - 2;
    let footer_len = footer.chars().count() as u16;
    let expected_x = inner_x + (inner_w.saturating_sub(footer_len)) / 2;
    // Allow ±1 column of jitter for rounding
    assert!(
        (footer_x as i32 - expected_x as i32).abs() <= 1,
        "footer must be centered: got x={footer_x}, expected ~{expected_x}"
    );
}

/// Scenario: dialog_rect centers the dialog within the available area
#[test]
fn dialog_rect_centers_the_dialog_within_the_available_area() {
    // @step Given an FspecDialog with content that yields a 50x10 rect on an 80x24 area
    // The exact natural size depends on render geometry; we just assert
    // that with adequate area, the rect is centered.
    let dialog = FspecDialog {
        accent: Accent::Cyan,
        title: "Help",
        rows: (0..4)
            .map(|i| DialogRow {
                spans: vec![Span::raw(format!("row{i}                              "))],
                selectable: false,
                selected: false,
            })
            .collect(),
        footer: "ESC",
        min_width: 46,
    };

    // @step When I call dialog_rect(area, &dialog)
    let area = Rect::new(0, 0, 80, 24);
    let rect = dialog_rect(area, &dialog);

    // @step Then the returned rect has x = (80 - rect.width) / 2
    // @step And the returned rect has y = (24 - rect.height) / 2
    let expected_x = (area.width.saturating_sub(rect.width)) / 2;
    let expected_y = (area.height.saturating_sub(rect.height)) / 2;
    assert_eq!(rect.x, expected_x);
    assert_eq!(rect.y, expected_y);
}

/// Scenario: dialog_rect clamps to the parent area when content exceeds it
#[test]
fn dialog_rect_clamps_to_the_parent_area_when_content_exceeds_it() {
    // @step Given an FspecDialog with content that would yield a 100x30 rect on a 60x20 area
    let dialog = FspecDialog {
        accent: Accent::Cyan,
        title: "T",
        rows: (0..40)
            .map(|i| DialogRow {
                spans: vec![Span::raw(format!(
                    "row{i} this is an extremely wide content line that exceeds the area width"
                ))],
                selectable: false,
                selected: false,
            })
            .collect(),
        footer: "footer",
        min_width: 100,
    };
    let area = Rect::new(0, 0, 60, 20);

    // @step When I call dialog_rect(area, &dialog)
    let rect = dialog_rect(area, &dialog);

    // @step Then the returned rect has width <= 60 and height <= 20
    assert!(rect.width <= 60, "width must clamp to area.width");
    assert!(rect.height <= 20, "height must clamp to area.height");

    // @step And the returned rect remains fully within the area bounds
    assert!(rect.x + rect.width <= area.x + area.width);
    assert!(rect.y + rect.height <= area.y + area.height);
}
