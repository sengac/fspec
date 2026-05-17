@done
@agent-view
@RPC-026
@rust
@tui
@source-shape
Feature: RPC-026 source-shape regressions for the resume picker, search palette, and dispatch wiring

  """
  RPC-026 (source-shape slice) — pin the file layout invariants for
  the resume picker / search palette + their App::dispatch wiring:

    - codelet/fspec-tui/src/views/agent/resume_picker.rs exists and is
      under 300 lines.
    - codelet/fspec-tui/src/views/agent/search_palette.rs exists and is
      under 300 lines.
    - codelet/fspec-tui/src/app/dispatch_rpc026.rs exists and is under
      300 lines.
    - codelet/fspec-tui/src/views/agent.rs stays under 300 lines after
      the two new pub Option fields land.
    - codelet/fspec-tui/src/app/dispatch.rs stays under 300 lines after
      the five new match arms land.
    - codelet/fspec-tui/src/app/dispatch_rpc020.rs stays under 300
      lines after the handle_slash_command arms are amended.
    - codelet/fspec-tui/src/components/mod.rs declares the new Action
      variants (OpenResumePicker, OpenSearchPalette, SessionListLoaded,
      AttachToSession, InsertIntoInput, SearchHistory,
      HistorySearchResults).
    - No file under codelet/fspec-tui/src/views/ imports forbidden
      crates (codelet_core, codelet_napi, tarpc, tokio_tungstenite).

  Tests: codelet/fspec-tui/tests/source_shape_rpc026.rs.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI
    I want the RPC-002 source-shape invariants to keep holding after the /resume + /search popups land
    So that no file blows past the 300-LoC ceiling and no view file pulls in the forbidden crates

  Scenario: The resume picker widget lives under views/agent with the right file shape
    Given the codelet/fspec-tui crate
    Then a file exists at codelet/fspec-tui/src/views/agent/resume_picker.rs
    And that file is under 300 lines
    And the file declares "pub struct ResumePicker"
    And the file declares "pub enum ResumePickerOutcome"
    And the file declares "pub fn set_sessions"
    And the file declares "pub fn handle_key"
    And the file declares "pub fn render"

  Scenario: The search palette widget lives under views/agent with the right file shape
    Given the codelet/fspec-tui crate
    Then a file exists at codelet/fspec-tui/src/views/agent/search_palette.rs
    And that file is under 300 lines
    And the file declares "pub struct SearchPalette"
    And the file declares "pub enum SearchPaletteOutcome"
    And the file declares "pub fn set_query"
    And the file declares "pub fn set_matches"
    And the file declares "pub fn handle_key"
    And the file declares "pub fn render"

  Scenario: The new dispatch helpers live in their own dispatch_rpc026.rs file under 300 lines
    Given the codelet/fspec-tui crate
    Then a file exists at codelet/fspec-tui/src/app/dispatch_rpc026.rs
    And that file is under 300 lines
    And the file declares "fn handle_open_resume_picker"
    And the file declares "fn handle_session_list_loaded"
    And the file declares "fn handle_attach_to_session"
    And the file declares "fn handle_open_search_palette"
    And the file declares "fn handle_search_history"
    And the file declares "fn handle_history_search_results"
    And the file declares "fn handle_insert_into_input"

  Scenario: AgentView stays under 300 LoC after the two new Option fields land
    Given codelet/fspec-tui/src/views/agent.rs after RPC-026 lands
    Then the file is under 300 lines
    And the file declares the "resume_popup" field
    And the file declares the "search_popup" field

  Scenario: App dispatch orchestrator stays under 300 LoC after the five new match arms land
    Given codelet/fspec-tui/src/app/dispatch.rs after RPC-026 lands
    Then the file is under 300 lines
    And the file routes "Action::OpenResumePicker" through handle_open_resume_picker
    And the file routes "Action::SessionListLoaded" through handle_session_list_loaded
    And the file routes "Action::AttachToSession" through handle_attach_to_session
    And the file routes "Action::OpenSearchPalette" through handle_open_search_palette
    And the file routes "Action::SearchHistory" through handle_search_history
    And the file routes "Action::HistorySearchResults" through handle_history_search_results
    And the file routes "Action::InsertIntoInput" through handle_insert_into_input

  Scenario: handle_slash_command in dispatch_rpc020.rs is amended to dispatch the new actions for Resume/Search
    Given codelet/fspec-tui/src/app/dispatch_rpc020.rs after RPC-026 lands
    Then the file is under 300 lines
    And the file routes "SlashCommandAction::Resume" through Action::OpenResumePicker
    And the file routes "SlashCommandAction::Search" through Action::OpenSearchPalette

  Scenario: Action enum gains the seven new variants required by RPC-026
    Given codelet/fspec-tui/src/components/mod.rs after RPC-026 lands
    Then the Action enum declares the "OpenResumePicker" variant
    And the Action enum declares the "OpenSearchPalette" variant
    And the Action enum declares the "SessionListLoaded" variant
    And the Action enum declares the "AttachToSession" variant
    And the Action enum declares the "InsertIntoInput" variant
    And the Action enum declares the "SearchHistory" variant
    And the Action enum declares the "HistorySearchResults" variant

  Scenario: No view file imports forbidden crates
    Given the codelet/fspec-tui crate
    Then no file under codelet/fspec-tui/src/views/ imports codelet_core
    And no file under codelet/fspec-tui/src/views/ imports codelet_napi
    And no file under codelet/fspec-tui/src/views/ imports tarpc
    And no file under codelet/fspec-tui/src/views/ imports tokio_tungstenite
