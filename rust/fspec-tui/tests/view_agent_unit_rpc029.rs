//! RPC-029 — AgentView structure alignment with TS Ink original.
//!
//! Feature: spec/features/rpc029-agent-structure-alignment.feature
//!
//! Validates the layout reorder (footer above input), removal of the
//! scrollback + input Block borders, dark-grey row backgrounds with
//! paddingX=1, the new work-unit-aware header semantics, and the
//! footer right-side colour split + branch glyph reversion (⌥ → ⎇).

use codelet_fspec_tui::store::AgentViewStore;
use codelet_rpc_types::{ModelInfo, SessionId, WorkspaceInfo};
use codelet_test_helpers::{OrFail, TestResult};
use ratatui::style::Color;

mod common;
mod rpc029_support;
use rpc029_support::*;

/// Scenario: Scrollback area has no border and no Agent title
#[tokio::test]
async fn scrollback_area_has_no_border_and_no_agent_title() -> TestResult {
    // @step Given an AgentViewStore with current_session "s-1"
    let (mut store, mut view) = fresh_view_with_session("s-1");
    // @step And the AgentView has pushed one scrollback line "user> hi"
    view.push_line(&mut store, "user> hi");
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view)?;
    let full: String = rows_of(&buf).join("\n");
    // @step Then the rendered buffer does NOT contain the substring "┌"
    assert!(
        !full.contains('┌'),
        "scrollback must have no top-left border glyph"
    );
    // @step And the rendered buffer does NOT contain the substring "└"
    assert!(
        !full.contains('└'),
        "scrollback must have no bottom-left border glyph"
    );
    // @step And the rows below the menu bar row do NOT contain the substring "│" (MENU-003: the bar row's zone separator itself uses '│')
    // The single-view pane's bar row is row 1 (one below the header);
    // everything below it is the scrollback / footer / input region,
    // which stays border-free.
    let below_bar = buf.area.y + 2..buf.area.height;
    for y in below_bar {
        let row: String = (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        assert!(
            !row.contains('│'),
            "row {y} (below the MENU-003 bar row) must have no vertical border glyph: {row:?}"
        );
    }
    // @step And the rendered buffer does NOT contain the substring " Agent — "
    assert!(
        !full.contains(" Agent — "),
        "scrollback must not paint ' Agent — ' title"
    );
    Ok(())
}

/// Scenario: Input area has no border and prompt sits at padded column
#[tokio::test]
async fn input_area_has_no_border_and_prompt_sits_at_padded_column() -> TestResult {
    // @step Given an empty AgentViewStore with no current_session
    let mut store = AgentViewStore::default();
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view)?;
    let rows = rows_of(&buf);
    let input_y = rows
        .iter()
        .position(|r| r.contains("> "))
        .or_fail("find input row containing '> '")?;
    let input_row = &rows[input_y];
    // @step Then the input row contains the substring "> "
    assert!(input_row.contains("> "), "input row should contain '> '");
    // @step And the input row does NOT contain the substring "│"
    assert!(
        !input_row.contains('│'),
        "input row must have no vertical border glyph"
    );
    // @step And the cell at column 1 of the input row contains the character ">"
    let cell = buf[(1, input_y as u16)].symbol();
    assert_eq!(
        cell, ">",
        "input prompt '>' must sit at column 1 (paddingX=1)"
    );
    Ok(())
}

/// Scenario: Footer row appears strictly above the input row
#[tokio::test]
async fn footer_row_appears_strictly_above_input_row() -> TestResult {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: Some("main") }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: Some("main".to_string()),
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view)?;
    let footer_y = find_row(&buf, "/tmp/scratch").or_fail("find footer row")?;
    // @step Then the row containing the substring "/tmp/scratch" appears strictly above the row containing the green ">" prompt
    let input_y = find_row(&buf, "> ").or_fail("find input row")?;
    assert!(
        footer_y < input_y,
        "footer row ({footer_y}) must appear strictly above input row ({input_y})"
    );
    Ok(())
}

/// Scenario: Header and footer have horizontal padding of one column on both edges
#[tokio::test]
async fn header_and_footer_have_horizontal_padding_of_one_column_on_both_edges() -> TestResult {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    let (mut store, mut view) = fresh_view_with_session("s-1");
    let sid = SessionId::new("s-1");
    // @step And model_info_by_session["s-1"] is ModelInfo { display_name: "demo", supports_reasoning: false, supports_vision: false, context_window: 0 }
    store.set_model_info(
        sid,
        ModelInfo {
            display_name: "demo".to_string(),
            supports_reasoning: false,
            supports_vision: false,
            context_window: None,
            compaction_threshold: None,
        },
    );
    // @step And workspace is WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    }));
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view)?;
    let header_y: u16 = 0;
    let footer_y = find_row(&buf, "/tmp/scratch").or_fail("find footer row")?;
    let last_x = buf.area.width - 1;
    // @step Then the first column of the header row contains no glyph
    assert_eq!(
        buf[(0, header_y)].symbol(),
        " ",
        "header col 0 must be empty"
    );
    // @step And the last column of the header row contains no glyph
    assert_eq!(
        buf[(last_x, header_y)].symbol(),
        " ",
        "header last col must be empty"
    );
    // @step And the first column of the footer row contains no glyph
    assert_eq!(
        buf[(0, footer_y)].symbol(),
        " ",
        "footer col 0 must be empty"
    );
    // @step And the last column of the footer row contains no glyph
    assert_eq!(
        buf[(last_x, footer_y)].symbol(),
        " ",
        "footer last col must be empty"
    );
    Ok(())
}

/// Scenario: Footer left side is empty - no Enter=send / Ctrl+C / ESC=back hints
#[tokio::test]
async fn footer_left_side_is_empty_no_hints() -> TestResult {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view)?;
    let footer_y = find_row(&buf, "/tmp/scratch").or_fail("find footer row")?;
    let row = &rows_of(&buf)[footer_y as usize];
    // @step Then the footer row does NOT contain the substring "Enter=send"
    assert!(
        !row.contains("Enter=send"),
        "footer must not contain 'Enter=send'; got: {row:?}"
    );
    // @step And the footer row does NOT contain the substring "Ctrl+C"
    assert!(
        !row.contains("Ctrl+C"),
        "footer must not contain 'Ctrl+C'; got: {row:?}"
    );
    // @step And the footer row does NOT contain the substring "ESC=back"
    assert!(
        !row.contains("ESC=back"),
        "footer must not contain 'ESC=back'; got: {row:?}"
    );
    Ok(())
}

/// Scenario: Footer branch glyph uses ⎇ U+2387 not ⌥ U+2325
#[tokio::test]
async fn footer_branch_glyph_uses_alternative_key_not_option_key() -> TestResult {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: Some("main") }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: Some("main".to_string()),
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view)?;
    let row = rows_of(&buf)
        .into_iter()
        .find(|r| r.contains("/tmp/scratch"))
        .or_fail("find footer row")?;
    // @step Then the footer row contains the substring "[⎇ main]"
    assert!(
        row.contains("[\u{2387} main]"),
        "footer must contain '[⎇ main]' (U+2387); got: {row:?}"
    );
    // @step And the footer row does NOT contain the substring "[⌥"
    assert!(
        !row.contains("[\u{2325}"),
        "footer must not contain '[⌥' (U+2325); got: {row:?}"
    );
    Ok(())
}

/// Scenario: Footer cwd span is dark-grey and bracketed branch span is cyan
#[tokio::test]
async fn footer_cwd_is_dim_and_bracketed_branch_is_cyan() -> TestResult {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: Some("main") }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: Some("main".to_string()),
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view)?;
    let (cwd_x, cwd_y) = find_substr_xy(&buf, "/tmp/scratch").or_fail("find cwd in footer")?;
    let (br_x, br_y) = find_substr_xy(&buf, "[\u{2387}").or_fail("find '[⎇' in footer")?;
    // @step Then the cell at the cwd position of the footer row has foreground color DarkGray
    assert_eq!(
        buf[(cwd_x, cwd_y)].fg,
        Color::DarkGray,
        "cwd cell fg must be DarkGray"
    );
    // @step And the cell at the branch suffix position of the footer row has foreground color Cyan
    assert_eq!(
        buf[(br_x, br_y)].fg,
        Color::Cyan,
        "branch suffix cell fg must be Cyan"
    );
    Ok(())
}
