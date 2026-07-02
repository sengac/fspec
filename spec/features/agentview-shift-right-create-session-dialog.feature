@done
@feature-management
@cli
@rust
@agent-view
@tui-component
@tui
@RPC-097
Feature: AgentView Shift+Right does not open CreateSessionDialog and option styling drifts from TS Ink — dialog flag set but never pushed onto compositor
  """
  TS canonical source: src/components/CreateSessionDialog.tsx (135 lines). Rust port target: codelet/fspec-tui/src/components/create_session_dialog.rs (rewrite render). Dispatch wiring: codelet/fspec-tui/src/app/dispatch_session_cycle.rs::handle_session_cycle NavTarget::CreateDialog branch.
  Base dialog primitive (REUSED, not bypassed): codelet/fspec-tui/src/components/dialog_theme.rs — render_dialog + FspecDialog + DialogRow + Accent. CreateSessionDialog::render continues to delegate to render_dialog. Only the DialogRow construction and the FOOTER constant change.
  Mount path (single unified entry point): Action::OpenCreateSessionDialog{preselect} → App::handle_open_create_session_dialog (dispatch_create_session_dialog.rs) → compositor.push(CreateSessionDialog::new(preselect, work_unit).with_action_tx(tx)). Already idempotent on CREATE_SESSION_DIALOG_ID. Shift+Right path is rewired to use this same helper instead of the orphan store-flag setter.
  ratatui Style for selected option button: Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD). Unselected: Style::default().fg(Color::Gray). These match TS Ink's <Text backgroundColor='blue' color='white' bold> / <Text color='gray'>.
  Action variants involved (already defined in components/mod.rs): Action::SessionNext (emitted by AgentView dispatch on Shift+Right), Action::OpenCreateSessionDialog{preselect: Option<CreateSessionOption>}, Action::CreateSessionSubmitted{isolated: bool}, Action::CreateSessionCancelled. No new Action variants required.
  Test fixtures use ratatui::backend::TestBackend with 80x24 and read styled cells via Buffer::get(x,y).style — assert bg/fg/modifier on the exact cells covering ' Yes ' / ' Yes - Isolated ' / ' Cancel '. Snapshots via insta for the rendered glyph layer. End-to-end Shift+Right test feeds KeyEvent{code: Right, modifiers: SHIFT} through App::handle_event and asserts on compositor state.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Shift+Right at the last (or only) open session must result in CreateSessionDialog being pushed onto the compositor — observable via compositor.contains(CREATE_SESSION_DIALOG_ID) == true after App::dispatch returns
  #   2. Shift+Right with zero open sessions also mounts CreateSessionDialog (NavTarget::CreateDialog from navigate_next on an empty store)
  #   3. The dispatch path for NavTarget::CreateDialog is unified with the RPC-060 path: handle_session_cycle calls handle_open_create_session_dialog(None) (or emits Action::OpenCreateSessionDialog{preselect:None}) — NOT the orphan store flag setter
  #   4. CreateSessionDialog rendering EXACTLY matches src/components/CreateSessionDialog.tsx: three centered button cells with labels ' Yes ', ' Yes - Isolated ', ' Cancel ' (one space pad each side)
  #   5. Selected button styling: backgroundColor=Blue, foreground=White, bold; unselected: foreground=Gray, no background, no bold — NO ▸/○ marker glyphs
  #   6. Footer is exactly '← → Select | Enter Confirm | Esc Cancel' using ASCII pipe U+007C (not box-drawing U+2502), rendered dim
  #   7. Title is context-aware: 'Start New Agent?' when current session has no WorkUnitContext; 'Work on <id>?' when it does — sourced from agent_view_store.work_unit_context_for(current_session)
  #   8. Description is context-aware: 'Begin a fresh AI conversation, not linked to any task.' (no work unit) or 'Start an AI session for this task' (with work unit), rendered dim
  #   9. Dialog border is rounded cyan (Accent::Cyan) — already correct in the current Rust port; preserve it
  #   10. Left/Right cycle wraps: Yes → Yes - Isolated → Cancel → Yes; Left wraps the other way — matches TS modulo arithmetic
  #   11. Enter on Yes emits Action::CreateSessionSubmitted{isolated:false}; Enter on Yes - Isolated emits Action::CreateSessionSubmitted{isolated:true}; Enter on Cancel or Esc emits Action::CreateSessionCancelled — and the dialog removes itself via its callback
  #   12. CreateSessionDialog must continue to delegate all rendering to dialog_theme::render_dialog (the canonical base dialog primitive) — no Block/Paragraph hand-rendering
  #   13. User draft in MultiLineInput is preserved across the Shift+Right summon (the user has not switched sessions, only opened a modal) — current RPC-096 contract retained
  #   14. Source-shape: create_session_dialog.rs stays under 300 LoC after the refactor
  #
  # EXAMPLES:
  #   1. Given AgentView with one open session at index 0 (the only session), when the user presses Shift+Right, then the Compositor contains CREATE_SESSION_DIALOG_ID at Priority::Foreground
  #   2. Given AgentView with three open sessions and current_session_index==2 (the last), when the user presses Shift+Right, then CreateSessionDialog is mounted and current_session_index stays 2
  #   3. Given AgentView with zero open sessions, when the user presses Shift+Right, then CreateSessionDialog is mounted with title 'Start New Agent?'
  #   4. Given current session has WorkUnitContext{id:'RPC-097'}, when Shift+Right mounts the dialog, then the rendered title is 'Work on RPC-097?' and the description is 'Start an AI session for this task'
  #   5. Given the dialog is freshly mounted with default selection Yes, the rendered 80x24 buffer shows ' Yes ' painted with bg=Blue/fg=White/bold and ' Yes - Isolated ' and ' Cancel ' painted with fg=Gray — and contains NO ▸ or ○ glyph anywhere
  #   6. Given the dialog is mounted, when the user presses Right, then ' Yes - Isolated ' becomes the blue-bg/white/bold selected button; pressing Right again selects ' Cancel '; pressing Right again wraps to ' Yes '
  #   7. Given the dialog is mounted with default selection Yes, when the user presses Left, then selection wraps to ' Cancel '
  #   8. Given the dialog is mounted with selection on Yes, when the user presses Enter, then Action::CreateSessionSubmitted{isolated:false} is emitted and the dialog is removed from the compositor
  #   9. Given the dialog is mounted with selection on Yes - Isolated, when the user presses Enter, then Action::CreateSessionSubmitted{isolated:true} is emitted (downstream RPC-060 routes this to backend.create_isolated_session)
  #   10. Given the dialog is mounted with selection on Cancel, when the user presses Enter, then Action::CreateSessionCancelled is emitted and the dialog is removed; the original MultiLineInput buffer is untouched
  #   11. Given the dialog is mounted, when the user presses Esc, then Action::CreateSessionCancelled is emitted and the dialog is removed
  #   12. Given the user has typed 'pending' into MultiLineInput at the last session index, when Shift+Right mounts the dialog and the user presses Esc, then 'pending' is still in the MultiLineInput buffer
  #   13. Given the dialog is already mounted, when Shift+Right is pressed again, then handle_open_create_session_dialog is idempotent (no second dialog pushed) — guarded by compositor.contains(CREATE_SESSION_DIALOG_ID)
  #   14. Rendered footer row exactly equals '← → Select | Enter Confirm | Esc Cancel' — ASCII pipe verified by reading the buffer back as a String and grepping for '│' (must be absent) and '|' (must appear three times)
  #   15. After the refactor, codelet/fspec-tui/src/components/create_session_dialog.rs LoC stays under 300 (verified by wc -l in source-shape regression test)
  #
  # ========================================
  Background: User Story
    As a user pressing Shift+Right at the end of the open-sessions list in the Rust AgentView
    I want to summon the Create Session dialog and pick Yes / Yes - Isolated / Cancel
    So that I can start a fresh AI session — bound or unbound to my current work unit — without ever leaving the keyboard

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Shift+Right with a single open session mounts CreateSessionDialog
    Given an App in AgentView with one open session at current_session_index 0
    When the user presses Shift+Right
    Then the Compositor contains CREATE_SESSION_DIALOG_ID
    And the dialog is at Priority::Foreground
    And current_session_index is still 0

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Shift+Right at the last index of three open sessions mounts CreateSessionDialog
    Given an App in AgentView with three open sessions s-1, s-2, s-3 and current_session_index 2
    When the user presses Shift+Right
    Then the Compositor contains CREATE_SESSION_DIALOG_ID
    And current_session_index is still 2

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Shift+Right with zero open sessions mounts CreateSessionDialog with generic title
    Given an App in AgentView with zero open sessions
    When the user presses Shift+Right
    Then the Compositor contains CREATE_SESSION_DIALOG_ID
    And the rendered dialog title is "Start New Agent?"
    And the rendered description is "Begin a fresh AI conversation, not linked to any task."

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Dialog title and description are work-unit-aware when the current session is bound to a work unit
    Given an App in AgentView with one open session bound to WorkUnitContext with id "RPC-097"
    When the user presses Shift+Right
    Then the rendered dialog title is "Work on RPC-097?"
    And the rendered description is "Start an AI session for this task"

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Freshly mounted dialog paints the Yes button blue/white/bold and the other two buttons gray
    Given an App in AgentView with one open session
    When the user presses Shift+Right
    And I render the App onto an 80x24 TestBackend
    Then the cells covering " Yes " have background Color::Blue and foreground Color::White and Modifier::BOLD
    And the cells covering " Yes - Isolated " have foreground Color::Gray
    And the cells covering " Cancel " have foreground Color::Gray
    And no cell in the buffer contains the glyph "▸"
    And no cell in the buffer contains the glyph "○"

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Right arrow cycles selection forward with wrap-around
    Given the CreateSessionDialog is mounted with default selection Yes
    When the user presses Right
    Then the selected option is Yes - Isolated
    When the user presses Right
    Then the selected option is Cancel
    When the user presses Right
    Then the selected option is Yes

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Left arrow wraps from Yes back to Cancel
    Given the CreateSessionDialog is mounted with default selection Yes
    When the user presses Left
    Then the selected option is Cancel

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Enter on Yes emits CreateSessionSubmitted with isolated false and dismisses the dialog
    Given the CreateSessionDialog is mounted with selection Yes
    When the user presses Enter
    Then Action::CreateSessionSubmitted with isolated false is emitted
    And the Compositor no longer contains CREATE_SESSION_DIALOG_ID

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Enter on Yes - Isolated emits CreateSessionSubmitted with isolated true
    Given the CreateSessionDialog is mounted with selection Yes - Isolated
    When the user presses Enter
    Then Action::CreateSessionSubmitted with isolated true is emitted
    And the Compositor no longer contains CREATE_SESSION_DIALOG_ID

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Enter on Cancel emits CreateSessionCancelled and leaves the MultiLineInput buffer untouched
    Given an App in AgentView with one open session and MultiLineInput value "hello"
    And the CreateSessionDialog is mounted with selection Cancel
    When the user presses Enter
    Then Action::CreateSessionCancelled is emitted
    And the Compositor no longer contains CREATE_SESSION_DIALOG_ID
    And the MultiLineInput value is still "hello"

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Esc emits CreateSessionCancelled and dismisses the dialog
    Given the CreateSessionDialog is mounted
    When the user presses Esc
    Then Action::CreateSessionCancelled is emitted
    And the Compositor no longer contains CREATE_SESSION_DIALOG_ID

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Typed MultiLineInput draft survives the Shift+Right summon and subsequent Esc
    Given an App in AgentView with one open session and MultiLineInput value "pending"
    When the user presses Shift+Right
    Then the MultiLineInput value is still "pending"
    When the user presses Esc
    Then the MultiLineInput value is still "pending"

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Shift+Right is idempotent when the dialog is already mounted
    Given an App in AgentView with one open session
    When the user presses Shift+Right
    Then the Compositor contains exactly one CREATE_SESSION_DIALOG_ID instance
    When the user presses Shift+Right again
    Then the Compositor contains exactly one CREATE_SESSION_DIALOG_ID instance

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Rendered footer uses ASCII pipe separators not box-drawing pipes
    Given an App in AgentView with one open session
    When the user presses Shift+Right
    And I render the App onto an 80x24 TestBackend
    Then the rendered buffer contains the string "← → Select | Enter Confirm | Esc Cancel"
    And the rendered buffer does not contain the glyph "│"
    And the ASCII pipe "|" appears in the footer row exactly two times

  @rust
  @tui
  @agent-view
  @tui-component
  @source-shape
  Scenario: Source-shape budget for the refactored CreateSessionDialog
    Given the file codelet/fspec-tui/src/components/create_session_dialog.rs
    Then it has fewer than 300 lines

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: CreateSessionDialog renders via dialog_theme::render_dialog (base dialog primitive reused)
    Given the source of codelet/fspec-tui/src/components/create_session_dialog.rs
    Then it imports render_dialog from super::dialog_theme
    And it does not call ratatui Block or Paragraph directly inside the render function

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: BoardView first Shift+Right with unattached work unit mounts CreateSessionDialog OVER BoardView (no view switch)
    Given an App in BoardView with a selected work unit that has no attached session
    When the user presses Shift+Right once
    Then navigator.active_view is still ViewMode::Board
    And the Compositor contains CREATE_SESSION_DIALOG_ID
    And the dialog overlays the BoardView

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: BoardView first Shift+Right with attached session jumps directly into AgentView without dialog
    Given an App in BoardView with a selected work unit that has an attached session "sid-1"
    When the user presses Shift+Right once
    Then navigator.active_view is ViewMode::Agent
    And agent_view_store.navigation_target is Some("sid-1")
    And the Compositor does not contain CREATE_SESSION_DIALOG_ID

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: Two Shift+Rights from BoardView with unattached work unit remain idempotent
    Given an App in BoardView with a selected work unit that has no attached session
    When the user presses Shift+Right once
    Then the Compositor contains exactly one CREATE_SESSION_DIALOG_ID instance
    And navigator.active_view is still ViewMode::Board
    When the user presses Shift+Right again
    Then the Compositor contains exactly one CREATE_SESSION_DIALOG_ID instance
    And navigator.active_view is still ViewMode::Board

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: BoardView Shift+Right then Esc cancels and leaves the user on BoardView
    Given an App in BoardView with a selected work unit that has no attached session
    When the user presses Shift+Right once
    Then the Compositor contains CREATE_SESSION_DIALOG_ID
    And navigator.active_view is still ViewMode::Board
    When the user presses Esc
    Then the Compositor does not contain CREATE_SESSION_DIALOG_ID
    And navigator.active_view is still ViewMode::Board

  @rust
  @tui
  @agent-view
  @tui-component
  Scenario: BoardView Shift+Right then Enter on Yes switches to AgentView and submits create-session
    Given an App in BoardView with a selected work unit that has no attached session
    When the user presses Shift+Right once
    Then the Compositor contains CREATE_SESSION_DIALOG_ID
    And navigator.active_view is still ViewMode::Board
    When the user presses Enter
    Then Action::CreateSessionSubmitted with isolated false is emitted
    And navigator.active_view is ViewMode::Agent
    And the Compositor does not contain CREATE_SESSION_DIALOG_ID

  # ========================================
  # RPC-097 reopen #2: BoardView Shift+Right MUST consult the global
  # open-session list (mirroring TS sessionGetNext) BEFORE deciding to
  # mount CreateSessionDialog. If any session is already open, resume
  # it instead of asking the user to create another.
  # See spec/attachments/RPC-097/reopen2-active-session-list-not-checked.md
  # ========================================
  @rust
  @tui
  @agent-view
  @tui-component
  @rpc-097-reopen2
  Scenario: BoardView Shift+Right with an already-open session resumes that session instead of showing the dialog
    Given an App in BoardView with a selected work unit that has no attached session
    And the agent_view_store has one open session "sid-A"
    When the user presses Shift+Right once
    Then navigator.active_view is ViewMode::Agent
    And agent_view_store.navigation_target is Some("sid-A")
    And the Compositor does not contain CREATE_SESSION_DIALOG_ID

  @rust
  @tui
  @agent-view
  @tui-component
  @rpc-097-reopen2
  Scenario: BoardView Shift+Right with two open sessions resumes the first one
    Given an App in BoardView with a selected work unit that has no attached session
    And the agent_view_store has two open sessions "sid-A" and "sid-B"
    When the user presses Shift+Right once
    Then navigator.active_view is ViewMode::Agent
    And agent_view_store.navigation_target is Some("sid-A")
    And the Compositor does not contain CREATE_SESSION_DIALOG_ID

  @rust
  @tui
  @agent-view
  @tui-component
  @rpc-097-reopen2
  Scenario: Shift+Left from Agent back to Board then Shift+Right resumes the open session (full round trip)
    Given an App in AgentView with one open session "sid-A" focused
    When the user presses Shift+Left
    Then navigator.active_view is ViewMode::Board
    When the user presses Shift+Right
    Then navigator.active_view is ViewMode::Agent
    And agent_view_store.navigation_target is Some("sid-A")
    And the Compositor does not contain CREATE_SESSION_DIALOG_ID

  @rust
  @tui
  @agent-view
  @tui-component
  @rpc-097-reopen2
  Scenario: BoardView Shift+Right with zero open sessions still mounts CreateSessionDialog (regression guard for reopen #1)
    Given an App in BoardView with a selected work unit that has no attached session
    And the agent_view_store has zero open sessions
    When the user presses Shift+Right once
    Then navigator.active_view is still ViewMode::Board
    And the Compositor contains CREATE_SESSION_DIALOG_ID
