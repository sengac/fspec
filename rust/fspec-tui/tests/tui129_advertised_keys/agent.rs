//! TUI-129 agent-view scenarios: End, the agent help rows and the collapsed
//! tool card hint.
//!
//! Feature: spec/features/advertised-keys-do-what-their-hints-say.feature

use codelet_fspec_tui::views::agent::RenderedChunk;
use codelet_fspec_tui::{Action, App, Component, HelpDialog, ViewMode};
use codelet_rpc_types::{SessionId, StreamChunk, ToolCallInfo, ToolResultInfo};
use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;

use super::fixture::*;

/// s-1 open and focused on the agent view.
async fn agent_app() -> Result<App, TestError> {
    let (mut app, _mock) = fresh_app();
    open_sessions(&mut app, 1).await?;
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    drain(&mut app).await?;
    app.navigator_mut().active_view = ViewMode::Agent;
    Ok(app)
}

fn seed_rows(app: &mut App, count: usize) -> TestResult {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(&sid("s-1"))
        .or_fail("s-1 has a session context")?;
    for i in 0..count {
        ctx.scrollback.push(RenderedChunk {
            row_ends: Vec::new(),
            seq: i as u64,
            lines: vec![Line::from(format!("row-{i}"))],
            source: None,
        });
    }
    Ok(())
}

/// (offset, stick_to_bottom) of s-1's scrollback.
fn scroll(app: &App) -> Result<(usize, bool), TestError> {
    let state = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .or_fail("s-1 has a session context")?
        .scrollback
        .scroll_state();
    Ok((state.offset, state.stick_to_bottom))
}

async fn page_up(app: &mut App, times: usize) -> TestResult {
    for _ in 0..times {
        press(app, KeyCode::PageUp).await?;
    }
    Ok(())
}

/// Scenario: End on an empty input jumps to the bottom of the scrollback
#[tokio::test(start_paused = true)]
async fn end_on_an_empty_input_jumps_to_the_bottom_of_the_scrollback() -> TestResult {
    // @step Given an agent view with 200 rows of scrollback scrolled up five pages and an empty input
    let mut app = agent_app().await?;
    seed_rows(&mut app, 200)?;
    screen(&mut app, 80, 40)?;
    page_up(&mut app, 5).await?;
    let (scrolled_to, _) = scroll(&app)?;

    // @step When the user presses End
    press(&mut app, KeyCode::End).await?;

    // @step Then the scrollback is at the bottom and sticks to it
    screen(&mut app, 80, 40)?;
    let (offset, stick) = scroll(&app)?;
    assert!(stick, "End must stick the scrollback to the bottom");
    assert!(
        offset > scrolled_to + 30,
        "End must reach the bottom, not one page: {scrolled_to} -> {offset}"
    );

    // @step Given the input holds "abc" with the cursor at its start and the scrollback scrolled up
    for ch in "abc".chars() {
        type_char(&mut app, ch).await?;
    }
    press(&mut app, KeyCode::Home).await?;
    assert_eq!(app.navigator().agent.input.cursor(), (0, 0));
    page_up(&mut app, 2).await?;
    let before = scroll(&app)?;

    // @step When the user presses End
    press(&mut app, KeyCode::End).await?;

    // @step Then the cursor is at the end of "abc" and the scrollback has not moved
    assert_eq!(app.navigator().agent.input.cursor(), (0, 3));
    assert_eq!(scroll(&app)?, before);
    Ok(())
}

fn help_rows(dialog: &mut HelpDialog) -> Vec<String> {
    let area = Rect::new(0, 0, 120, 80);
    let mut buf = Buffer::empty(area);
    dialog.render(area, &mut buf);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect()
}

/// Scenario: The agent help describes Home, End, Alt+Enter and Esc as they behave
#[test]
fn the_agent_help_describes_home_end_alt_enter_and_esc_as_they_behave() {
    // @step When the agent help is opened
    let rows = help_rows(&mut HelpDialog::for_agent());
    let text = rows.join("\n");

    // @step Then it shows "Top / Bottom (empty input)" for Home and End
    assert!(
        text.contains("Home / End    Top / Bottom (empty input)"),
        "{text}"
    );

    // @step And it shows "Newline (queues the draft while busy)" for Alt+Enter
    assert!(
        text.contains("Alt+Enter     Newline (queues the draft while busy)"),
        "{text}"
    );

    // @step And its Esc row names stopping a run, clearing the draft and closing the agent
    let esc = rows
        .iter()
        .find(|row| row.contains("ESC           "))
        .map(String::as_str)
        .unwrap_or_default();
    for word in ["Stop", "clear", "close"] {
        assert!(esc.contains(word), "Esc row must name '{word}': {esc}");
    }

    // @step And it does not show "ESC           Back"
    assert!(!text.contains("ESC           Back"), "{text}");
}

/// Scenario: A collapsed tool card names Tab and Enter
#[tokio::test(start_paused = true)]
async fn a_collapsed_tool_card_names_tab_and_enter() -> TestResult {
    let mut app = agent_app().await?;
    let body = (1..=20)
        .map(|i| format!("line-{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::tool_call(ToolCallInfo {
            id: "tc-1".to_string(),
            name: "Bash".to_string(),
            input: "{\"command\":\"echo\"}".to_string(),
        }),
    ));
    app.dispatch(Action::ChunkReceived(
        SessionId::new("s-1"),
        StreamChunk::tool_result(ToolResultInfo {
            tool_call_id: "tc-1".to_string(),
            content: body,
            is_error: false,
        }),
    ));
    drain(&mut app).await?;

    // @step When a tool card with more than 8 output lines is rendered collapsed
    let rows = screen(&mut app, 120, 40)?;

    // @step Then it ends with "(Tab to select, Enter to view full)"
    assert!(
        shows(&rows, "... +12 lines (Tab to select, Enter to view full)"),
        "{}",
        rows.join("\n")
    );
    Ok(())
}
