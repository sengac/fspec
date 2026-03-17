@CMPCT-018
Feature: SessionSearch Scoped Turn Range Queries

  """
  Uses existing SessionSearch types, handler, and persistence infrastructure — extends, doesn't replace
  Files: types.rs (add fields), mod.rs (schema), session_search_handler.rs (filter logic + pass-through in create_handler)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Turn range parameters are Optional<usize>, 0-based inclusive indices
  #   2. start_turn without end_turn means from start_turn to end of session
  #   3. end_turn without start_turn means from beginning to end_turn
  #   4. start_turn > end_turn returns empty results (not an error)
  #   5. Turn range filter applies BEFORE max_turns and user_only filters in show
  #   6. Turn range filter applies per-message in search — only matches within range are returned
  #   7. context_turns in search can extend outside the turn range (but only matched turns must be within range)
  #   8. Turn index is the persisted message index (0-based) after system reminders are excluded
  #
  # EXAMPLES:
  #   1. Show with start_turn=10, end_turn=20 on a 50-turn session returns only turns 10-20
  #   2. Show with start_turn=10, no end_turn returns turns 10 through end of session
  #   3. Show with end_turn=5, no start_turn returns turns 0 through 5
  #   4. Show with start_turn=50 on a 20-turn session returns empty messages
  #   5. Show with start_turn=20, end_turn=10 (inverted) returns empty messages
  #   6. Show with turn range + max_turns: first filters to range, then takes last N from filtered set
  #   7. Show with turn range + user_only: filters range first, then applies user_only within range
  #   8. Search with start_turn=0, end_turn=5 only returns matches from turns 0-5 even though later turns also match
  #   9. Search with turn range preserves context_turns that extend outside range
  #   10. JSON deserialization of Show with start_turn and end_turn succeeds
  #   11. JSON deserialization of Search with start_turn and end_turn succeeds
  #   12. Tool definition schema includes start_turn and end_turn as optional integer parameters
  #
  # ========================================

  Background: User Story
    As a AI coding agent
    I want to filter SessionSearch results by turn range
    So that I can drill into specific DAG node turn ranges without retrieving the entire session history

  Scenario: Show returns only turns within specified range
    Given a session with 50 turns of conversation history
    When the agent calls SessionSearch show with start_turn=10 and end_turn=20
    Then the result contains exactly turns 10 through 20
    And each returned message has a turn_index between 10 and 20 inclusive

  Scenario: Show with start_turn only returns from that turn to end
    Given a session with 50 turns of conversation history
    When the agent calls SessionSearch show with start_turn=10 and no end_turn
    Then the result contains turns 10 through 49
    And turns 0 through 9 are excluded

  Scenario: Show with end_turn only returns from beginning to that turn
    Given a session with 50 turns of conversation history
    When the agent calls SessionSearch show with end_turn=5 and no start_turn
    Then the result contains turns 0 through 5
    And turns 6 and above are excluded

  Scenario: Show with start_turn beyond session length returns empty
    Given a session with 20 turns of conversation history
    When the agent calls SessionSearch show with start_turn=50
    Then the result contains zero messages

  Scenario: Show with inverted range returns empty
    Given a session with 50 turns of conversation history
    When the agent calls SessionSearch show with start_turn=20 and end_turn=10
    Then the result contains zero messages
    And the result is not an error

  Scenario: Show applies turn range before max_turns
    Given a session with 50 turns of conversation history
    When the agent calls SessionSearch show with start_turn=10 and end_turn=30 and max_turns=5
    Then the turn range filter reduces to turns 10-30 first
    And max_turns takes the last 5 from the filtered set
    And the result contains exactly 5 messages from the range 26-30

  Scenario: Show applies turn range before user_only
    Given a session with 50 turns alternating user and assistant messages
    When the agent calls SessionSearch show with start_turn=10 and end_turn=20 and user_only=true
    Then only user messages within turns 10-20 are returned
    And user messages outside turns 10-20 are excluded

  Scenario: Search restricts matches to turn range
    Given a session with messages containing "compaction" at turns 3, 15, and 42
    When the agent calls SessionSearch search with query "compaction" and start_turn=0 and end_turn=5
    Then only the match at turn 3 is returned
    And matches at turns 15 and 42 are excluded

  Scenario: Search context_turns can extend outside turn range
    Given a session with a message containing "target" at turn 5
    And the session has 20 turns of context around it
    When the agent calls SessionSearch search with query "target" and start_turn=5 and end_turn=5 and context_turns=2
    Then the match at turn 5 is returned
    And context turns 3, 4, 6, and 7 are included even though they are outside the strict range

  Scenario: Show action deserializes with turn range parameters
    Given a JSON payload with action_type "show" and start_turn 10 and end_turn 20
    When the payload is deserialized into SessionSearchArgs
    Then the Show variant contains start_turn=10 and end_turn=20

  Scenario: Search action deserializes with turn range parameters
    Given a JSON payload with action_type "search" and query "test" and start_turn 0 and end_turn 50
    When the payload is deserialized into SessionSearchArgs
    Then the Search variant contains start_turn=0 and end_turn=50

  Scenario: Tool definition includes turn range parameters in schema
    Given the SessionSearchTool definition
    When the schema is inspected
    Then it includes "start_turn" as an optional integer parameter
    And it includes "end_turn" as an optional integer parameter
    And both parameters mention they apply to show and search actions
