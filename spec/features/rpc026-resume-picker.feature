@done
@RPC-026
@rust
@tui
@agent-view
@multi-session
Feature: RPC-026 ResumePicker popup — session list with Enter-to-select / Esc-to-dismiss

  """
  RPC-026 (resume picker slice) — a centred floating overlay rendered
  above AgentView's MultiLineInput. Shown when the user picks the
  /resume slash command. The picker lists every session reported by
  the backend's list_sessions() call. Selection navigation mirrors the
  RPC-020 SlashCommandPopup / FileSearchPopup pattern: ↑/↓ with
  wrap-around, Enter selects, Tab is ignored (no partial fill makes
  sense), Esc dismisses.

  File: codelet/fspec-tui/src/views/agent/resume_picker.rs (< 300 LoC).
  Owned by AgentView as `Option<ResumePicker>`. Backend interaction is
  done at the App::dispatch layer — the widget itself only knows about
  the SessionInfo list it was handed via `set_sessions`.

  Tests: codelet/fspec-tui/tests/resume_picker_widget_rpc026.rs.
  """

  Background: User Story
    As a fspec TUI user
    I want to open the /resume picker and pick a previous session from a floating overlay
    So that I can attach to an earlier session without leaving the AgentView

  Scenario: A new ResumePicker has no sessions and selected_index == 0
    Given a fresh ResumePicker
    Then resume_picker.session_count() equals 0
    And resume_picker.selected_index() equals 0
    And resume_picker.selected() returns None

  Scenario: set_sessions populates the rows and resets selection to the first row
    Given a fresh ResumePicker
    When resume_picker.set_sessions is called with three SessionInfos in order ["s-1", "s-2", "s-3"]
    Then resume_picker.session_count() equals 3
    And resume_picker.selected_index() equals 0
    And resume_picker.selected() returns Some(SessionInfo with id "s-1")

  Scenario: Down arrow advances selection and wraps around at the end
    Given a ResumePicker populated with three sessions ["s-1", "s-2", "s-3"]
    When the user presses Down
    Then resume_picker.selected_index() equals 1
    When the user presses Down
    Then resume_picker.selected_index() equals 2
    When the user presses Down
    Then resume_picker.selected_index() equals 0

  Scenario: Up arrow walks backward and wraps to the last row
    Given a ResumePicker populated with three sessions ["s-1", "s-2", "s-3"]
    When the user presses Up
    Then resume_picker.selected_index() equals 2
    When the user presses Up
    Then resume_picker.selected_index() equals 1

  Scenario: Enter on a highlighted row emits Selected with the SessionId
    Given a ResumePicker populated with three sessions ["s-1", "s-2", "s-3"]
    When the user presses Down
    And the user presses Enter
    Then handle_key returns ResumePickerOutcome::Selected(SessionId("s-2"))

  Scenario: Enter on an empty session list is ignored
    Given a fresh ResumePicker with zero sessions
    When the user presses Enter
    Then handle_key returns ResumePickerOutcome::Ignored

  Scenario: Esc on the popup returns Dismiss
    Given a ResumePicker populated with one session ["s-1"]
    When the user presses Esc
    Then handle_key returns ResumePickerOutcome::Dismiss

  Scenario: Tab is ignored by the resume picker
    Given a ResumePicker populated with one session ["s-1"]
    When the user presses Tab
    Then handle_key returns ResumePickerOutcome::Ignored

  Scenario: Modifier-prefixed keys are propagated so AgentView can route Shift+arrow chords
    Given a ResumePicker populated with two sessions ["s-1", "s-2"]
    When the user presses Shift+Down
    Then handle_key returns ResumePickerOutcome::Ignored
    And resume_picker.selected_index() is unchanged at 0

  Scenario: Empty session list renders the "(no sessions to resume)" placeholder
    Given a fresh ResumePicker with zero sessions
    When the popup is rendered
    Then the rendered body contains the literal string "(no sessions to resume)"

  Scenario: Populated session list renders one row per SessionInfo
    Given a ResumePicker populated with two sessions [SessionInfo("s-1", "first"), SessionInfo("s-2", "second")]
    When the popup is rendered
    Then the rendered body contains a row referencing "s-1"
    And the rendered body contains a row referencing "s-2"
    And the rendered body contains the navigation hint "↑↓ Navigate │ Enter Attach │ Esc Close"
