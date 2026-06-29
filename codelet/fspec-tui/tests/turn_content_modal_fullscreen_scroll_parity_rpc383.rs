// Feature: spec/features/agentview-turn-content-modal-fullscreen-scroll.feature
//
//! RPC-383 — `TurnContentModal` full-screen + scrollable parity.
//!
//! Feature: spec/features/agentview-turn-content-modal-fullscreen-scroll.feature
//!
//! Brings the Rust port of `TurnContentModal` to parity with the
//! TypeScript reference (`src/tui/components/TurnContentModal.tsx`):
//!
//!   1. SIZING — the modal must paint at a FIXED rect of
//!      `area.width - 4` × `area.height - 6` (centered), independent of
//!      content length. Today it shrinks-to-content (`min_width: 0` /
//!      natural height) — these tests assert the fixed full-screen rect.
//!   2. SCROLLING — the modal must support a scroll offset (reset to 0
//!      on open), Up/Down (1 row), PageUp/PageDown (page), Home/End
//!      (top/bottom), the mouse wheel, a visible scrollbar (reusing
//!      `scrollback_paint::paint_scrollbar`), and a dim centered footer
//!      `↑↓ Scroll | Esc Close`. Scrolling must NOT move the underlying
//!      turn selection (`selected_seq`).
//!
//! These tests target the INTENDED public surface and are EXPECTED to
//! FAIL until RPC-383 lands — the correct red state for ACDD.
//!
//! ── ASSUMED NEW PUBLIC API (to be confirmed by the supervisor) ────────
//!
//! • `AgentView.turn_modal_offset: usize`
//!     The modal body's first-visible visual-row index. Mirrors
//!     `turn_modal_seq`; reset to 0 in `App::handle_open_turn_modal`.
//!     Read in tests via `app.navigator().agent.turn_modal_offset`.
//!
//! • New `Action` variants, reduced on the App task to mutate
//!   `turn_modal_offset` (clamped so the last page is fully visible),
//!   emitted by `views/agent/dispatch_select.rs` while the modal is open:
//!       Action::TurnModalScrollUp      (Up   → -1 row)
//!       Action::TurnModalScrollDown    (Down → +1 row)
//!       Action::TurnModalPageUp        (PageUp)
//!       Action::TurnModalPageDown      (PageDown)
//!       Action::TurnModalHome          (Home → offset 0)
//!       Action::TurnModalEnd           (End  → last page)
//!   The mouse wheel over the open modal routes ScrollUp/ScrollDown into
//!   TurnModalScrollUp/Down (mirroring scrollback wheel handling).
//!
//! • `TurnContentModal` rendering changes (asserted via the buffer):
//!     - fixed rect `area.width-4 × area.height-6`, centered;
//!     - dim centered footer `↑↓ Scroll | Esc Close`;
//!     - single-column scrollbar (`■`/`│`) when content overflows;
//!     - body windowed by `turn_modal_offset` (no silent clipping).
//!
//! NOTE: these tests drive the modal END-TO-END through `App` (open via
//! Tab+Enter in SELECT mode, then key/wheel events) so they exercise the
//! real dispatch + reducer wiring, mirroring `turn_content_modal_rpc382`
//! and `scrollback_scroll_rpc094`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::views::agent::rendered_chunk::ChunkSource;
use codelet_fspec_tui::{Action, App, ChunkKind, FspecBackend, RenderedChunk, ViewMode};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui::Terminal;

mod common;
use common::MockBackend;

// ─────────────────────────────────────────────────────────────────────
// Local helpers (kept inline; the RPC-382 helpers are not re-exported
// across test crates, and these need a LONG-body turn fixture).
// ─────────────────────────────────────────────────────────────────────

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn tab() -> Event {
    key(KeyCode::Tab)
}

fn wheel(kind: MouseEventKind, col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Push one turn carrying a real `ChunkSource` so the modal has a full
/// body to render.
fn push_text_turn(app: &mut App, id: &SessionId, seq: u64, text: &str) {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(id)
        .expect("SessionContext present");
    ctx.scrollback.push(RenderedChunk {
        seq,
        lines: text.lines().map(|l| Line::from(l.to_string())).collect(),
        source: Some(ChunkSource {
            text: text.to_string(),
            color: Color::White,
            kind: ChunkKind::AssistantText,
            is_streaming: false,
        }),
    });
}

/// Drain the App's action bus, dispatching each queued Action back into
/// the App so reducer side-effects run synchronously.
fn drain(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

fn selected_seq(app: &App, id: &SessionId) -> Option<u64> {
    app.agent_view_store()
        .session_context_for(id)
        .and_then(|c| c.scrollback.selected_seq())
}

fn modal_seq(app: &App) -> Option<u64> {
    app.navigator().agent.turn_modal_seq
}

/// ASSUMED NEW API: the modal body's scroll offset (first visible row).
fn modal_offset(app: &App) -> usize {
    app.navigator().agent.turn_modal_offset
}

/// Render the App and return the rows of glyphs.
fn render_rows(app: &mut App, w: u16, h: u16) -> Vec<String> {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    (0..buf.area.height)
        .map(|y| {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            row
        })
        .collect()
}

fn joined(rows: &[String]) -> String {
    rows.join("\n")
}

/// Column (char index, NOT byte offset) of the first `ch` in `row`. The
/// rendered buffer rows contain multibyte glyphs (●, ↑↓, box-drawing),
/// so `str::find` byte offsets do not equal terminal columns.
fn char_col(row: &str, ch: char) -> Option<usize> {
    row.chars().position(|c| c == ch)
}

/// Extract ONLY the modal's interior text (everything between the first
/// `╭`/`│`/`╰` border column and the matching right border on each row),
/// joined with newlines. The modal is a full-screen overlay, but the
/// underlying scrollback still paints in the margin columns to the left
/// and right of the modal border; scoping to the interior prevents those
/// background rows (e.g. a stray `LASTLINE` from the scrollback) from
/// polluting modal-content assertions.
fn modal_interior(rows: &[String]) -> String {
    let mut out: Vec<String> = Vec::new();
    for row in rows {
        let chars: Vec<char> = row.chars().collect();
        let left = chars
            .iter()
            .position(|c| matches!(c, '\u{256D}' | '\u{2502}' | '\u{2570}'));
        let right = chars
            .iter()
            .rposition(|c| matches!(c, '\u{256E}' | '\u{2502}' | '\u{256F}'));
        if let (Some(l), Some(r)) = (left, right) {
            if r > l {
                out.push(chars[l + 1..r].iter().collect());
            }
        }
    }
    out.join("\n")
}

/// An App in `ViewMode::Agent` with one session seeded with three short
/// turns (FIRSTBODY / SECONDBODY / THIRDBODY).
fn agent_app_with_short_turns() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    push_text_turn(&mut app, &sid("s-1"), 0, "first FIRSTBODY");
    push_text_turn(&mut app, &sid("s-1"), 1, "second SECONDBODY");
    push_text_turn(&mut app, &sid("s-1"), 2, "third THIRDBODY");
    app
}

/// An App whose three turns are all short EXCEPT the last, whose body is
/// a long numbered list (`LINE000`..`LINE099`) plus distinct
/// `TOPLINE` / `LASTLINE` sentinels at the extremes. Used for the
/// scrolling scenarios.
fn agent_app_with_long_last_turn() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    push_text_turn(&mut app, &sid("s-1"), 0, "first turn");
    push_text_turn(&mut app, &sid("s-1"), 1, "second turn");
    push_text_turn(&mut app, &sid("s-1"), 2, &long_body());
    app
}

/// An App whose three turns are all long, so the modal overflows on ANY
/// selected turn. Used by the Down-scroll scenario where the 2nd turn is
/// the one opened.
fn agent_app_with_long_turns() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    push_text_turn(&mut app, &sid("s-1"), 0, &long_body());
    push_text_turn(&mut app, &sid("s-1"), 1, &long_body());
    push_text_turn(&mut app, &sid("s-1"), 2, &long_body());
    app
}

/// A 100+-line body with `TOPLINE` / `LASTLINE` sentinels at the extremes.
fn long_body() -> String {
    let mut long = String::from("TOPLINE\n");
    for i in 0..100 {
        long.push_str(&format!("LINE{i:03}\n"));
    }
    long.push_str("LASTLINE");
    long
}

/// Open the modal over the LAST turn (auto-selected on entering SELECT
/// mode) and render once so `last_render_area` is cached (the scroll
/// reducer sizes the modal viewport from it).
fn open_modal_on_long_turn(app: &mut App) {
    let _ = app.handle_event(&tab()); // enter SELECT mode, auto-select last turn
    drain(app);
    let _ = app.handle_event(&key(KeyCode::Enter)); // open modal
    drain(app);
    let _ = render_rows(app, 60, 20);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: The modal fills the screen regardless of short content
// ─────────────────────────────────────────────────────────────────────

#[test]
fn the_modal_fills_the_screen_regardless_of_short_content() {
    // @step Given a turn content modal showing a 3-line turn on a 40x12 terminal
    let mut app = agent_app_with_short_turns();
    let _ = app.handle_event(&tab());
    drain(&mut app);
    let _ = app.handle_event(&key(KeyCode::Enter)); // open modal on last (short) turn
    drain(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open");

    // @step When the modal is rendered
    let rows = render_rows(&mut app, 40, 12);

    // @step Then the modal occupies 36 columns and 6 rows
    // Fixed rect = area.width-4 (36) x area.height-6 (6), centered:
    //   x = (40-36)/2 = 2 .. 38 ; y = (12-6)/2 = 3 .. 9.
    // The rounded border draws ╭/╮ at the rect corners.
    let top = &rows[3];
    let bottom = &rows[8];
    assert!(
        top.contains('\u{256D}') && top.contains('\u{256E}'),
        "modal top border row must span the fixed rect (╭…╮) at y=3; got: {top:?}"
    );
    assert!(
        bottom.contains('\u{2570}') && bottom.contains('\u{256F}'),
        "modal bottom border row must be at y=8 (6 rows tall); got: {bottom:?}"
    );
    let left_col = char_col(top, '\u{256D}').expect("top-left corner present");
    let right_col = char_col(top, '\u{256E}').expect("top-right corner present");
    assert_eq!(
        left_col, 2,
        "modal left edge must be centered at col 2; got {left_col} in {top:?}"
    );
    assert_eq!(
        right_col, 37,
        "modal right edge must be at col 37 (36 cols wide); got {right_col} in {top:?}"
    );

    // @step And the modal is not shrunk to fit the 3 lines of content
    // A shrink-to-content modal would be only ~9 rows tall and far
    // narrower; the fixed modal is exactly 6 rows (y 3..=8) and 36 wide.
    assert!(
        char_col(&rows[2], '\u{256D}').is_none(),
        "no border above y=3 — modal must not float higher (shrunk); got: {:?}",
        rows[2]
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A scrollbar appears when content overflows the viewport
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_scrollbar_appears_when_content_overflows_the_viewport() {
    // @step Given a turn content modal showing a turn with 100 wrapped lines on a 20-row screen
    let mut app = agent_app_with_long_last_turn();
    open_modal_on_long_turn(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open");

    // @step When the modal is rendered
    let rows = render_rows(&mut app, 60, 20);

    // @step Then a single-column scrollbar is shown in the modal's rightmost column
    // The canonical painter uses ■ (U+25A0, thumb) / │ (U+2502, track).
    let joined = joined(&rows);
    assert!(
        joined.contains('\u{25A0}'),
        "overflowing body must show a scrollbar thumb (■); got: {rows:?}"
    );
    assert!(
        joined.contains('\u{2502}'),
        "overflowing body must show a scrollbar track (│); got: {rows:?}"
    );

    // @step And only the first page of lines is visible
    assert_eq!(
        modal_offset(&app),
        0,
        "freshly opened modal must show the top (offset 0)"
    );
    let interior = modal_interior(&rows);
    assert!(
        interior.contains("TOPLINE"),
        "first page must show the top sentinel; got: {rows:?}"
    );
    assert!(
        !interior.contains("LASTLINE"),
        "the bottom sentinel must NOT be on the first page (content overflows); got: {rows:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Pressing Down scrolls the body without moving the selection
// ─────────────────────────────────────────────────────────────────────

#[test]
fn pressing_down_scrolls_the_body_without_moving_the_selection() {
    // @step Given a turn content modal open over a long turn with the second of three turns selected
    let mut app = agent_app_with_long_turns();
    let id = sid("s-1");
    let _ = app.handle_event(&tab()); // SELECT mode, auto-select last (3rd) turn
    drain(&mut app);
    let _ = app.handle_event(&key(KeyCode::Up)); // last -> 2nd (seq 1)
    drain(&mut app);
    assert_eq!(
        selected_seq(&app, &id),
        Some(1),
        "second turn must be selected"
    );
    let _ = app.handle_event(&key(KeyCode::Enter)); // open modal on seq 1
    drain(&mut app);
    assert_eq!(
        modal_seq(&app),
        Some(1),
        "modal must be open on the 2nd turn"
    );
    // Render so the reducer can size the modal viewport (last_render_area).
    let _ = render_rows(&mut app, 60, 20);
    let offset_before = modal_offset(&app);

    // @step When I press the Down arrow key
    let _ = app.handle_event(&key(KeyCode::Down));
    drain(&mut app);

    // @step Then the visible window advances by one line
    assert_eq!(
        modal_offset(&app),
        offset_before + 1,
        "Down must advance the modal scroll offset by exactly one row"
    );

    // @step And the selected turn is still the second turn
    assert_eq!(
        selected_seq(&app, &id),
        Some(1),
        "scrolling the modal must NOT move the underlying turn selection"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Pressing End scrolls to the bottom of the content
// ─────────────────────────────────────────────────────────────────────

#[test]
fn pressing_end_scrolls_to_the_bottom_of_the_content() {
    // @step Given a turn content modal open over a long turn
    let mut app = agent_app_with_long_last_turn();
    open_modal_on_long_turn(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open");
    let before = render_rows(&mut app, 60, 20);
    assert!(
        !modal_interior(&before).contains("LASTLINE"),
        "precondition: bottom sentinel must be off-screen before End; got: {before:?}"
    );

    // @step When I press the End key
    let _ = app.handle_event(&key(KeyCode::End));
    drain(&mut app);

    // @step Then the modal shows the final line of the body
    let after = render_rows(&mut app, 60, 20);
    assert!(
        modal_interior(&after).contains("LASTLINE"),
        "End must scroll to the bottom, revealing the final line; got: {after:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Pressing PageDown advances the body by a page
// ─────────────────────────────────────────────────────────────────────

#[test]
fn pressing_pagedown_advances_the_body_by_a_page() {
    // @step Given a turn content modal open over a long turn
    let mut app = agent_app_with_long_last_turn();
    open_modal_on_long_turn(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open");
    let _ = render_rows(&mut app, 60, 20);
    let offset_before = modal_offset(&app);

    // @step When I press the PageDown key
    let _ = app.handle_event(&key(KeyCode::PageDown));
    drain(&mut app);

    // @step Then the visible window advances by more than one line
    assert!(
        modal_offset(&app) > offset_before + 1,
        "PageDown must advance the modal scroll offset by more than one row \
         (a page); before={offset_before}, after={}",
        modal_offset(&app)
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Pressing PageUp moves the body back by a page
// ─────────────────────────────────────────────────────────────────────

#[test]
fn pressing_pageup_moves_the_body_back_by_a_page() {
    // @step Given a turn content modal scrolled to the bottom of a long turn
    let mut app = agent_app_with_long_last_turn();
    open_modal_on_long_turn(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open");
    let _ = app.handle_event(&key(KeyCode::End)); // jump to bottom
    drain(&mut app);
    let _ = render_rows(&mut app, 60, 20);
    let offset_before = modal_offset(&app);
    assert!(
        offset_before > 1,
        "precondition: modal must be scrolled well past the top before PageUp; \
         got offset {offset_before}"
    );

    // @step When I press the PageUp key
    let _ = app.handle_event(&key(KeyCode::PageUp));
    drain(&mut app);

    // @step Then the visible window moves back by more than one line
    assert!(
        modal_offset(&app) + 1 < offset_before,
        "PageUp must move the modal scroll offset back by more than one row \
         (a page); before={offset_before}, after={}",
        modal_offset(&app)
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Pressing Home jumps to the top of the content
// ─────────────────────────────────────────────────────────────────────

#[test]
fn pressing_home_jumps_to_the_top_of_the_content() {
    // @step Given a turn content modal scrolled down over a long turn
    let mut app = agent_app_with_long_last_turn();
    open_modal_on_long_turn(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open");
    for _ in 0..5 {
        let _ = app.handle_event(&key(KeyCode::Down));
        drain(&mut app);
    }
    assert!(
        modal_offset(&app) > 0,
        "precondition: modal must be scrolled down before Home; got offset {}",
        modal_offset(&app)
    );

    // @step When I press the Home key
    let _ = app.handle_event(&key(KeyCode::Home));
    drain(&mut app);

    // @step Then the modal shows the first line of the body
    assert_eq!(
        modal_offset(&app),
        0,
        "Home must reset the modal scroll offset to the top"
    );
    let rows = render_rows(&mut app, 60, 20);
    assert!(
        modal_interior(&rows).contains("TOPLINE"),
        "Home must reveal the top sentinel; got: {rows:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: The mouse wheel scrolls the modal body
// ─────────────────────────────────────────────────────────────────────

#[test]
fn the_mouse_wheel_scrolls_the_modal_body() {
    // @step Given a turn content modal open over a long turn
    let mut app = agent_app_with_long_last_turn();
    open_modal_on_long_turn(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open");
    // Render so any layout caches are populated.
    let _ = render_rows(&mut app, 60, 20);
    let offset_before = modal_offset(&app);

    // @step When I scroll the mouse wheel down over the modal
    // The fixed modal is centered in a 60x20 area, so (30, 10) lands
    // inside it.
    let _ = app.handle_event(&wheel(MouseEventKind::ScrollDown, 30, 10));
    drain(&mut app);

    // @step Then the visible window advances
    assert!(
        modal_offset(&app) > offset_before,
        "wheel ScrollDown over the modal must advance the scroll offset; \
         before={offset_before}, after={}",
        modal_offset(&app)
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: The modal shows the scroll/close footer
// ─────────────────────────────────────────────────────────────────────

#[test]
fn the_modal_shows_the_scroll_close_footer() {
    // @step Given a turn content modal open over a long turn
    let mut app = agent_app_with_long_last_turn();
    open_modal_on_long_turn(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open");

    // @step When the modal is rendered
    let rows = render_rows(&mut app, 60, 20);

    // @step Then the modal's bottom row shows the dim text "↑↓ Scroll | Esc Close"
    assert!(
        joined(&rows).contains("\u{2191}\u{2193} Scroll | Esc Close"),
        "modal must render the footer hint '↑↓ Scroll | Esc Close'; got: {rows:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Re-opening the modal resets the scroll offset to the top
// ─────────────────────────────────────────────────────────────────────

#[test]
fn re_opening_the_modal_resets_the_scroll_offset_to_the_top() {
    // @step Given a turn content modal that has been scrolled down over a long turn
    let mut app = agent_app_with_long_last_turn();
    open_modal_on_long_turn(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be open");
    let _ = render_rows(&mut app, 60, 20);
    // Scroll down several rows so we are no longer at the top.
    for _ in 0..5 {
        let _ = app.handle_event(&key(KeyCode::Down));
        drain(&mut app);
    }
    assert!(
        modal_offset(&app) > 0,
        "precondition: modal must be scrolled down before re-open; got offset {}",
        modal_offset(&app)
    );

    // @step When I close the modal and re-open it on the same turn
    let _ = app.handle_event(&key(KeyCode::Esc)); // close modal, stay in SELECT mode
    drain(&mut app);
    assert_eq!(modal_seq(&app), None, "Esc must close the modal");
    let _ = app.handle_event(&key(KeyCode::Enter)); // re-open on the same selected turn
    drain(&mut app);
    assert!(modal_seq(&app).is_some(), "modal must be re-opened");

    // @step Then the modal shows the top of the content again
    assert_eq!(
        modal_offset(&app),
        0,
        "re-opening the modal must reset the scroll offset to the top"
    );
    let rows = render_rows(&mut app, 60, 20);
    assert!(
        joined(&rows).contains("TOPLINE"),
        "re-opened modal must show the top sentinel; got: {rows:?}"
    );
}
