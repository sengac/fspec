//! RPC-029 — AgentView session header alignment with the TS Ink original.
//!
//! Feature: spec/features/rpc029-agent-header-alignment.feature
//!
//! Split from tests/view_agent_unit_rpc029.rs (REFAC-051): the work-unit-aware
//! header semantics, the dark-grey header/footer rows and the header badges.

use codelet_fspec_tui::store::AgentViewStore;
use codelet_fspec_tui::views::agent::header::SessionHeader;
use codelet_rpc_types::{ModelInfo, SessionId, ThinkingLevel, WorkspaceInfo};
use codelet_test_helpers::{OrFail, TestResult};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::widgets::Widget;

mod rpc029_support;
use rpc029_support::*;

const DARK_GREY: Color = Color::Rgb(0x33, 0x33, 0x33);

/// Scenario: Header inserts work-unit prefix between session number and model
#[tokio::test]
async fn header_inserts_work_unit_prefix_between_session_number_and_model() -> TestResult {
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
            context_window: None,
            compaction_threshold: None,
        },
    );
    // @step And the store's current_work_unit_id is "RPC-029"
    // @step And the store's current_work_unit_status is "implementing"
    store.set_current_work_unit(
        Some("RPC-029".to_string()),
        Some("implementing".to_string()),
    );
    store.set_work_unit_context(
        SessionId::new("s-1"),
        codelet_rpc_types::WorkUnitContext {
            id: "RPC-029".into(),
            title: "Header card".into(),
            status: "implementing".into(),
        },
    );
    // @step When the App renders AgentView against a 120x20 TestBackend
    let buf = render_buffer(120, 20, &mut store, &mut view)?;
    let top = &rows_of(&buf)[0];
    // @step Then the rendered buffer's top row contains the substring "#1 (RPC-029: implementing): claude-sonnet-4"
    assert!(
        top.contains("#1 (RPC-029: implementing): claude-sonnet-4"),
        "top row missing work-unit prefix; got: {top:?}"
    );
    Ok(())
}

/// Scenario: Header omits work-unit prefix when no work unit is set
#[tokio::test]
async fn header_omits_work_unit_prefix_when_no_work_unit_is_set() -> TestResult {
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
            context_window: None,
            compaction_threshold: None,
        },
    );
    // @step And the store has no current_work_unit_id
    store.set_current_work_unit(None, None);
    // @step When the App renders AgentView against a 120x20 TestBackend
    let buf = render_buffer(120, 20, &mut store, &mut view)?;
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
    Ok(())
}

/// Scenario: Header and footer rows paint dark grey #333333 background on every cell
#[tokio::test]
async fn header_and_footer_rows_paint_dark_grey_background_on_every_cell() -> TestResult {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let buf = render_buffer(80, 20, &mut store, &mut view)?;
    let header_y: u16 = 0;
    let footer_y = find_row(&buf, "/tmp/scratch").or_fail("find footer row")?;
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
    Ok(())
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
async fn header_debug_badge_paints_red_bold_when_debug_enabled() -> TestResult {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    // @step And model_info_by_session["s-1"] is ModelInfo { display_name: "demo", supports_reasoning: false, supports_vision: false, context_window: 0 }
    let model = ModelInfo {
        display_name: "demo".to_string(),
        supports_reasoning: false,
        supports_vision: false,
        context_window: None,
        compaction_threshold: None,
    };
    // @step And the SessionHeader's is_debug_enabled field is true
    let header = SessionHeader {
        session_index: Some((1, 1)),
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
    let idx = row.find("[DEBUG]").or_fail("find debug badge")?;
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
    Ok(())
}

/// Scenario: Header [ISOLATED] badge paints green when session is isolated
#[tokio::test]
async fn header_isolated_badge_paints_green_when_session_is_isolated() -> TestResult {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    // @step And model_info_by_session["s-1"] is ModelInfo { display_name: "demo", supports_reasoning: false, supports_vision: false, context_window: 0 }
    let model = ModelInfo {
        display_name: "demo".to_string(),
        supports_reasoning: false,
        supports_vision: false,
        context_window: None,
        compaction_threshold: None,
    };
    // @step And the SessionHeader's is_isolated field is true
    let header = SessionHeader {
        session_index: Some((1, 1)),
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
    let idx = row.find("[ISOLATED]").or_fail("find isolated badge")?;
    let prefix = &row[..idx];
    let col = prefix.chars().count() as u16 + 1; // skip '['
    let cell = &buf[(col, 0)];
    assert_eq!(cell.symbol(), "I", "cell at col {col} should be 'I'");
    assert_eq!(cell.fg, Color::Green, "[ISOLATED] fg must be Green");
    Ok(())
}

/// Scenario: Header prefix + work unit + model run paints cyan and bold
#[tokio::test]
async fn header_prefix_work_unit_and_model_paint_cyan_bold() -> TestResult {
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
            context_window: None,
            compaction_threshold: None,
        },
    );
    // @step And the store's current_work_unit_id is "RPC-029"
    // @step And the store's current_work_unit_status is "implementing"
    store.set_current_work_unit(
        Some("RPC-029".to_string()),
        Some("implementing".to_string()),
    );
    // @step When the App renders AgentView against a 120x20 TestBackend
    let buf = render_buffer(120, 20, &mut store, &mut view)?;
    // @step Then the cell containing the "c" of "claude-sonnet-4" in the header row has foreground color Cyan and Bold modifier
    let (cx, cy) = find_substr_xy(&buf, "claude-sonnet-4").or_fail("find model name")?;
    let cell = &buf[(cx, cy)];
    assert_eq!(cell.symbol(), "c", "cell at ({cx},{cy}) should be 'c'");
    assert_eq!(cell.fg, Color::Cyan, "model name fg must be Cyan");
    assert!(
        cell.modifier.contains(Modifier::BOLD),
        "model name must be Bold; modifier={:?}",
        cell.modifier
    );
    Ok(())
}
