//! MUX-013 — the mux bar has the New Bot, New Agent and Close buttons of
//! the board and agent views: they wrap onto a second bar row when the
//! row is too narrow, start sessions in the next agent pane (MUX-002
//! placement) and close the focused pane's bot or agent (TUI-124).
//!
//! Feature: spec/features/mux-bar-new-bot-new-agent-and-close-buttons.feature
//!
//! Every test drives the real `App` over a `MockBackend` on a paused tokio
//! clock (TEST-042): sessions open through `Action::SessionCreated`, the
//! layout through `/mux <layout>`, keys and clicks through
//! `App::handle_event`, and frames are drawn on a `TestBackend` of the
//! scenario's size.

use std::sync::Arc;

use codelet_fspec_tui::components::create_session_dialog::CREATE_SESSION_DIALOG_ID;
use codelet_fspec_tui::components::menu_bar::MenuFocus;
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, SessionStatus};
use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

mod common;
use common::{MockBackend, TaskOutcome};

const CLOSE_DIALOG_ID: &str = "exit-confirmation-dialog";
const ALL_BUTTONS: &str = "[ New Bot ]  [ New Agent ]  [ Close ]";

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Run every pending task and queued action until nothing moves.
async fn drain(app: &mut App) -> Result<(), TestError> {
    loop {
        let mut moved = false;
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
            moved = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            moved = true;
        }
        if !moved {
            return Ok(());
        }
    }
}

/// The app at `size`: its rows as text after one drawn frame.
struct Screen {
    app: App,
    mock: Arc<MockBackend>,
    size: (u16, u16),
}

impl Screen {
    /// Sessions `ids` open, mux on with `layout` at `width` x `height`.
    async fn new(ids: &[&str], layout: &str, width: u16, height: u16) -> Result<Self, TestError> {
        let mock = Arc::new(MockBackend::new());
        let backend: Arc<dyn FspecBackend> = mock.clone();
        let mut screen = Self {
            app: App::new(backend),
            mock,
            size: (width, height),
        };
        for id in ids {
            screen.app.dispatch(Action::SessionCreated(sid(id)));
        }
        drain(&mut screen.app).await?;
        screen.rows()?;
        let session = screen.app.current_session().or_fail("a current session")?;
        screen
            .app
            .dispatch(Action::InputSubmitted(session, format!("/mux {layout}")));
        drain(&mut screen.app).await?;
        screen.rows()?;
        if screen.app.navigator().active_view != ViewMode::Mux {
            return Err(TestError::from(format!("/mux {layout} did not enter mux")));
        }
        Ok(screen)
    }

    fn rows(&mut self) -> Result<Vec<String>, TestError> {
        let (width, height) = self.size;
        let mut term = Terminal::new(TestBackend::new(width, height)).or_fail("Terminal::new")?;
        let app = &mut self.app;
        term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
            .or_fail("draw")?;
        let buf = term.backend().buffer();
        Ok((0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
            .collect())
    }

    fn text(&mut self) -> Result<String, TestError> {
        Ok(self.rows()?.join("\n"))
    }

    async fn event(&mut self, event: Event) -> TestResult {
        let _ = self.app.handle_event(&event);
        drain(&mut self.app).await?;
        Ok(())
    }

    async fn key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> TestResult {
        self.event(Event::Key(KeyEvent::new(code, modifiers))).await
    }

    /// Left-click the first cell of `label` on the bar rows.
    async fn click(&mut self, label: &str) -> TestResult {
        let rows = self.rows()?;
        let (x, y) = rows
            .iter()
            .take(2)
            .enumerate()
            .find_map(|(y, row)| row.find(label).map(|b| (row[..b].chars().count(), y)))
            .or_fail(&format!(
                "the bar paints {label}:\n{}",
                rows[..2].join("\n")
            ))?;
        self.event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x as u16 + 1,
            row: y as u16,
            modifiers: KeyModifiers::NONE,
        }))
        .await
    }

    fn bar_focus(&self) -> Option<MenuFocus> {
        self.app.navigator().mux.menu_focus()
    }

    /// Enter the bar at its first item and press Right until `target`.
    async fn walk_to(&mut self, target: MenuFocus) -> TestResult {
        self.rows()?;
        self.app.dispatch(Action::MenuMoveToItem(0));
        drain(&mut self.app).await?;
        for _ in 0..20 {
            if self.bar_focus() == Some(target) {
                return Ok(());
            }
            self.key(KeyCode::Right, KeyModifiers::NONE).await?;
        }
        Err(TestError::from(format!(
            "the ring never reached {target:?} (at {:?})",
            self.bar_focus()
        )))
    }

    fn focus(&self) -> usize {
        self.app.navigator().mux.focus()
    }

    fn panes(&self) -> Vec<MuxPaneKind> {
        self.app.navigator().mux.effective_panes().to_vec()
    }

    fn rects(&self) -> Vec<Rect> {
        self.app.navigator().mux.pane_rects().to_vec()
    }

    fn focused_session(&self) -> Option<SessionId> {
        self.app.navigator().mux.focused_session_id()
    }

    fn sessions(&self) -> usize {
        self.app.agent_view_store().open_sessions().len()
    }
}

fn ends_with(row: &str, tail: &str) -> bool {
    row.trim_end().ends_with(tail)
}

/// Scenario: At 120 columns the buttons end the single bar row
#[tokio::test(start_paused = true)]
async fn at_120_columns_the_buttons_end_the_single_bar_row() -> TestResult {
    // @step Given the terminal is 120x40 and mux is on with the layout "board, agent", one agent open and the agent pane focused
    let mut s = Screen::new(&["s-1"], "board, agent", 120, 40).await?;
    s.app.navigator_mut().mux.set_focus(1);
    // @step When the mux is drawn
    let rows = s.rows()?;
    // @step Then the bar row ends with "[ New Bot ]  [ New Agent ]  [ Close ]"
    assert!(ends_with(&rows[0], ALL_BUTTONS), "row 0: {:?}", rows[0]);
    assert!(rows[0].contains("[ Kanban ]"));
    // @step And the panes start on the row below the bar
    assert!(s.rects().iter().all(|r| r.y == 1), "{:?}", s.rects());
    Ok(())
}

/// Scenario: At 80 columns the buttons wrap onto a second bar row
#[tokio::test(start_paused = true)]
async fn at_80_columns_the_buttons_wrap_onto_a_second_bar_row() -> TestResult {
    // @step Given the terminal is 80x24 and mux is on with the layout "board, agent", one agent open and the agent pane focused
    let mut s = Screen::new(&["s-1"], "board, agent", 80, 24).await?;
    s.app.navigator_mut().mux.set_focus(1);
    // @step When the mux is drawn
    let rows = s.rows()?;
    // @step Then the first bar row holds the menu items and no button
    assert!(rows[0].contains("[ Kanban ]") && rows[0].contains("[ Help ]"));
    assert!(!rows[0].contains("[ New"), "row 0: {:?}", rows[0]);
    // @step And the second bar row ends with "[ New Bot ]  [ New Agent ]  [ Close ]"
    assert!(ends_with(&rows[1], ALL_BUTTONS), "row 1: {:?}", rows[1]);
    // @step And the panes start on row 2 over a 21-row body
    assert!(
        s.rects().iter().all(|r| r.y == 2 && r.height == 21),
        "{:?}",
        s.rects()
    );
    Ok(())
}

/// Scenario: Close is shown only while the focused pane shows a bot or an agent
#[tokio::test(start_paused = true)]
async fn close_is_shown_only_while_the_focused_pane_shows_a_bot_or_an_agent() -> TestResult {
    // @step Given the terminal is 80x24 and mux is on with the layout "board, agent", one agent open and the agent pane focused
    let mut s = Screen::new(&["s-1"], "board, agent", 80, 24).await?;
    s.app.navigator_mut().mux.set_focus(1);
    s.rows()?;
    let before = s.rects();
    // @step When the user presses Shift+Left to focus the board pane
    s.key(KeyCode::Left, KeyModifiers::SHIFT).await?;
    assert_eq!(s.focus(), 0);
    let rows = s.rows()?;
    // @step Then the second bar row ends with "[ New Bot ]  [ New Agent ]" and shows no "[ Close ]"
    assert!(
        ends_with(&rows[1], "[ New Bot ]  [ New Agent ]"),
        "row 1: {:?}",
        rows[1]
    );
    assert!(!rows[0].contains("[ Close ]") && !rows[1].contains("[ Close ]"));
    // @step And the panes have not moved
    assert_eq!(s.rects(), before);
    Ok(())
}

/// Scenario: New Agent starts an agent in the next agent pane and focuses it
#[tokio::test(start_paused = true)]
async fn new_agent_starts_an_agent_in_the_next_agent_pane_and_focuses_it() -> TestResult {
    // @step Given the terminal is 120x40 and mux is on with the layout "board, agent, agent", one agent open and the board pane focused
    let mut s = Screen::new(&["s-1"], "board, agent, agent", 120, 40).await?;
    s.app.navigator_mut().mux.set_focus(0);
    // @step When the user clicks "[ New Agent ]"
    s.click("[ New Agent ]").await?;
    // @step Then the "Start New Agent?" dialog is shown
    assert!(s.app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    assert!(s.text()?.contains("Start New Agent?"));
    // @step When the user chooses Start
    s.mock.script_create_session(sid("s-2"));
    s.key(KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step Then agent 2 opens in a new pane
    assert_eq!(s.sessions(), 2);
    assert_eq!(
        s.panes(),
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent]
    );
    // @step And the agent 2 pane is focused
    assert_eq!(s.focus(), 2);
    assert_eq!(s.focused_session(), Some(sid("s-2")));
    Ok(())
}

/// Scenario: New Agent with every agent slot filled steps the agent window forward
#[tokio::test(start_paused = true)]
async fn new_agent_with_every_agent_slot_filled_steps_the_agent_window_forward() -> TestResult {
    // @step Given the terminal is 120x40 and mux is on with the layout "board, agent", one agent open and the board pane focused
    let mut s = Screen::new(&["s-1"], "board, agent", 120, 40).await?;
    s.app.navigator_mut().mux.set_focus(0);
    // @step When the user walks the bar ring to "[ New Agent ]", presses Enter and chooses Start
    s.walk_to(MenuFocus::ZoneC(1)).await?;
    s.key(KeyCode::Enter, KeyModifiers::NONE).await?;
    assert!(s.app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    s.mock.script_create_session(sid("s-2"));
    s.key(KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step Then the agent pane shows agent 2
    assert_eq!(s.sessions(), 2);
    assert_eq!(s.app.navigator().mux.window_session_ids(), vec![sid("s-2")]);
    // @step And the agent pane is focused
    assert_eq!(s.focus(), 1);
    assert_eq!(s.focused_session(), Some(sid("s-2")));
    Ok(())
}

/// Scenario: The mux Kanban menu's New Agent row places the new agent like the button
#[tokio::test(start_paused = true)]
async fn the_mux_kanban_menus_new_agent_row_places_the_new_agent_like_the_button() -> TestResult {
    // @step Given the terminal is 120x40 and mux is on with the layout "board, agent, agent", one agent open and the board pane focused
    let mut s = Screen::new(&["s-1"], "board, agent, agent", 120, 40).await?;
    s.app.navigator_mut().mux.set_focus(0);
    // @step When the user runs "New Agent" from the Kanban menu and chooses Start
    s.walk_to(MenuFocus::Item(0)).await?;
    s.key(KeyCode::Enter, KeyModifiers::NONE).await?;
    assert_eq!(s.app.navigator().mux.open_menu(), Some((0, 0)));
    s.key(KeyCode::Enter, KeyModifiers::NONE).await?;
    assert!(s.app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    s.mock.script_create_session(sid("s-2"));
    s.key(KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step Then agent 2 opens in a new pane
    assert_eq!(s.sessions(), 2);
    assert_eq!(s.panes().len(), 3);
    // @step And the agent 2 pane is focused
    assert_eq!(s.focus(), 2);
    assert_eq!(s.focused_session(), Some(sid("s-2")));
    Ok(())
}

/// Scenario: New Bot opens the Start New Bot dialog and Cancel changes nothing
#[tokio::test(start_paused = true)]
async fn new_bot_opens_the_start_new_bot_dialog_and_cancel_changes_nothing() -> TestResult {
    // @step Given the terminal is 120x40 and mux is on with the layout "board, agent", one agent open and the agent pane focused
    let mut s = Screen::new(&["s-1"], "board, agent", 120, 40).await?;
    s.app.navigator_mut().mux.set_focus(1);
    s.rows()?;
    let (panes, rects) = (s.panes(), s.rects());
    // @step When the user clicks "[ New Bot ]"
    s.click("[ New Bot ]").await?;
    // @step Then the "Start New Bot?" dialog is shown
    assert!(s.text()?.contains("Start New Bot?"));
    // @step When the user cancels it
    s.key(KeyCode::Esc, KeyModifiers::NONE).await?;
    assert!(!s.app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    // @step Then no session is started
    assert_eq!(s.sessions(), 1);
    assert!(!s.app.navigator().mux.pending_new_agent());
    // @step And the panes and the focus are as they were
    s.rows()?;
    assert_eq!((s.panes(), s.rects(), s.focus()), (panes, rects, 1));
    Ok(())
}

/// Scenario: Close on an idle agent opens Close Agent and Close closes it
#[tokio::test(start_paused = true)]
async fn close_on_an_idle_agent_opens_close_agent_and_close_closes_it() -> TestResult {
    // @step Given the terminal is 120x40 and mux is on with the layout "board, agent", one idle agent open and the agent pane focused
    let mut s = Screen::new(&["s-1"], "board, agent", 120, 40).await?;
    s.app
        .agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Idle);
    s.app.navigator_mut().mux.set_focus(1);
    // @step When the user walks the bar ring to "[ Close ]" and presses Enter
    s.walk_to(MenuFocus::ZoneC(2)).await?;
    s.key(KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step Then the "Close Agent?" dialog is shown with "Close" and "Cancel"
    assert!(s.app.compositor().contains(CLOSE_DIALOG_ID));
    let text = s.text()?;
    assert!(text.contains("Close Agent?"), "{text}");
    assert!(
        text.contains(" Close ") && text.contains("Cancel"),
        "{text}"
    );
    // @step When the user confirms "Close"
    s.key(KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step Then the agent is closed
    assert_eq!(s.mock.destroyed_sessions(), vec![sid("s-1")]);
    assert_eq!(s.sessions(), 0);
    // @step And only the board pane is left in the grid
    s.rows()?;
    assert_eq!(s.panes(), vec![MuxPaneKind::Board]);
    Ok(())
}

/// Scenario: Close on a bot says its sub-agents close with it
#[tokio::test(start_paused = true)]
async fn close_on_a_bot_says_its_sub_agents_close_with_it() -> TestResult {
    // @step Given the terminal is 120x40 and mux is on with the layout "agent, agent", a bot with two idle sub-agents open and the bot pane focused
    let mut s = Screen::new(&["sub-1", "sub-2", "bot-1"], "agent, agent", 120, 40).await?;
    s.app.dispatch(Action::BotSessionCreated(sid("bot-1")));
    drain(&mut s.app).await?;
    for id in ["sub-1", "sub-2", "bot-1"] {
        s.app
            .agent_view_store_mut()
            .set_session_status(sid(id), SessionStatus::Idle);
    }
    s.mock
        .seed_subordinates_for(sid("bot-1"), vec![sid("sub-1"), sid("sub-2")]);
    let _ = s
        .app
        .navigator_mut()
        .mux
        .focus_agent_pane_for_session(&sid("bot-1"));
    assert_eq!(s.focused_session(), Some(sid("bot-1")));
    // @step When the user clicks "[ Close ]"
    s.click("[ Close ]").await?;
    // @step Then the "Close Bot?" dialog says "Its 2 sub-agents will be closed too."
    let text = s.text()?;
    assert!(text.contains("Close Bot?"), "{text}");
    assert!(
        text.contains("Its 2 sub-agents will be closed too"),
        "{text}"
    );
    // @step And nothing is closed before the user confirms
    assert!(s.mock.destroyed_sessions().is_empty());
    assert_eq!(s.sessions(), 3);
    Ok(())
}

/// Scenario: The bar ring reaches the buttons after the chips
#[tokio::test(start_paused = true)]
async fn the_bar_ring_reaches_the_buttons_after_the_chips() -> TestResult {
    // @step Given the terminal is 120x40 and mux is on with the layout "board, agent", one agent open, the agent pane focused and the bar focus on the last chip
    let mut s = Screen::new(&["s-1"], "board, agent", 120, 40).await?;
    s.app.navigator_mut().mux.set_focus(1);
    let last_chip = s.app.navigator().mux.view_label_count() + s.app.navigator().mux.menu_chips();
    s.walk_to(MenuFocus::ZoneB(last_chip - 1)).await?;
    // @step When the user presses Right three times
    let mut seen = Vec::new();
    for _ in 0..3 {
        s.key(KeyCode::Right, KeyModifiers::NONE).await?;
        seen.push(s.bar_focus());
    }
    // @step Then "[ New Bot ]", "[ New Agent ]" and "[ Close ]" are focused in turn
    assert_eq!(
        seen,
        vec![
            Some(MenuFocus::ZoneC(0)),
            Some(MenuFocus::ZoneC(1)),
            Some(MenuFocus::ZoneC(2))
        ]
    );
    // @step When the user presses Right again
    s.key(KeyCode::Right, KeyModifiers::NONE).await?;
    // @step Then the "Kanban" menu item is focused
    assert_eq!(s.bar_focus(), Some(MenuFocus::Item(0)));
    Ok(())
}

/// Scenario: The mux help lists the buttons
#[tokio::test(start_paused = true)]
async fn the_mux_help_lists_the_buttons() -> TestResult {
    // @step Given mux is on with the layout "board, agent"
    let mut s = Screen::new(&["s-1"], "board, agent", 120, 40).await?;
    // @step When the user submits "/mux help"
    let session = s.app.current_session().or_fail("a current session")?;
    s.app.dispatch(Action::InputSubmitted(
        session.clone(),
        "/mux help".to_string(),
    ));
    drain(&mut s.app).await?;
    // @step Then the help says the bar buttons "[ New Bot ]", "[ New Agent ]" and "[ Close ]" start a bot, start an agent and close the focused pane's bot or agent
    let chunks = s
        .app
        .agent_view_store()
        .session_context_for(&session)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    let text: String = chunks
        .iter()
        .flat_map(|c| c.lines.iter())
        .flat_map(|l| l.spans.iter().map(|span| span.content.to_string()))
        .collect::<Vec<_>>()
        .join(" ");
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    for part in [
        "[ New Bot ] starts a bot",
        "[ New Agent ] starts an agent",
        "[ Close ] closes the focused pane's bot or agent",
    ] {
        assert!(flat.contains(part), "the help lacks {part:?}: {flat}");
    }
    Ok(())
}
