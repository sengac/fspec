//! MUX-005 — mux footer bar styling: white foreground on a dark purple
//! background across the full footer row.
//!
//! Feature: spec/features/mux-footer-bar-styling.feature
//!
//! The mux footer (views/multiplex/render.rs::paint_footer) must paint the
//! ENTIRE last row with a dark purple background (RGB 74, 44, 112) and
//! white foreground text; single-view rendering (mux off) must remain
//! free of that background (R10 from rust-mux-mode.feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_rpc_types::WorkUnitInfo;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

use codelet_fspec_tui::store::{AgentViewStore, BoardStore};
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::{Navigator, ViewMode};

/// MUX-005: dark purple footer background (spec: RGB 74, 44, 112).
const MUX_FOOTER_PURPLE: Color = Color::Rgb(74, 44, 112);

fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn fresh() -> (Navigator, tokio::sync::mpsc::UnboundedReceiver<codelet_fspec_tui::components::Action>) {
    let (tx, rx) = unbounded_channel();
    (Navigator::new(Arc::new(Theme::default()), tx), rx)
}

/// MUX-001: enable the default preset on the Navigator's mux layout.
fn enable_default(nav: &mut Navigator) {
    nav.mux.enable_default();
    nav.active_view = ViewMode::Mux;
}

/// MUX-002: an agent slot only renders when a session is open — seed one
/// into the agent store so the agent pane is visible in the grid.
fn seed_agent_session(agent: &mut AgentViewStore) {
    agent.append_session(codelet_fspec_tui::store::SessionContext::new(
        codelet_rpc_types::SessionId::new("s-1"),
    ));
}

/// Render the Navigator into a 120x24 test buffer and return the buffer.
fn render_buffer(
    nav: &mut Navigator,
    board: &BoardStore,
    agent: &mut AgentViewStore,
) -> ratatui::buffer::Buffer {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| {
        nav.render_with_stores(frame.area(), frame.buffer_mut(), board, agent);
    })
    .expect("draw");
    term.backend().buffer().clone()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The mux footer bar paints white text on a dark purple
// background across the full row
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The mux footer bar paints white text on a dark purple background across the full row
#[test]
fn footer_bar_paints_white_text_on_dark_purple_across_the_full_row() {
    // @step Given mux mode is active with the default two-pane grid (Board | Agent)
    let (mut nav, _rx) = fresh();
    enable_default(&mut nav);
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    seed_agent_session(&mut agent);
    // @step When the mux frame is rendered to a terminal buffer
    let buf = render_buffer(&mut nav, &board, &mut agent);
    let footer_row = buf.area.height - 1;
    // @step Then the footer row contains the footer label "MUX 2 panes [Board|Agent]"
    let row_text: String = (0..buf.area.width)
        .map(|x| buf[(x, footer_row)].symbol().to_string())
        .collect();
    assert!(
        row_text.contains("MUX 2 panes [Board|Agent]"),
        "footer label missing in footer row: {row_text:?}"
    );
    // @step And every label cell in the footer row uses white foreground on a dark purple background (RGB 74, 44, 112)
    for x in 0..buf.area.width {
        let cell = &buf[(x, footer_row)];
        assert_eq!(
            cell.bg,
            MUX_FOOTER_PURPLE,
            "footer cell ({x}, {footer_row}) must carry the dark purple background; got {:?}",
            cell.bg
        );
        if cell.symbol() != " " {
            assert_eq!(
                cell.fg,
                Color::White,
                "footer label cell ({x}, {footer_row}) must be white foreground; got {:?}",
                cell.fg
            );
        }
    }
    // @step And every cell from the end of the label to the right terminal edge is also dark purple (no terminal-background gap)
    let last_glyph = (0..buf.area.width)
        .rev()
        .find(|&x| buf[(x, footer_row)].symbol() != " ")
        .expect("footer label must be painted");
    assert!(
        last_glyph + 1 < buf.area.width,
        "the footer label must end before the right edge so a tail region exists"
    );
    for x in last_glyph + 1..buf.area.width {
        let cell = &buf[(x, footer_row)];
        assert_eq!(
            cell.bg,
            MUX_FOOTER_PURPLE,
            "tail cell ({x}, {footer_row}) must stay dark purple (solid bar, no terminal-background gap); got {:?}",
            cell.bg
        );
        assert_eq!(
            cell.symbol(),
            " ",
            "tail cell ({x}, {footer_row}) must be a plain space; got {:?}",
            cell.symbol()
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: No footer bar is painted when mux is off
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: No footer bar is painted when mux is off
#[test]
fn no_footer_bar_is_painted_when_mux_is_off() {
    // @step Given the TUI is in a single-view mode (Board or Agent) with mux disabled
    let (mut nav, _rx) = fresh();
    assert_eq!(nav.active_view, ViewMode::Board);
    assert!(!nav.mux.config().enabled, "mux must start disabled");
    let mut board = BoardStore::default();
    board.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut agent = AgentViewStore::default();
    // @step When the frame is rendered to a terminal buffer
    let buf = render_buffer(&mut nav, &board, &mut agent);
    // @step Then no footer bar is painted at the bottom of the screen
    let last_row: String = (0..buf.area.width)
        .map(|x| buf[(x, buf.area.height - 1)].symbol().to_string())
        .collect();
    assert!(
        !last_row.contains("MUX"),
        "no mux footer bar may be painted when mux is off: {last_row:?}"
    );
    // @step And no cell in the buffer has the dark purple background
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            assert_ne!(
                buf[(x, y)].bg,
                MUX_FOOTER_PURPLE,
                "cell ({x}, {y}) must not carry the mux footer background when mux is off"
            );
        }
    }
}
