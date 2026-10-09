//! BUG-202 — the mux menu bar's focused-pane view label paints the bar's
//! palette, not a standalone cyan.
//!
//! Feature: spec/features/mux-bar-focused-pane-view-label-paints-the-bar-palette-not-cyan.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert).
//!
//! Harness: App + MockBackend (the `menu004_mux_surface.rs` pattern) —
//! `fresh_app` + `drain_pending` + full-App render into a TestBackend at
//! 240x24. Observation points:
//! - the bar row's cell styles (row 0 of the mux area — the 2-zone bar):
//!   the focused pane's view label must paint BOLD + theme.fg (white),
//!   never a standalone `Color::Cyan` foreground; the inverse-video
//!   selection highlight (bg Cyan / fg Black) stays the only cyan in the
//!   bar row.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    components::menu_bar::DisplayCell, App, FspecBackend, MuxConfig, MuxOrientation, MuxPaneKind,
    ViewMode,
};
use codelet_rpc_types::{SessionId, SessionStatus, WorkUnitInfo};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

mod common;
use common::MockBackend;

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

/// Render the full App into a 240x24 buffer (the `menu004_mux_surface.rs`
/// geometry: bar on row 0, panes below, footer on the last row).
fn render_app(app: &mut App) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(240, 24)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Enter mux with the given pane list (the `menu004_mux_surface.rs`
/// `enable_mux` shape — no lazy panes here, so no load wiring needed).
fn enable_mux(app: &mut App, panes: Vec<MuxPaneKind>) {
    let n = panes.len();
    let config = MuxConfig {
        orientation: MuxOrientation::Horizontal,
        splits: vec![50; n.saturating_sub(1)],
        panes,
        focused_pane: 0,
        enabled: true,
    };
    let nav = app.navigator_mut();
    nav.mux.enable_with_config(config, ViewMode::Board);
    nav.active_view = ViewMode::Mux;
}

fn focus_pane(app: &mut App, idx: usize) {
    app.navigator_mut().mux.set_focus(idx);
}

/// Seed `n` open sessions (s-1..s-n, Idle by default).
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    drain_pending(app).await;
}

fn set_status(app: &mut App, sid: &str, status: SessionStatus) {
    app.dispatch(codelet_fspec_tui::Action::SessionStatusChanged(
        SessionId::new(sid),
        status,
    ));
}

async fn seed_units(app: &mut App) {
    app.dispatch(codelet_fspec_tui::Action::WorkUnitsLoaded(vec![
        make_unit("AUTH-001", "backlog"),
        make_unit("AUTH-002", "testing"),
        make_unit("AUTH-003", "blocked"),
    ]));
    drain_pending(app).await;
}

fn make_unit(id: &str, status: &str) -> WorkUnitInfo {
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

// ─────────────────────────────────────────────────────────────────────────
// BUG-202 helpers
// ─────────────────────────────────────────────────────────────────────────

/// The mux bar's row 0 (the top row of the 240x24 render — the bar owns
/// it whenever the height allows the 2-zone layout).
const BAR_ROW: u16 = 0;

/// True iff the cell at (x, y) paints a cyan FOREGROUND (the BUG-202
/// regression: the focused pane label's standalone `fg(Color::Cyan)`).
/// The inverse-video highlight carries cyan on the BACKGROUND — that is
/// the legitimate one and is checked separately.
fn fg_cyan(buf: &Buffer, x: u16, y: u16) -> bool {
    buf[(x, y)].fg == Color::Cyan
}

/// The `DisplayCell::View` hit rect whose label is `needle`, from the
/// cached bar geometry (display order — the non-agent panes first).
fn view_label_rect(buf: &Buffer, app: &App, needle: &str) -> (u16, u16, u16) {
    let row = row_text(buf, BAR_ROW);
    let byte_idx = row
        .find(needle)
        .unwrap_or_else(|| panic!("the {needle:?} view label must paint on the bar row: {row}"));
    let x = row[..byte_idx].chars().count() as u16;
    let cells = app
        .navigator()
        .mux
        .menu_cells()
        .expect("a painted bar caches its Zone B cells");
    for (cell, rect) in cells {
        if matches!(cell, DisplayCell::View { label, .. } if *label == needle) {
            return (rect.x, rect.y, rect.width);
        }
    }
    // Fallback (degenerate geometry): the painted label's own span.
    (x, 0, needle.len() as u16)
}

/// The chip's painted cell span (display order: the view labels first,
/// then the chips in global session order).
fn chip_cell_span(_buf: &Buffer, app: &App, chip_index: usize) -> (u16, u16) {
    let cells = app
        .navigator()
        .mux
        .menu_cells()
        .expect("a painted bar caches its Zone B cells");
    let labels = app.navigator().mux.view_label_count();
    let cell_no = labels + chip_index;
    let (cell, rect) = cells
        .get(cell_no)
        .unwrap_or_else(|| panic!("chip cell {chip_index} must be cached (labels {labels}, cell index {cell_no})"));
    assert!(
        matches!(cell, DisplayCell::Chip { .. }),
        "cell {cell_no} must be a chip: {cell:?}"
    );
    (rect.x, rect.width)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The focused pane's view label paints bold white instead of cyan
#[tokio::test]
async fn scenario_the_focused_panes_view_label_paints_bold_white_instead_of_cyan() {
    // @step Given the mux grid is [Board | Files | Ckpts] with no open sessions and the Board pane is focused
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ],
    );
    // `enable_mux` seeds `focused_pane: 0` — the Board pane.

    // @step When the mux view renders
    let buf = render_app(&mut app);
    let row = row_text(&buf, BAR_ROW);

    // @step Then the '[ Board ]' label paints bold with the primary foreground (theme.fg — white)
    let (x, _, width) = view_label_rect(&buf, &app, "Board");
    for cx in x..x + width {
        let cell = &buf[(cx, BAR_ROW)];
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "the focused pane's view label cell {cx} must be bold: {row}"
        );
        assert_eq!(
            cell.fg,
            Color::White,
            "the focused pane's view label cell {cx} must paint the primary foreground (theme.fg), not cyan: {row}"
        );
        assert!(
            !fg_cyan(&buf, cx, BAR_ROW),
            "the focused pane's view label cell {cx} must never carry a cyan foreground: {row}"
        );
    }

    // @step And the '[ Files ]' and '[ Ckpts ]' labels paint the dimmed foreground (theme.dim)
    for label in ["Files", "Ckpts"] {
        let (x, _, width) = view_label_rect(&buf, &app, label);
        for cx in x..x + width {
            let cell = &buf[(cx, BAR_ROW)];
            assert_eq!(
                cell.fg,
                Color::DarkGray,
                "the inactive view label {label:?} cell {cx} must paint the dimmed foreground (theme.dim): {row}"
            );
        }
    }

    // @step And no text cell in the top menu bar row carries a cyan foreground
    for x in 0..buf.area.width {
        assert!(
            !fg_cyan(&buf, x, BAR_ROW),
            "bar-row cell {x} unexpectedly carries a cyan foreground: {row}"
        );
    }
}

/// Scenario: The active view label and the selected-chip highlight stay distinguishable
#[tokio::test]
async fn scenario_the_active_view_label_and_the_selected_chip_highlight_stay_distinguishable() {
    // @step Given the mux grid is [Board | Agent] with 1 open session and the Agent pane is focused showing session #1
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    enable_mux(&mut app, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    focus_pane(&mut app, 1); // the Agent pane — it shows session s-1

    // @step When the mux view renders
    let buf = render_app(&mut app);
    let row = row_text(&buf, BAR_ROW);

    // @step Then chip '[ #1 ● ]' carries the selected-item highlight (cyan background, black text)
    let (chip_x, chip_w) = chip_cell_span(&buf, &app, 0);
    for cx in chip_x..chip_x + chip_w {
        assert_eq!(
            buf[(cx, BAR_ROW)].bg,
            Color::Cyan,
            "chip cell {cx} must carry the selected-item (inverse) highlight background: {row}"
        );
        assert_eq!(
            buf[(cx, BAR_ROW)].fg,
            Color::Black,
            "chip cell {cx} must read black (inverse-video fg): {row}"
        );
    }

    // @step And the inactive '[ Board ]' view label stays dimmed (theme.dim) on the bar background
    let (x, _, width) = view_label_rect(&buf, &app, "Board");
    for cx in x..x + width {
        assert_eq!(
            buf[(cx, BAR_ROW)].fg,
            Color::DarkGray,
            "the inactive view label cell {cx} must stay dimmed (the Agent pane is focused): {row}"
        );
        assert_ne!(
            buf[(cx, BAR_ROW)].bg,
            Color::Cyan,
            "the inactive view label cell {cx} keeps the bar background (no inverse highlight): {row}"
        );
    }

    // @step And no text cell in the top menu bar row carries a cyan foreground outside the selected chip's cells
    for x in 0..buf.area.width {
        if x >= chip_x && x < chip_x + chip_w {
            continue; // the selected chip's own cells (inverse-video fg Black)
        }
        assert!(
            !fg_cyan(&buf, x, BAR_ROW),
            "bar-row cell {x} unexpectedly carries a cyan foreground outside the selected chip: {row}"
        );
    }
}

/// Scenario: Everything else on the bar keeps its existing styling
#[tokio::test]
async fn scenario_everything_else_on_the_bar_keeps_its_existing_styling() {
    // @step Given the mux grid is [Board | Agent | Agent] with 1 Running and 1 Idle session and the Board pane is focused
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    set_status(&mut app, "s-1", SessionStatus::Running);
    drain_pending(&mut app).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    // `enable_mux` seeds `focused_pane: 0` — the Board pane.

    // @step When the mux view renders
    let buf = render_app(&mut app);
    let row = row_text(&buf, BAR_ROW);

    // @step Then the Zone A items and the '[ New Agent ]'-free row paint exactly as before (white items, dim separator)
    // The mux bar paints no Zone C (MENU-009 Q1) — no New Agent / Close
    // Agent buttons on this row.
    assert!(
        !row.contains("New Agent") && !row.contains("Close Agent"),
        "the mux bar paints no Zone C buttons: {row}"
    );
    // The Zone A items paint the primary foreground (white) at full width.
    let kanban_x = row
        .find("Kanban")
        .expect("the Kanban item must paint: {row}");
    let kx = row[..kanban_x].chars().count() as u16;
    assert_eq!(
        buf[(kx, BAR_ROW)].fg,
        Color::White,
        "the Kanban item must keep its primary (white) foreground: {row}"
    );
    // The dim `│` separator between the zones.
    let sep_x = row
        .find('│')
        .expect("the zone separator must paint: {row}");
    let sx = row[..sep_x].chars().count() as u16;
    assert_eq!(
        buf[(sx, BAR_ROW)].fg,
        Color::DarkGray,
        "the separator must stay dimmed: {row}"
    );

    // @step And the Running chip's braille glyph stays magenta and the Idle chip's dot stays green-dimmed
    // The chip's glyph is the ONLY cell of the chip span painted in its
    // status color (brackets/prefix/bracket-gap use the cell fg) — scan
    // the span for it instead of hardcoding the bracketed offset.
    let (running_x, running_w) = chip_cell_span(&buf, &app, 0);
    let running_magenta = (running_x..running_x + running_w)
        .filter(|cx| buf[(*cx, BAR_ROW)].fg == Color::Magenta)
        .count();
    assert_eq!(
        running_magenta,
        1,
        "exactly one Running chip cell must be magenta (the braille glyph): {row}"
    );
    let (idle_x, idle_w) = chip_cell_span(&buf, &app, 1);
    let idle_green: Vec<u16> = (idle_x..idle_x + idle_w)
        .filter(|cx| buf[(*cx, BAR_ROW)].fg == Color::Green)
        .collect();
    assert_eq!(
        idle_green.len(),
        1,
        "exactly one Idle chip cell must be green (the dot glyph): {row}"
    );
    assert!(
        buf[(idle_green[0], BAR_ROW)]
            .modifier
            .contains(Modifier::DIM),
        "the Idle chip's dot must stay dimmed: {row}"
    );

    // @step And the focused pane's '[ Board ]' label is bold white — the only view-label change
    let (x, _, width) = view_label_rect(&buf, &app, "Board");
    for cx in x..x + width {
        let cell = &buf[(cx, BAR_ROW)];
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "the focused pane's view label cell {cx} must be bold: {row}"
        );
        assert_eq!(
            cell.fg,
            Color::White,
            "the focused pane's view label cell {cx} must paint the primary foreground: {row}"
        );
    }
}
