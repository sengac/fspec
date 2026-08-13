//! RPC-029 — AgentView structure alignment with TS Ink original.
//!
//! Feature: spec/features/rpc029-agent-structure-alignment.feature
//!
//! Validates the layout reorder (footer above input), removal of the
//! scrollback + input Block borders, dark-grey row backgrounds with
//! paddingX=1, the new work-unit-aware header semantics, and the
//! footer right-side colour split + branch glyph reversion (⌥ → ⎇).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::store::AgentViewStore;
use codelet_fspec_tui::views::agent::header::SessionHeader;
use codelet_fspec_tui::views::AgentView;
use codelet_rpc_types::{ModelInfo, SessionId, ThinkingLevel, WorkspaceInfo};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::widgets::Widget;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

mod common;

const DARK_GREY: Color = Color::Rgb(0x33, 0x33, 0x33);

/// Render AgentView against an N×M TestBackend and return the buffer.
fn render_buffer(
    width: u16,
    height: u16,
    store: &mut AgentViewStore,
    view: &mut AgentView,
) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    term.backend().buffer().clone()
}

/// Collect the glyph rows of a buffer.
fn rows_of(buf: &Buffer) -> Vec<String> {
    let mut rows = Vec::new();
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

/// Locate the y-coordinate of the first row whose glyph content
/// contains `needle`.
fn find_row(buf: &Buffer, needle: &str) -> Option<u16> {
    rows_of(buf)
        .iter()
        .position(|r| r.contains(needle))
        .map(|i| i as u16)
}

/// Locate the (x, y) of the first occurrence of `needle` in any row.
fn find_substr_xy(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        if let Some(byte_idx) = row.find(needle) {
            // Convert byte index to character/column index assuming each
            // grapheme rendered as one cell (true for ASCII; the inputs
            // we search for here are ASCII).
            let prefix = &row[..byte_idx];
            let col = prefix.chars().count();
            return Some((col as u16, y));
        }
    }
    None
}

fn fresh_view() -> AgentView {
    let (tx, _rx) = unbounded_channel();
    AgentView::new(tx)
}

fn fresh_view_with_session(sid: &str) -> (AgentViewStore, AgentView) {
    let mut store = AgentViewStore::default();
    let session = SessionId::new(sid);
    store.append_session(codelet_fspec_tui::SessionContext::new(session));
    (store, fresh_view())
}

/// Scenario: Scrollback area has no border and no Agent title
#[tokio::test]
async fn scrollback_area_has_no_border_and_no_agent_title() {
    // @step Given an AgentViewStore with current_session "s-1"
    let (mut store, mut view) = fresh_view_with_session("s-1");
    // @step And the AgentView has pushed one scrollback line "user> hi"
    view.push_line(&mut store, "user> hi");
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view);
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
    // @step And the rendered buffer does NOT contain the substring "│"
    assert!(
        !full.contains('│'),
        "scrollback must have no vertical border glyph"
    );
    // @step And the rendered buffer does NOT contain the substring " Agent — "
    assert!(
        !full.contains(" Agent — "),
        "scrollback must not paint ' Agent — ' title"
    );
}

/// Scenario: Input area has no border and prompt sits at padded column
#[tokio::test]
async fn input_area_has_no_border_and_prompt_sits_at_padded_column() {
    // @step Given an empty AgentViewStore with no current_session
    let mut store = AgentViewStore::default();
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view);
    let rows = rows_of(&buf);
    let input_y = rows
        .iter()
        .position(|r| r.contains("> "))
        .expect("find input row containing '> '");
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
}

/// Scenario: Footer row appears strictly above the input row
#[tokio::test]
async fn footer_row_appears_strictly_above_input_row() {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: Some("main") }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: Some("main".to_string()),
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view);
    let footer_y = find_row(&buf, "/tmp/scratch").expect("find footer row");
    // @step Then the row containing the substring "/tmp/scratch" appears strictly above the row containing the green ">" prompt
    let input_y = find_row(&buf, "> ").expect("find input row");
    assert!(
        footer_y < input_y,
        "footer row ({footer_y}) must appear strictly above input row ({input_y})"
    );
}

/// Scenario: Header inserts work-unit prefix between session number and model
#[tokio::test]
async fn header_inserts_work_unit_prefix_between_session_number_and_model() {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    let (mut store, mut view) = fresh_view_with_session("s-1");
    let sid = SessionId::new("s-1");
    // @step And model_info_by_session["s-1"] is ModelInfo { display_name: "claude-sonnet-4", supports_reasoning: false, supports_vision: false, context_window: 0 }
    store.set_model_info(
        sid,
        ModelInfo {
            display_name: "claude-sonnet-4".to_string(),
            supports_reasoning: false,
            supports_vision: false,
            context_window: 0,
            compaction_threshold: 0,
        },
    );
    // @step And the store's current_work_unit_id is "RPC-029"
    // @step And the store's current_work_unit_status is "implementing"
    store.set_current_work_unit(
        Some("RPC-029".to_string()),
        Some("implementing".to_string()),
    );
    // @step When the App renders AgentView against a 120x20 TestBackend
    let buf = render_buffer(120, 20, &mut store, &mut view);
    let top = &rows_of(&buf)[0];
    // @step Then the rendered buffer's top row contains the substring "#1 (RPC-029: implementing): claude-sonnet-4"
    assert!(
        top.contains("#1 (RPC-029: implementing): claude-sonnet-4"),
        "top row missing work-unit prefix; got: {top:?}"
    );
}

/// Scenario: Header omits work-unit prefix when no work unit is set
#[tokio::test]
async fn header_omits_work_unit_prefix_when_no_work_unit_is_set() {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    let (mut store, mut view) = fresh_view_with_session("s-1");
    let sid = SessionId::new("s-1");
    // @step And model_info_by_session["s-1"] is ModelInfo { display_name: "claude-sonnet-4", supports_reasoning: false, supports_vision: false, context_window: 0 }
    store.set_model_info(
        sid,
        ModelInfo {
            display_name: "claude-sonnet-4".to_string(),
            supports_reasoning: false,
            supports_vision: false,
            context_window: 0,
            compaction_threshold: 0,
        },
    );
    // @step And the store has no current_work_unit_id
    store.set_current_work_unit(None, None);
    // @step When the App renders AgentView against a 120x20 TestBackend
    let buf = render_buffer(120, 20, &mut store, &mut view);
    let top = &rows_of(&buf)[0];
    // @step Then the rendered buffer's top row contains the substring "#1: claude-sonnet-4"
    assert!(
        top.contains("#1: claude-sonnet-4"),
        "top row missing '#1: claude-sonnet-4'; got: {top:?}"
    );
    // @step And the rendered buffer's top row does NOT contain the substring "(RPC"
    assert!(
        !top.contains("(RPC"),
        "top row should not contain '(RPC' when no work unit set; got: {top:?}"
    );
}

/// Scenario: Header and footer rows paint dark grey #333333 background on every cell
#[tokio::test]
async fn header_and_footer_rows_paint_dark_grey_background_on_every_cell() {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view);
    let header_y: u16 = 0;
    let footer_y = find_row(&buf, "/tmp/scratch").expect("find footer row");
    // @step Then every cell of the header row has background color RGB(0x33, 0x33, 0x33)
    for x in 0..buf.area.width {
        assert_eq!(
            buf[(x, header_y)].bg,
            DARK_GREY,
            "header cell ({x},{header_y}) bg must be #333333"
        );
    }
    // @step And every cell of the footer row has background color RGB(0x33, 0x33, 0x33)
    for x in 0..buf.area.width {
        assert_eq!(
            buf[(x, footer_y)].bg,
            DARK_GREY,
            "footer cell ({x},{footer_y}) bg must be #333333"
        );
    }
}

/// Scenario: Header and footer have horizontal padding of one column on both edges
#[tokio::test]
async fn header_and_footer_have_horizontal_padding_of_one_column_on_both_edges() {
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
            context_window: 0,
            compaction_threshold: 0,
        },
    );
    // @step And workspace is WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    }));
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view);
    let header_y: u16 = 0;
    let footer_y = find_row(&buf, "/tmp/scratch").expect("find footer row");
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
}

/// Scenario: Footer left side is empty - no Enter=send / Ctrl+C / ESC=back hints
#[tokio::test]
async fn footer_left_side_is_empty_no_hints() {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view);
    let footer_y = find_row(&buf, "/tmp/scratch").expect("find footer row");
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
}

/// Scenario: Footer branch glyph uses ⎇ U+2387 not ⌥ U+2325
#[tokio::test]
async fn footer_branch_glyph_uses_alternative_key_not_option_key() {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: Some("main") }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: Some("main".to_string()),
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view);
    let row = rows_of(&buf)
        .into_iter()
        .find(|r| r.contains("/tmp/scratch"))
        .expect("find footer row");
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
}

/// Scenario: Footer cwd span is dark-grey and bracketed branch span is cyan
#[tokio::test]
async fn footer_cwd_is_dim_and_bracketed_branch_is_cyan() {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: Some("main") }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: Some("main".to_string()),
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view);
    let (cwd_x, cwd_y) = find_substr_xy(&buf, "/tmp/scratch").expect("find cwd in footer");
    let (br_x, br_y) = find_substr_xy(&buf, "[\u{2387}").expect("find '[⎇' in footer");
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
}

/// Render a bare SessionHeader into a 80x1 buffer.
fn render_header_row(header: SessionHeader<'_>) -> Buffer {
    let area = Rect::new(0, 0, 80, 1);
    let mut buf = Buffer::empty(area);
    header.render(area, &mut buf);
    buf
}

/// Scenario: Header [DEBUG] badge paints red-bold when debug enabled
#[tokio::test]
async fn header_debug_badge_paints_red_bold_when_debug_enabled() {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    // @step And model_info_by_session["s-1"] is ModelInfo { display_name: "demo", supports_reasoning: false, supports_vision: false, context_window: 0 }
    let model = ModelInfo {
        display_name: "demo".to_string(),
        supports_reasoning: false,
        supports_vision: false,
        context_window: 0,
        compaction_threshold: 0,
    };
    // @step And the SessionHeader's is_debug_enabled field is true
    let header = SessionHeader {
        session_index: (1, 1),
        model: Some(&model),
        thinking: ThinkingLevel::Off,
        tokens: Default::default(),
        work_unit_id: None,
        work_unit_status: None,
        is_isolated: false,
        is_debug_enabled: true,
        is_select_mode: false,
        tokens_per_second: None,
        reasoning_tokens: 0,
        compaction_reduction: None,
        is_loading: false,
        subordinate_label: None,
    };
    // @step When the SessionHeader renders against an 80x1 buffer
    let buf = render_header_row(header);
    let row: String = (0..buf.area.width)
        .map(|x| buf[(x, 0)].symbol().to_string())
        .collect();
    // @step Then the rendered buffer's row 0 contains the substring "[DEBUG]"
    assert!(
        row.contains("[DEBUG]"),
        "header must paint '[DEBUG]'; got: {row:?}"
    );
    // @step And the cell containing the "D" of "[DEBUG]" has foreground color Red and Bold modifier
    let idx = row.find("[DEBUG]").expect("find debug badge");
    let prefix = &row[..idx];
    let col = prefix.chars().count() as u16 + 1; // skip '['
    let cell = &buf[(col, 0)];
    assert_eq!(cell.symbol(), "D", "cell at col {col} should be 'D'");
    assert_eq!(cell.fg, Color::Red, "[DEBUG] fg must be Red");
    assert!(
        cell.modifier.contains(Modifier::BOLD),
        "[DEBUG] must be Bold; modifier={:?}",
        cell.modifier
    );
}

/// Scenario: Header [ISOLATED] badge paints green when session is isolated
#[tokio::test]
async fn header_isolated_badge_paints_green_when_session_is_isolated() {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    // @step And model_info_by_session["s-1"] is ModelInfo { display_name: "demo", supports_reasoning: false, supports_vision: false, context_window: 0 }
    let model = ModelInfo {
        display_name: "demo".to_string(),
        supports_reasoning: false,
        supports_vision: false,
        context_window: 0,
        compaction_threshold: 0,
    };
    // @step And the SessionHeader's is_isolated field is true
    let header = SessionHeader {
        session_index: (1, 1),
        model: Some(&model),
        thinking: ThinkingLevel::Off,
        tokens: Default::default(),
        work_unit_id: None,
        work_unit_status: None,
        is_isolated: true,
        is_debug_enabled: false,
        is_select_mode: false,
        tokens_per_second: None,
        reasoning_tokens: 0,
        compaction_reduction: None,
        is_loading: false,
        subordinate_label: None,
    };
    // @step When the SessionHeader renders against an 80x1 buffer
    let buf = render_header_row(header);
    let row: String = (0..buf.area.width)
        .map(|x| buf[(x, 0)].symbol().to_string())
        .collect();
    // @step Then the rendered buffer's row 0 contains the substring "[ISOLATED]"
    assert!(
        row.contains("[ISOLATED]"),
        "header must paint '[ISOLATED]'; got: {row:?}"
    );
    // @step And the cell containing the "I" of "[ISOLATED]" has foreground color Green
    let idx = row.find("[ISOLATED]").expect("find isolated badge");
    let prefix = &row[..idx];
    let col = prefix.chars().count() as u16 + 1; // skip '['
    let cell = &buf[(col, 0)];
    assert_eq!(cell.symbol(), "I", "cell at col {col} should be 'I'");
    assert_eq!(cell.fg, Color::Green, "[ISOLATED] fg must be Green");
}

/// Scenario: Header prefix + work unit + model run paints cyan and bold
#[tokio::test]
async fn header_prefix_work_unit_and_model_paint_cyan_bold() {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    let (mut store, mut view) = fresh_view_with_session("s-1");
    let sid = SessionId::new("s-1");
    // @step And model_info_by_session["s-1"] is ModelInfo { display_name: "claude-sonnet-4", supports_reasoning: false, supports_vision: false, context_window: 0 }
    store.set_model_info(
        sid,
        ModelInfo {
            display_name: "claude-sonnet-4".to_string(),
            supports_reasoning: false,
            supports_vision: false,
            context_window: 0,
            compaction_threshold: 0,
        },
    );
    // @step And the store's current_work_unit_id is "RPC-029"
    // @step And the store's current_work_unit_status is "implementing"
    store.set_current_work_unit(
        Some("RPC-029".to_string()),
        Some("implementing".to_string()),
    );
    // @step When the App renders AgentView against a 120x20 TestBackend
    let buf = render_buffer(120, 20, &mut store, &mut view);
    // @step Then the cell containing the "c" of "claude-sonnet-4" in the header row has foreground color Cyan and Bold modifier
    let (cx, cy) = find_substr_xy(&buf, "claude-sonnet-4").expect("find model name");
    let cell = &buf[(cx, cy)];
    assert_eq!(cell.symbol(), "c", "cell at ({cx},{cy}) should be 'c'");
    assert_eq!(cell.fg, Color::Cyan, "model name fg must be Cyan");
    assert!(
        cell.modifier.contains(Modifier::BOLD),
        "model name must be Bold; modifier={:?}",
        cell.modifier
    );
}
