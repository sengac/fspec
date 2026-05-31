@done
@multi-session
@rpc
@history-search
@session-resume
@agent-view
@tui
@RPC-026
Feature: App::dispatch wiring for resume/search mode views (RPC-021c)
  """
  App::dispatch::handle_slash_command in src/app/dispatch_rpc020.rs is amended so SlashCommandAction::Resume dispatches Action::OpenResumeView (which opens the resume_view AND spawns backend.list_sessions() → Action::SessionListLoaded on completion) and SlashCommandAction::Search dispatches Action::OpenSearchView (which opens an empty search_view — no initial backend call). The `[notice] /<name> not yet implemented` arm continues to fire for the OTHER unimplemented SlashCommandAction variants but Resume/Search are real wires now.
  Action::AttachToSession(session_id) handling: if the session is already in AgentViewStore.open_sessions, set current_session_index to it; if NOT, append a fresh SessionContext::new(session_id) and set current_session_index to the new tail. Also publish to active_session_tx so the chunks subscriber re-points and refresh_session_chrome runs (RPC-018) so the new session gets its model + thinking + workspace state. The resume_view is dropped (set to None) as part of this action handler.
  Action::InsertIntoInput(text) handling: replace AgentView.input's value with `text` AND drop the search_view (set to None). Submission is NOT auto-triggered — the user may edit before pressing Enter (mirrors TS /search behaviour). Action::CloseResumeView / Action::CloseSearchView simply null the corresponding Option field without other side-effects.
  """

  # See spec/features/rpc026-* for the broader RPC-026 example-mapping context.
  # This file covers App::dispatch handlers for OpenResumeView/OpenSearchView/AttachToSession/
  # CloseResumeView/InsertIntoInput and the slash-command wiring path.
  Background: User Story
    As a developer using the Rust ratatui TUI
    I want to press /resume or /search (and Ctrl+R) to open full-screen mode views that mirror the TypeScript Ink TUI — listing resumable sessions or filtering submitted-input history with delete confirmation — rather than small floating popups
    So that the Rust frontend's `/resume` and `/search` UX matches the existing TypeScript frontend pixel-for-pixel and feature-for-feature, so habits and integration tests carry across implementations unchanged

  @resume
  @dispatch
  Scenario: Slash command /resume opens the full-screen resume view and spawns list_sessions
    Given AgentView has no popups or mode views open
    And the backend returns ["s1", "s2", "s3"] from list_sessions
    When the user submits "/resume" via the input field
    Then AgentView.slash_popup is None
    And AgentView.resume_view is Some(default ResumeSessionView)
    And a tokio task is spawned that calls backend.list_sessions()
    When the spawned task completes
    Then Action::SessionListLoaded(["s1", "s2", "s3"]) is dispatched
    And resume_view.sessions equals ["s1", "s2", "s3"]
    And resume_view.selected_index equals 0

  @resume
  @attach
  Scenario: Enter on a new session appends it and attaches focus
    Given resume_view is open with sessions ["s-2", "s-3", "s-4"]
    And open_sessions contains exactly SessionContext("s-9") with current_session_index 0
    And resume_view.selected_index is 0
    When the user presses Enter
    Then Action::AttachToSession("s-2") is dispatched
    And AgentView.resume_view is None
    And open_sessions equals [SessionContext("s-9"), SessionContext("s-2")]
    And AgentViewStore.current_session_index equals 1
    And active_session_tx publishes Some(SessionId("s-2"))
    And refresh_session_chrome was called with SessionId("s-2")

  @resume
  @attach
  Scenario: Enter on an already-open session moves focus without duplicating
    Given resume_view is open with sessions ["s-1", "s-2", "s-3"]
    And open_sessions contains [SessionContext("s-1"), SessionContext("s-2"), SessionContext("s-3")] with current_session_index 0
    And resume_view.selected_index is 1
    When the user presses Enter
    Then Action::AttachToSession("s-2") is dispatched
    And open_sessions length stays at 3
    And AgentViewStore.current_session_index equals 1
    And active_session_tx publishes Some(SessionId("s-2"))

  @resume
  @dismiss
  Scenario: Esc closes the resume view without changing the focused session
    Given resume_view is open with sessions ["s-1", "s-2", "s-3"]
    And AgentViewStore.current_session_index is 0
    When the user presses Esc
    Then Action::CloseResumeView is dispatched
    And AgentView.resume_view is None
    And AgentViewStore.current_session_index is unchanged at 0
    And no AttachToSession action was dispatched
    And the next AgentView.render_with_store paints the normal header/scrollback/input/footer layout

  @search
  @dispatch
  Scenario: Slash command /search opens the full-screen search view empty
    Given AgentView has no popups or mode views open
    When the user submits "/search" via the input field
    Then AgentView.slash_popup is None
    And AgentView.search_view is Some(default SearchHistoryView with empty query)
    And no backend call has been made
    When AgentView.render_with_store paints
    Then the header row contains "(search): " followed by an inverse-space block cursor
    And the body shows the placeholder "(type to search history)"

  @search
  @insert
  Scenario: Enter on a highlighted match inserts the text into the input
    Given search_view is open with query "git" and 2 matches with "git status" highlighted
    When the user presses Enter
    Then Action::InsertIntoInput("git status") is dispatched
    And AgentView.search_view is None
    And AgentView.input.value() equals "git status"
    And focus remains on the input
    And NO Action::InputSubmitted was dispatched
