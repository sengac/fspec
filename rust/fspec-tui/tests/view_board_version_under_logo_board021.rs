//! BOARD-021 — BoardView header strip: build version painted under the logo.
//!
//! Feature: spec/features/display-fspec-version-under-the-board-logo.feature
//!
//! Drives `BoardView::render_with_store` against a `TestBackend` and
//! asserts the 4th logo row (previously blank) carries the compile-time
//! version string `v{CARGO_PKG_VERSION}` in the theme's dim color, while
//! the 3 glyph rows and the right-hand header widgets are unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{BoardStore, BoardView, Theme};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

/// The compile-time version string the logo's 4th row must paint.
const VERSION_LINE: &str = concat!("v", env!("CARGO_PKG_VERSION"));

fn fresh() -> (
    BoardView,
    tokio::sync::mpsc::UnboundedReceiver<codelet_fspec_tui::Action>,
) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

/// Render the board against a `width`x`height` TestBackend and return the
/// buffer plus the x position of the logo block's left edge.
fn render(width: u16, height: u16, store: &BoardStore) -> (Buffer, u16) {
    let (view, _rx) = fresh();
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    // Header layout: row 0 = top border, rows 1..=4 = 4-row header strip.
    // The logo block spans x=2..14 (after the left `│` border at x=0 and
    // the paddingX=1 cell at x=1 — see header.rs).
    (buf, 2)
}

/// Read row `y` of the buffer as a String, one char per cell.
fn row_string(buf: &Buffer, y: u16) -> String {
    let mut row = String::with_capacity(buf.area.width as usize);
    for x in 0..buf.area.width {
        row.push_str(buf[(x, y)].symbol());
    }
    row
}

/// Find `needle` in `row` and return its **cell (char) index** — NOT the
/// byte offset that `str::find` returns. Each buffer cell holds exactly
/// one char, so the char index is the buffer x coordinate. Returns None
/// when the needle is absent.
fn find_cell(row: &str, needle: &str) -> Option<usize> {
    let needle_chars: Vec<char> = needle.chars().collect();
    row.chars()
        .collect::<Vec<char>>()
        .windows(needle_chars.len())
        .position(|w| w == needle_chars.as_slice())
}

/// Expected centered start of the version string: block left edge (x=2)
/// plus the left padding `((12 - len) / 2)`.
fn expected_version_x() -> usize {
    2 + ((12 - VERSION_LINE.len()) / 2)
}

/// Scenario: Board header paints the build version on the 4th logo row
#[test]
fn board_header_paints_the_build_version_on_the_4th_logo_row() {
    // @step Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    let store = BoardStore::default();
    // @step When the App renders BoardView against a 120x24 TestBackend
    let (buf, logo_x) = render(120, 24, &store);
    // @step Then the 4th row of the header strip (the row that also carries the keybinding chord) contains the substring "v" + env!("CARGO_PKG_VERSION")
    let row4 = row_string(&buf, 4);
    assert!(
        row4.contains(VERSION_LINE),
        "4th header row must contain `{VERSION_LINE}`; got `{row4}`"
    );
    // @step And that substring is centered within the 12-cell logo block, mirroring the centered glyph rows above it
    let glyph_row1 = row_string(&buf, 1);
    let glyph_x = find_cell(&glyph_row1, "┏").expect("row 1 must contain the ┏ glyph");
    assert_eq!(
        glyph_x, logo_x as usize,
        "logo left edge drifted (glyph at x={glyph_x}, helper says {logo_x})"
    );
    let version_x = find_cell(&row4, VERSION_LINE).expect("version substring found above");
    assert_eq!(
        version_x,
        expected_version_x(),
        "version must be centered in the 12-cell logo block (x={}); got x={version_x}",
        expected_version_x()
    );
}

/// Scenario: Logo glyph rows are unchanged when the version row is painted
#[test]
fn logo_glyph_rows_are_unchanged_when_the_version_row_is_painted() {
    // @step Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    let store = BoardStore::default();
    // @step When the App renders BoardView against a 120x24 TestBackend
    let (buf, _logo_x) = render(120, 24, &store);
    // @step Then the rendered buffer contains the substring "┏┓┏┓┏┓┏┓┏┓"
    let row1 = row_string(&buf, 1);
    assert!(
        row1.contains("┏┓┏┓┏┓┏┓┏┓"),
        "missing logo glyph row 1:\n{row1}"
    );
    // @step And the rendered buffer contains the substring "┣ ┗┓┃┃┣ ┃"
    let row2 = row_string(&buf, 2);
    assert!(
        row2.contains("┣ ┗┓┃┃┣ ┃"),
        "missing logo glyph row 2:\n{row2}"
    );
    // @step And the rendered buffer contains the substring "┻ ┗┛┣┛┗┛┗┛"
    let row3 = row_string(&buf, 3);
    assert!(
        row3.contains("┻ ┗┛┣┛┗┛┗┛"),
        "missing logo glyph row 3:\n{row3}"
    );
    // @step And the rendered buffer contains the substring "Checkpoints: None"
    assert!(
        row1.contains("Checkpoints: None"),
        "missing 'Checkpoints: None' on the checkpoint row:\n{row1}"
    );
    // @step And the rendered buffer contains the substring "C Checkpoints"
    let row4 = row_string(&buf, 4);
    assert!(
        row4.contains("C Checkpoints"),
        "missing keybinding chord:\n{row4}"
    );
}

/// Scenario: The version row is styled with the theme's dim color
#[test]
fn the_version_row_is_styled_with_the_themes_dim_color() {
    // @step Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    let store = BoardStore::default();
    // @step And the Theme is the default dark variant
    let theme = Theme::default();
    assert_eq!(
        theme.dim,
        Color::DarkGray,
        "precondition: default theme dim color"
    );
    // @step When the App renders BoardView against a 120x24 TestBackend
    let (buf, _logo_x) = render(120, 24, &store);
    // @step Then the buffer cells spelling the version string on the 4th logo row carry the theme's dim foreground color
    let row4 = row_string(&buf, 4);
    let version_x =
        find_cell(&row4, VERSION_LINE).expect("version substring must be present (see scenario 1)");
    for offset in 0..VERSION_LINE.len() {
        let cell = &buf[(version_x as u16 + offset as u16, 4)];
        assert_eq!(
            cell.fg,
            theme.dim,
            "version cell x={} y=4 must be dim; got fg={:?}",
            version_x + offset,
            cell.fg
        );
    }
    // @step And the buffer cells spelling the 3 logo glyph rows carry the default (non-dim) foreground color
    for (y, glyph) in [(1u16, "┏┓"), (2, "┣ ┗"), (3, "┻ ┗")] {
        let row = row_string(&buf, y);
        let x = find_cell(&row, glyph)
            .unwrap_or_else(|| panic!("glyph `{glyph}` missing on row {y}:\n{row}"));
        for offset in 0..glyph.chars().count() {
            let cell = &buf[(x as u16 + offset as u16, y)];
            assert_ne!(
                cell.fg,
                theme.dim,
                "glyph cell x={} y={y} must NOT be dim; got fg={:?}",
                x + offset,
                cell.fg
            );
        }
    }
}

/// Scenario: The version text never overflows the 12-cell logo block
#[test]
fn the_version_text_never_overflows_the_12_cell_logo_block() {
    // @step Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    let store = BoardStore::default();
    // @step When the App renders BoardView against a 120x24 TestBackend
    let (buf, _logo_x) = render(120, 24, &store);
    // @step Then the version string occupies at most 12 cells starting at the logo's left edge
    // (logo block: x=2..14, i.e. 12 cells after the left border + paddingX)
    let row4 = row_string(&buf, 4);
    let version_x =
        find_cell(&row4, VERSION_LINE).expect("version substring must be present (see scenario 1)");
    assert!(
        version_x + VERSION_LINE.len() <= 2 + 12,
        "version must fit inside the 12-cell logo block (x {version_x} + {} cells); right edge would be x={}",
        VERSION_LINE.len(),
        version_x + VERSION_LINE.len()
    );
    // @step And the keybinding chord on the 4th header row begins at the same x position as before (right after the 12-cell logo block)
    let chord_x = find_cell(&row4, "C Checkpoints").expect("keybinding chord must be present");
    assert_eq!(
        chord_x, 14,
        "chord must start right after the 12-cell logo block (x=14); got x={chord_x}"
    );
}
