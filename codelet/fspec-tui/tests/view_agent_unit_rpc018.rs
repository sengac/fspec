//! RPC-018 — AgentView SessionHeader + SessionFooter widget render tests.
//!
//! Feature: spec/features/rpc018-agent-chrome.feature
//!
//! Drives the rendering scenarios for the new 1-row Header (model
//! badges + token deltas) + 1-row Footer (input hints + cwd + branch)
//! that sandwich AgentView's scrollback and input from RPC-009/RPC-012.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::store::AgentViewStore;
use codelet_fspec_tui::views::AgentView;
use codelet_rpc_types::{
    ContextFillInfo, ModelInfo, SessionId, StreamChunk, ThinkingLevel, TokenTracker,
    WorkspaceInfo,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

mod common;

/// Helper: render AgentView against an N×M TestBackend and return the
/// buffer as a Vec<String> of row text.
fn render_rows(width: u16, height: u16, store: &AgentViewStore, view: &mut AgentView) -> Vec<String> {
    let backend = TestBackend::new(width, height);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    let buf: Buffer = term.backend().buffer().clone();
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

fn fresh_view() -> AgentView {
    let (tx, _rx) = unbounded_channel();
    AgentView::new(tx)
}

/// Scenario: Empty AgentViewStore paints placeholder header and bare-cwd footer
#[tokio::test]
async fn empty_agent_view_store_paints_placeholder_header_and_bare_cwd_footer() {
    // @step Given an empty AgentViewStore with no current_session, no model_info, no thinking_level, and no workspace snapshot
    let store = AgentViewStore::default();
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 80x20 TestBackend
    let rows = render_rows(80, 20, &store, &mut view);
    let top = &rows[0];
    let bottom = &rows[rows.len() - 1];
    let full = rows.join("\n");
    // @step Then the rendered buffer's top row contains the substring "Agent"
    assert!(top.contains("Agent"), "top row should contain 'Agent', got: {top:?}");
    // @step And the rendered buffer's top row contains the substring "tokens: 0↓ 0↑ [0%]"
    assert!(
        top.contains("tokens: 0↓ 0↑ [0%]"),
        "top row should contain 'tokens: 0↓ 0↑ [0%]', got: {top:?}"
    );
    // @step And the rendered buffer's bottom row contains the substring "Enter=send"
    assert!(bottom.contains("Enter=send"), "bottom row should contain 'Enter=send', got: {bottom:?}");
    // @step And the rendered buffer's bottom row contains the substring "ESC=back"
    assert!(bottom.contains("ESC=back"), "bottom row should contain 'ESC=back', got: {bottom:?}");
    // @step And the rendered buffer does NOT contain the substring "[R]"
    assert!(!full.contains("[R]"), "full buffer should NOT contain '[R]'");
    // @step And the rendered buffer does NOT contain the substring "[V]"
    assert!(!full.contains("[V]"), "full buffer should NOT contain '[V]'");
    // @step And the rendered buffer does NOT contain the substring "[T:"
    assert!(!full.contains("[T:"), "full buffer should NOT contain '[T:'");
}

/// Scenario: Header paints model badges and thinking level when session has model info
#[tokio::test]
async fn header_paints_model_badges_and_thinking_level_when_session_has_model_info() {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    let mut store = AgentViewStore::default();
    let sid = SessionId::new("s-1");
    store.set_current_session(Some(sid.clone()));
    store.set_session_index(1, 1);
    // @step And model_info_by_session["s-1"] is ModelInfo { display_name: "Claude Opus 4.7", supports_reasoning: true, supports_vision: true, context_window: 192000 }
    store.set_model_info(
        sid.clone(),
        ModelInfo {
            display_name: "Claude Opus 4.7".to_string(),
            supports_reasoning: true,
            supports_vision: true,
            context_window: 192_000,
        },
    );
    // @step And thinking_level_by_session["s-1"] is ThinkingLevel::High
    store.set_thinking_level(sid, ThinkingLevel::High);

    let mut view = fresh_view();
    // @step When the App renders AgentView against an 100x20 TestBackend
    let rows = render_rows(100, 20, &store, &mut view);
    let top = &rows[0];
    // @step Then the rendered buffer's top row contains the substring "#1:"
    assert!(top.contains("#1:"), "top row missing '#1:', got: {top:?}");
    // @step And the rendered buffer's top row contains the substring "Claude Opus 4.7"
    assert!(top.contains("Claude Opus 4.7"), "top row missing model name, got: {top:?}");
    // @step And the rendered buffer's top row contains the substring "[R]"
    assert!(top.contains("[R]"), "top row missing '[R]', got: {top:?}");
    // @step And the rendered buffer's top row contains the substring "[V]"
    assert!(top.contains("[V]"), "top row missing '[V]', got: {top:?}");
    // @step And the rendered buffer's top row contains the substring "[192k]"
    assert!(top.contains("[192k]"), "top row missing '[192k]', got: {top:?}");
    // @step And the rendered buffer's top row contains the substring "[T:High]"
    assert!(top.contains("[T:High]"), "top row missing '[T:High]', got: {top:?}");
}

/// Scenario: Header right-side reflects TokenUpdate followed by ContextFillUpdate
#[tokio::test]
async fn header_right_side_reflects_token_update_followed_by_context_fill_update() {
    // @step Given an AgentViewStore with current_session "s-1"
    let mut store = AgentViewStore::default();
    let sid = SessionId::new("s-1");
    store.set_current_session(Some(sid.clone()));
    store.set_session_index(1, 1);
    // @step And token_state_by_session["s-1"] is TokenState { input_tokens: 1234, output_tokens: 567, context_fill_pct: 45 }
    store.set_token_state(
        sid,
        codelet_fspec_tui::store::TokenState {
            input_tokens: 1234,
            output_tokens: 567,
            context_fill_pct: 45,
        },
    );
    let mut view = fresh_view();
    // @step When the App renders AgentView against an 100x20 TestBackend
    let rows = render_rows(100, 20, &store, &mut view);
    let top = &rows[0];
    // @step Then the rendered buffer's top row contains the substring "tokens: 1234↓ 567↑ [45%]"
    assert!(
        top.contains("tokens: 1234↓ 567↑ [45%]"),
        "top row missing token deltas, got: {top:?}"
    );
}

/// Scenario: Footer abbreviates cwd to ~ inside $HOME and appends [⌥ branch] in a git repo
#[tokio::test]
async fn footer_abbreviates_cwd_to_tilde_inside_home_and_appends_branch_in_a_git_repo() {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/Users/rquast/projects/fspec", git_branch: Some("codelet-integration") }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/Users/rquast/projects/fspec".to_string(),
        git_branch: Some("codelet-integration".to_string()),
    }));
    // @step And the env var HOME is "/Users/rquast"
    std::env::set_var("HOME", "/Users/rquast");
    let mut view = fresh_view();
    // @step When the App renders AgentView against a 100x20 TestBackend
    let rows = render_rows(100, 20, &store, &mut view);
    let bottom = &rows[rows.len() - 1];
    // @step Then the rendered buffer's bottom row contains the substring "~/projects/fspec"
    assert!(bottom.contains("~/projects/fspec"), "bottom missing ~/projects/fspec, got: {bottom:?}");
    // @step And the rendered buffer's bottom row contains the substring "[⌥ codelet-integration]"
    assert!(
        bottom.contains("[⌥ codelet-integration]"),
        "bottom missing branch decoration, got: {bottom:?}"
    );
    // @step And the rendered buffer's bottom row does NOT contain the substring "/Users/rquast/projects/fspec"
    assert!(
        !bottom.contains("/Users/rquast/projects/fspec"),
        "bottom should not contain absolute home path, got: {bottom:?}"
    );
}

/// Scenario: Footer omits the [⌥ ...] segment when the workspace is not a git repo
#[tokio::test]
async fn footer_omits_branch_segment_when_workspace_is_not_a_git_repo() {
    // @step Given an AgentViewStore with workspace WorkspaceInfo { cwd: "/tmp/scratch", git_branch: None }
    let mut store = AgentViewStore::default();
    store.set_workspace(Some(WorkspaceInfo {
        cwd: "/tmp/scratch".to_string(),
        git_branch: None,
    }));
    let mut view = fresh_view();
    // @step When the App renders AgentView against a 100x20 TestBackend
    let rows = render_rows(100, 20, &store, &mut view);
    let bottom = &rows[rows.len() - 1];
    // @step Then the rendered buffer's bottom row contains the substring "/tmp/scratch"
    assert!(bottom.contains("/tmp/scratch"), "bottom missing cwd, got: {bottom:?}");
    // @step And the rendered buffer's bottom row does NOT contain the substring "[⌥"
    assert!(!bottom.contains("[⌥"), "bottom should not have branch decoration, got: {bottom:?}");
}

/// Scenario: AgentView layout splits area into Header / Scrollback / Input / Footer
#[tokio::test]
async fn agent_view_layout_splits_area_into_header_scrollback_input_footer() {
    // @step Given an AgentViewStore with current_session "s-1" listed as session #1 of 1
    let mut store = AgentViewStore::default();
    let sid = SessionId::new("s-1");
    store.set_current_session(Some(sid.clone()));
    store.set_session_index(1, 1);
    store.set_model_info(
        sid,
        ModelInfo {
            display_name: "demo".to_string(),
            supports_reasoning: false,
            supports_vision: false,
            context_window: 0,
        },
    );

    let mut view = fresh_view();
    // @step And the AgentView has pushed two scrollback lines "user> hi" and "assistant> hello"
    view.push_line("user> hi");
    view.push_line("assistant> hello");
    // @step When the App renders AgentView against an 80x10 TestBackend
    let rows = render_rows(80, 10, &store, &mut view);

    // @step Then the rendered buffer's row 0 contains the substring "#1:"
    assert!(rows[0].contains("#1:"), "row 0 should contain '#1:', got: {:?}", rows[0]);
    // @step And the rendered buffer's rows 1 through 5 contain the substring "user> hi"
    let scroll_area: String = rows[1..=5].join("\n");
    assert!(scroll_area.contains("user> hi"), "rows 1-5 should contain 'user> hi'; got:\n{scroll_area}");
    // @step And the rendered buffer's rows 1 through 5 contain the substring "assistant> hello"
    assert!(
        scroll_area.contains("assistant> hello"),
        "rows 1-5 should contain 'assistant> hello'; got:\n{scroll_area}"
    );
    // @step And the rendered buffer's row 9 contains the substring "Enter=send"
    assert!(
        rows[9].contains("Enter=send"),
        "row 9 should contain 'Enter=send'; got: {:?}",
        rows[9]
    );
}

/// Scenario: StreamChunk::TokenUpdate updates AgentViewStore.token_state_by_session for the current session
#[test]
fn stream_chunk_token_update_updates_token_state_for_current_session() {
    // @step Given an App with current_session "s-1"
    let mut store = AgentViewStore::default();
    let sid = SessionId::new("s-1");
    store.set_current_session(Some(sid.clone()));
    // @step And token_state_by_session["s-1"] starts at TokenState::default()
    assert_eq!(
        store.token_state_for(&sid).copied().unwrap_or_default(),
        codelet_fspec_tui::store::TokenState::default()
    );
    // @step When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::TokenUpdate with tokens { input_tokens: 1234, output_tokens: 567 })
    let chunk = StreamChunk::TokenUpdate {
        tokens: TokenTracker {
            input_tokens: 1234,
            output_tokens: 567,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            tokens_per_second: None,
            cumulative_billed_input: None,
            cumulative_billed_output: None,
            reasoning_tokens: None,
        },
    };
    store.apply_chunk_to_token_state(&sid, &chunk);
    let ts = store.token_state_for(&sid).copied().expect("token state");
    // @step Then AgentViewStore.token_state_by_session["s-1"] has input_tokens = 1234
    assert_eq!(ts.input_tokens, 1234);
    // @step And AgentViewStore.token_state_by_session["s-1"] has output_tokens = 567
    assert_eq!(ts.output_tokens, 567);
}

/// Scenario: StreamChunk::ContextFillUpdate updates context_fill_pct
#[test]
fn stream_chunk_context_fill_update_updates_context_fill_pct() {
    // @step Given an App with current_session "s-1"
    let mut store = AgentViewStore::default();
    let sid = SessionId::new("s-1");
    store.set_current_session(Some(sid.clone()));
    // @step And token_state_by_session["s-1"] starts at TokenState { input_tokens: 100, output_tokens: 50, context_fill_pct: 0 }
    store.set_token_state(
        sid.clone(),
        codelet_fspec_tui::store::TokenState {
            input_tokens: 100,
            output_tokens: 50,
            context_fill_pct: 0,
        },
    );
    // @step When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate with context_fill { fill_percentage: 45, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 })
    let chunk = StreamChunk::ContextFillUpdate {
        context_fill: ContextFillInfo {
            fill_percentage: 45,
            effective_tokens: 0.0,
            threshold: 0.0,
            context_window: 0.0,
        },
    };
    store.apply_chunk_to_token_state(&sid, &chunk);
    let ts = store.token_state_for(&sid).copied().expect("token state");
    // @step Then AgentViewStore.token_state_by_session["s-1"] has context_fill_pct = 45
    assert_eq!(ts.context_fill_pct, 45);
    // @step And input_tokens and output_tokens are unchanged (100 and 50)
    assert_eq!(ts.input_tokens, 100);
    assert_eq!(ts.output_tokens, 50);
}

/// Scenario: Non-token StreamChunk variants leave token_state unchanged
#[test]
fn non_token_stream_chunk_variants_leave_token_state_unchanged() {
    // @step Given an App with current_session "s-1"
    let mut store = AgentViewStore::default();
    let sid = SessionId::new("s-1");
    store.set_current_session(Some(sid.clone()));
    // @step And token_state_by_session["s-1"] is TokenState { input_tokens: 1234, output_tokens: 567, context_fill_pct: 45 }
    store.set_token_state(
        sid.clone(),
        codelet_fspec_tui::store::TokenState {
            input_tokens: 1234,
            output_tokens: 567,
            context_fill_pct: 45,
        },
    );
    // @step When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::Text { text: "hi", correlation_id: None, observed_correlation_ids: None })
    let chunk = StreamChunk::Text {
        text: "hi".to_string(),
        correlation_id: None,
        observed_correlation_ids: None,
    };
    store.apply_chunk_to_token_state(&sid, &chunk);
    let ts = store.token_state_for(&sid).copied().expect("token state");
    // @step Then AgentViewStore.token_state_by_session["s-1"] still has input_tokens = 1234
    assert_eq!(ts.input_tokens, 1234);
    // @step And output_tokens still equals 567
    assert_eq!(ts.output_tokens, 567);
    // @step And context_fill_pct still equals 45
    assert_eq!(ts.context_fill_pct, 45);
}
