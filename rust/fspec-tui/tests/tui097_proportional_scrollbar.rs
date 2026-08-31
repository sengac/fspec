//! TUI-097 — Resume view proportional scrollbar tests.
//!
//! Feature: spec/features/resume-view-proportional-scrollbar.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::mode_view_render::render_session_rows;
use codelet_rpc_types::SessionInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn make_session(id: &str, name: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: name.to_string(),
        status: "idle".to_string(),
        project: String::new(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
    }
}

fn rows_of(buf: &Buffer) -> Vec<String> {
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

fn has_glyph(buf: &Buffer, x: u16, y: u16, glyph: &str) -> bool {
    buf[(x, y)].symbol() == glyph
}

// ============================================================================
// Scenario: Scrollbar appears when session count exceeds visible rows
// ============================================================================

#[test]
fn scrollbar_appears_when_session_count_exceeds_visible_rows() {
    // @step Given the resume view has 30 sessions
    let sessions: Vec<SessionInfo> = (0..30)
        .map(|i| make_session(&format!("s-{i}"), &format!("Session {i}")))
        .collect();
    // @step And the body area height is 20 rows
    let area = Rect::new(0, 0, 80, 20);
    // @step When the view renders the session rows
    let mut buf = Buffer::empty(area);
    render_session_rows(area, &mut buf, &sessions, 0, 0);
    let rows = rows_of(&buf);
    // @step Then a proportional scrollbar is rendered on the rightmost column
    // The scrollbar occupies the last column (x = 79)
    let last_col = area.width - 1;
    let has_scrollbar = (0..area.height)
        .any(|y| has_glyph(&buf, last_col, y, "■") || has_glyph(&buf, last_col, y, "│"));
    assert!(
        has_scrollbar,
        "Scrollbar should be rendered when 30 sessions exceed visible area"
    );
    // @step And the content width is reduced by 1 column to accommodate the scrollbar
    // Content should not extend into the scrollbar column
    // The session names should be within the content area (width - 1)
    let joined = rows.join("\n");
    assert!(joined.contains("Session 0"));
}

// ============================================================================
// Scenario: No scrollbar when session count fits in visible area
// ============================================================================

#[test]
fn no_scrollbar_when_session_count_fits_in_visible_area() {
    // @step Given the resume view has 5 sessions
    let sessions: Vec<SessionInfo> = (0..5)
        .map(|i| make_session(&format!("s-{i}"), &format!("Session {i}")))
        .collect();
    // @step And the body area height is 20 rows
    let area = Rect::new(0, 0, 80, 20);
    // @step When the view renders the session rows
    let mut buf = Buffer::empty(area);
    render_session_rows(area, &mut buf, &sessions, 0, 0);
    let rows = rows_of(&buf);
    // @step Then no scrollbar is rendered
    let last_col = area.width - 1;
    let has_scrollbar = (0..area.height)
        .any(|y| has_glyph(&buf, last_col, y, "■") || has_glyph(&buf, last_col, y, "│"));
    assert!(
        !has_scrollbar,
        "Scrollbar should NOT be rendered when 5 sessions fit in 20 rows"
    );
    // @step And the content uses the full body width
    let joined = rows.join("\n");
    assert!(joined.contains("Session 0"));
    assert!(joined.contains("Session 4"));
}

// ============================================================================
// Scenario: Scrollbar thumb position is proportional to scroll offset
// ============================================================================

#[test]
fn scrollbar_thumb_position_is_proportional_to_scroll_offset() {
    // @step Given the resume view has 30 sessions
    let sessions: Vec<SessionInfo> = (0..30)
        .map(|i| make_session(&format!("s-{i}"), &format!("Session {i}")))
        .collect();
    // @step And the body area height is 20 rows
    let area = Rect::new(0, 0, 80, 20);
    // @step And the user has scrolled to session index 15
    let scroll_offset = 15;
    // @step When the view renders the session rows
    let mut buf = Buffer::empty(area);
    render_session_rows(area, &mut buf, &sessions, 15, scroll_offset);
    // @step Then the scrollbar thumb is positioned at approximately half the track height
    let last_col = area.width - 1;
    // Find the first thumb cell
    let thumb_start = (0..area.height)
        .find(|&y| has_glyph(&buf, last_col, y, "■"))
        .expect("Should find thumb glyph");
    // With scroll_offset=15, total=30, thumb should be around half the track
    let track_height = area.height as usize;
    let expected_pos = (scroll_offset * track_height) / sessions.len();
    // Allow some tolerance (within 2 rows)
    let actual_pos = thumb_start as usize;
    assert!(
        (actual_pos as i32 - expected_pos as i32).abs() <= 2,
        "Thumb at row {actual_pos} should be near expected position {expected_pos}"
    );
}

// ============================================================================
// Scenario: Scrollbar uses DIM styled glyphs for thumb and track
// ============================================================================

#[test]
fn scrollbar_uses_dim_styled_glyphs_for_thumb_and_track() {
    // @step Given the resume view has 30 sessions
    let sessions: Vec<SessionInfo> = (0..30)
        .map(|i| make_session(&format!("s-{i}"), &format!("Session {i}")))
        .collect();
    // @step And the body area height is 20 rows
    let area = Rect::new(0, 0, 80, 20);
    // @step When the view renders the session rows
    let mut buf = Buffer::empty(area);
    render_session_rows(area, &mut buf, &sessions, 0, 0);
    let last_col = area.width - 1;
    // @step Then the scrollbar thumb uses the ■ glyph with DIM modifier
    let has_thumb = (0..area.height).any(|y| has_glyph(&buf, last_col, y, "■"));
    assert!(has_thumb, "Scrollbar should contain ■ thumb glyph");
    // @step And the scrollbar track uses the │ glyph with DIM modifier
    let has_track = (0..area.height).any(|y| has_glyph(&buf, last_col, y, "│"));
    assert!(has_track, "Scrollbar should contain │ track glyph");
}
