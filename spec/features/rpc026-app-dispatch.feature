@done
@RPC-026
@rust
@tui
@agent-view
@multi-session
@command-history
Feature: RPC-026 App::dispatch wires /resume and /search slash commands and the new Action variants

  """
  RPC-026 (App dispatch slice) — wire the seven new Action variants
  into App::dispatch so the resume / search popups are real, not
  placeholders:

    - SlashCommandAction::Resume → Action::OpenResumePicker → opens
      AgentView.resume_popup AND spawns backend.list_sessions().
    - Action::SessionListLoaded(Vec<SessionInfo>) → folds into the
      open resume_popup via set_sessions.
    - Action::AttachToSession(SessionId) → in-place index move if the
      session already exists in open_sessions, else append a fresh
      SessionContext::new(id). Also publishes to active_session_tx
      and runs refresh_session_chrome (RPC-018).
    - SlashCommandAction::Search → Action::OpenSearchPalette → opens
      AgentView.search_popup with an empty query (no spawn yet).
    - Action::SearchHistory(query) → spawns
      backend.persistence_search_history(query) and dispatches
      Action::HistorySearchResults on success.
    - Action::HistorySearchResults(Vec<HistoryMatch>) → folds into
      the open search_popup via set_matches.
    - Action::InsertIntoInput(text) → sets the MultiLineInput value
      to `text` AND drops the search_popup. Does NOT auto-submit.

  All routing helpers live in codelet/fspec-tui/src/app/dispatch_rpc026.rs
  (under the 300-LoC ceiling). The main dispatch.rs orchestrator gains
  five new match arms that route through them.

  Tests: codelet/fspec-tui/tests/app_dispatch_resume_search_rpc026.rs.
  """

  Background: User Story
    As a developer using the Rust ratatui TUI with the AgentView open
    I want /resume to open a session picker and /search to open a history typeahead
    So that I can attach to a previous session or recall a past command without retyping

  Scenario: SlashCommandAction::Resume opens the resume_popup and spawns list_sessions
    Given an App with the AgentView focused and AgentView.resume_popup is None
    When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Resume)
    Then AgentView.resume_popup is Some(_)
    And AgentView.slash_popup is None
    And the input buffer is reset to ""
    And backend.list_sessions() is invoked exactly once via tokio::spawn

  Scenario: Action::SessionListLoaded folds the result into the open resume_popup
    Given an App with AgentView.resume_popup == Some(ResumePicker)
    When App::dispatch handles Action::SessionListLoaded([SessionInfo("s-1"), SessionInfo("s-2")])
    Then AgentView.resume_popup.session_count() equals 2

  Scenario: Action::SessionListLoaded when the popup is already closed is a no-op
    Given an App with AgentView.resume_popup == None
    When App::dispatch handles Action::SessionListLoaded([SessionInfo("s-1")])
    Then AgentView.resume_popup is still None
    And no panic occurs

  Scenario: Action::AttachToSession to a session already in open_sessions moves the index without appending
    Given an AgentViewStore with open_sessions [SessionContext(id="s-1"), SessionContext(id="s-2"), SessionContext(id="s-3")] and current_session_index == 0
    When App::dispatch handles Action::AttachToSession(SessionId("s-3"))
    Then AgentViewStore.current_session_index equals 2
    And AgentViewStore.open_sessions.len() equals 3
    And active_session_tx receives Some(SessionId("s-3"))
    And refresh_session_chrome is invoked for SessionId("s-3")

  Scenario: Action::AttachToSession to a session NOT in open_sessions appends and focuses it
    Given an AgentViewStore with open_sessions [SessionContext(id="s-1")] and current_session_index == 0
    When App::dispatch handles Action::AttachToSession(SessionId("s-99"))
    Then AgentViewStore.open_sessions.len() equals 2
    And AgentViewStore.open_sessions[1].id equals SessionId("s-99")
    And AgentViewStore.current_session_index equals 1
    And active_session_tx receives Some(SessionId("s-99"))

  Scenario: SlashCommandAction::Search opens the search_popup empty and does NOT spawn
    Given an App with the AgentView focused and AgentView.search_popup is None
    When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Search)
    Then AgentView.search_popup is Some(_)
    And AgentView.search_popup.query() equals ""
    And AgentView.search_popup.match_count() equals 0
    And AgentView.slash_popup is None
    And the input buffer is reset to ""
    And backend.persistence_search_history is NOT invoked

  Scenario: Action::SearchHistory spawns persistence_search_history and folds the result
    Given an App with AgentView.search_popup == Some(SearchPalette)
    When App::dispatch handles Action::SearchHistory("git")
    Then backend.persistence_search_history("git") is invoked exactly once via tokio::spawn
    When the spawned task resolves with [HistoryMatch(text="git status"), HistoryMatch(text="git push")]
    And App::dispatch handles Action::HistorySearchResults([HistoryMatch(text="git status"), HistoryMatch(text="git push")])
    Then AgentView.search_popup.match_count() equals 2
    And AgentView.search_popup.selected_index() equals 0

  Scenario: Action::HistorySearchResults when search_popup is closed is a no-op
    Given an App with AgentView.search_popup == None
    When App::dispatch handles Action::HistorySearchResults([HistoryMatch(text="git status")])
    Then AgentView.search_popup is still None

  Scenario: Action::InsertIntoInput sets the input buffer and drops the search_popup
    Given an App with AgentView.search_popup == Some(SearchPalette) and AgentView.input.value() == ""
    When App::dispatch handles Action::InsertIntoInput("git status")
    Then AgentView.input.value() equals "git status"
    And AgentView.search_popup is None
    And no Action::InputSubmitted is dispatched (the user must press Enter to submit)

  Scenario: handle_slash_command no longer fires the "[notice] not yet implemented" arm for Resume/Search
    Given an App with the AgentView focused
    When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Resume)
    Then no scrollback line containing "/resume not yet implemented" is appended
    When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Search)
    Then no scrollback line containing "/search not yet implemented" is appended

  Scenario: handle_slash_command still fires "[notice]" for the OTHER unimplemented variants
    Given an App with the AgentView focused
    When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Model)
    Then a scrollback line containing "/model not yet implemented" is appended
