//! Interactive agent selector for `fspec init` (RPC-239, interactive TUI).
//!
//! Feature: spec/features/interactive-agent-selector-for-init.feature
//!
//! Port of the Ink `AgentSelector` component (`src/components/AgentSelector.tsx`)
//! to ratatui. When `fspec init` is run at a real terminal with no `--agent`
//! flag, this renders an inline list of available agents; ↑/↓ move the cursor
//! (clamped, no wrap), ENTER selects, and Esc/q/Ctrl-C cancel. The chosen agent
//! id is then installed through the SAME `codelet_fspec_core::commands::init::run`
//! used by both the CLI bridge and the LLM dispatcher.
//!
//! The navigation state ([`AgentSelectorState`]) and key handling
//! ([`handle_key`]) are pure and unit-tested; the ratatui render + crossterm
//! event loop in [`run_interactive_selector`] are a thin shell around them.

use std::io::{self, Stdout};

use anyhow::Result;
use codelet_fspec_core::commands::init::AgentInfo;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Frame, Terminal};

/// One selectable row: an agent plus whether it was auto-detected in the
/// project root (rendered with a dim `(detected)` suffix).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRow {
    pub id: String,
    pub name: String,
    pub detected: bool,
}

/// Pure navigation state for the selector. Built from the available agent
/// list plus the set of detected agent ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSelectorState {
    pub agents: Vec<AgentRow>,
    pub cursor: usize,
}

/// Result of feeding one key event to the selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorOutcome {
    /// Keep looping (navigation or an ignored key).
    Continue,
    /// The user pressed ENTER on the agent with this id.
    Selected(String),
    /// The user cancelled (Esc / q / Ctrl-C).
    Cancelled,
}

impl AgentSelectorState {
    /// Build the selector state. The initial cursor lands on the FIRST
    /// detected agent (parity with `AgentSelector` `initialCursor =
    /// agents.findIndex(a => a.id === preSelected[0])`); when none are
    /// detected — or the detected id is not in the list — it starts at 0.
    pub fn new(available: Vec<AgentInfo>, detected: &[String]) -> Self {
        let agents: Vec<AgentRow> = available
            .into_iter()
            .map(|a| AgentRow {
                detected: detected.contains(&a.id),
                id: a.id,
                name: a.name,
            })
            .collect();

        let cursor = detected
            .first()
            .and_then(|first| agents.iter().position(|row| &row.id == first))
            .unwrap_or(0);

        Self { agents, cursor }
    }

    /// Move the cursor up one row, clamped at index 0 (no wrap).
    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move the cursor down one row, clamped at the last row (no wrap).
    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.agents.len() {
            self.cursor += 1;
        }
    }

    /// The id of the agent currently under the cursor.
    pub fn current_id(&self) -> &str {
        &self.agents[self.cursor].id
    }
}

/// Apply one key event to the selector state, returning the outcome.
/// ↑/↓ navigate, ENTER selects the current agent, Esc/q/Ctrl-C cancel,
/// everything else is ignored (Continue).
pub fn handle_key(state: &mut AgentSelectorState, key: KeyEvent) -> SelectorOutcome {
    match key.code {
        KeyCode::Up => {
            state.move_up();
            SelectorOutcome::Continue
        }
        KeyCode::Down => {
            state.move_down();
            SelectorOutcome::Continue
        }
        KeyCode::Enter => SelectorOutcome::Selected(state.current_id().to_string()),
        KeyCode::Esc | KeyCode::Char('q') => SelectorOutcome::Cancelled,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            SelectorOutcome::Cancelled
        }
        _ => SelectorOutcome::Continue,
    }
}

/// Render the selector into the inline viewport (parity with the
/// `AgentSelector` JSX: bold title, dim hint, blank line, then one row per
/// agent with a `▶` cursor marker and a dim `(detected)` suffix).
fn render(frame: &mut Frame, state: &AgentSelectorState) {
    let mut lines: Vec<Line> = Vec::with_capacity(state.agents.len() + 3);

    lines.push(Line::from(Span::styled(
        "Select your AI coding agent:",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "(Use ↑↓ to navigate, ENTER to select)",
        Style::default().add_modifier(Modifier::DIM),
    )));
    lines.push(Line::from(""));

    for (index, row) in state.agents.iter().enumerate() {
        let is_cursor = index == state.cursor;
        let marker = if is_cursor { "▶" } else { " " };
        let style = if is_cursor {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let mut spans = vec![Span::styled(format!("{marker} {}", row.name), style)];
        if row.detected {
            spans.push(Span::styled(
                " (detected)",
                Style::default().add_modifier(Modifier::DIM),
            ));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), frame.area());
}

/// Run the interactive selector against the real terminal. Returns
/// `Some(agent_id)` when the user confirms a choice, `None` when they cancel.
///
/// Uses the alternate screen + raw mode (the same lifecycle the ported fspec
/// TUI's `TerminalGuard` uses) so the selector takes over the screen cleanly
/// and the surrounding shell output is restored on exit. The terminal is
/// always restored before returning, even on error.
pub fn run_interactive_selector(state: AgentSelectorState) -> Result<Option<String>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if let Err(err) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(err.into());
    }

    let result = match Terminal::new(CrosstermBackend::new(io::stdout())) {
        Ok(mut terminal) => event_loop(&mut terminal, state),
        Err(err) => Err(err.into()),
    };

    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    disable_raw_mode()?;
    result
}

/// The blocking event loop shared by [`run_interactive_selector`]. Separated so
/// the terminal teardown in the caller always runs.
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut state: AgentSelectorState,
) -> Result<Option<String>> {
    loop {
        terminal.draw(|frame| render(frame, &state))?;

        if let Event::Key(key) = event::read()? {
            // crossterm emits Press, Repeat and Release on some platforms;
            // only act on Press to avoid double-stepping.
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match handle_key(&mut state, key) {
                SelectorOutcome::Continue => {}
                SelectorOutcome::Selected(id) => return Ok(Some(id)),
                SelectorOutcome::Cancelled => return Ok(None),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Feature: spec/features/interactive-agent-selector-for-init.feature
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use assert_cmd::Command;
    use codelet_fspec_core::commands::init::available_agents;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use predicates::prelude::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn selector_starts_on_first_agent_when_none_detected() {
        // @step Given the list of available agents and no detected agents in the project root
        let available = available_agents();
        let detected: Vec<String> = Vec::new();

        // @step When I build the interactive agent selector
        let state = AgentSelectorState::new(available, &detected);

        // @step Then the cursor starts at index 0 and the highlighted agent id is 'claude'
        assert_eq!(state.cursor, 0);
        assert_eq!(state.current_id(), "claude");
    }

    #[test]
    fn selector_preselects_a_detected_agent() {
        // @step Given a project root containing a .cursor directory
        // (detection is exercised end-to-end in the core detect_agents test;
        // here the detected id is supplied directly)
        let available = available_agents();

        // @step When I detect agents and build the interactive agent selector
        let detected = vec!["cursor".to_string()];
        let state = AgentSelectorState::new(available, &detected);

        // @step Then the detected agent id 'cursor' is reported
        assert_eq!(state.current_id(), "cursor");

        // @step And the cursor starts on the 'cursor' row and that row is marked '(detected)'
        let row = &state.agents[state.cursor];
        assert!(row.detected);
        assert_eq!(row.name, "Cursor");
    }

    #[test]
    fn navigation_clamps_at_the_list_bounds() {
        // @step Given an interactive agent selector positioned at index 0
        let mut state = AgentSelectorState::new(available_agents(), &[]);
        assert_eq!(state.cursor, 0);

        // @step When I move the cursor up
        let outcome = handle_key(&mut state, key(KeyCode::Up));
        // @step Then the cursor stays at index 0
        assert_eq!(outcome, SelectorOutcome::Continue);
        assert_eq!(state.cursor, 0);

        // @step When I move the cursor down once
        handle_key(&mut state, key(KeyCode::Down));
        // @step Then the cursor moves to index 1
        assert_eq!(state.cursor, 1);

        // @step When I move the cursor down past the last agent
        for _ in 0..state.agents.len() + 5 {
            handle_key(&mut state, key(KeyCode::Down));
        }
        // @step Then the cursor stays on the last agent
        assert_eq!(state.cursor, state.agents.len() - 1);
    }

    #[test]
    fn cancelling_the_selector_writes_nothing() {
        // @step Given an empty project root directory and a visible interactive selector
        let mut state = AgentSelectorState::new(available_agents(), &[]);

        // @step When I cancel the selector with Esc
        let outcome = handle_key(&mut state, key(KeyCode::Esc));

        // @step Then the selection result is cancelled and no agent files are written
        assert_eq!(outcome, SelectorOutcome::Cancelled);

        // @step And the command exits with code 0 after printing 'Init cancelled'
        // (the exit-0 / "Init cancelled" wiring is asserted by the CLI bridge;
        // a Cancelled outcome is what triggers that branch — no install runs)
        let q = handle_key(&mut state, key(KeyCode::Char('q')));
        assert_eq!(q, SelectorOutcome::Cancelled);
        let ctrl_c = handle_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert_eq!(ctrl_c, SelectorOutcome::Cancelled);
    }

    #[test]
    fn selecting_an_agent_installs_its_files() {
        // @step Given an empty project root directory and the interactive selector positioned on the 'gemini' row
        let tmp = tempfile::TempDir::new().unwrap();
        let mut state = AgentSelectorState::new(available_agents(), &[]);
        let gemini_index = state
            .agents
            .iter()
            .position(|row| row.id == "gemini")
            .expect("gemini is available");
        state.cursor = gemini_index;

        // @step When I confirm the selection and run init with the chosen agent
        let outcome = handle_key(&mut state, key(KeyCode::Enter));
        let selected = match outcome {
            SelectorOutcome::Selected(id) => id,
            other => panic!("expected Selected, got {other:?}"),
        };
        assert_eq!(selected, "gemini");
        let args = serde_json::json!({ "agent": [selected] }).to_string();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let rendered = runtime
            .block_on(codelet_fspec_core::commands::init::run(&args, tmp.path()))
            .expect("init run succeeds");
        assert!(rendered.contains("\"success\":true"));

        // @step Then spec/GEMINI.md is created in the project root
        assert!(tmp.path().join("spec/GEMINI.md").exists());

        // @step And .gemini/commands/fspec.toml is created in the project root
        assert!(tmp.path().join(".gemini/commands/fspec.toml").exists());
    }

    #[test]
    fn non_tty_shell_without_agent_shows_the_tty_guard() {
        // @step Given stdin is not a TTY
        // (assert_cmd spawns the child binary with a non-terminal stdin)
        let mut cmd = Command::cargo_bin("fspec").expect("fspec binary builds");

        // @step When I run the init CLI with no --agent flag
        let assert = cmd.arg("init").assert();

        // @step Then no selector is shown and the output contains 'Interactive mode requires a TTY. Use --agent flag instead:'
        // @step And the command exits with code 1
        assert.failure().code(1).stderr(predicate::str::contains(
            "Interactive mode requires a TTY. Use --agent flag instead:",
        ));
    }
}
