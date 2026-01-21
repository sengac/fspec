@WATCH-011
Feature: Cross-Pane Selection with Correlation IDs

  """
  SplitSessionView builds correlation maps for bi-directional turn highlighting
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. StreamChunk struct in Rust gets a new optional field: correlation_id: Option<String>
  #   2. Parent session assigns correlation_id to each StreamChunk in handle_output() using an atomic counter
  #   3. Correlation IDs are scoped per-session (each session has its own counter starting from 0)
  #   4. When watcher evaluates accumulated observations and produces a response, its output chunks include observed_correlation_ids listing parent chunk IDs that triggered the evaluation
  #   5. SplitSessionView builds a correlation map linking parent chunk correlation_ids to watcher chunk observed_correlation_ids
  #   6. When user selects a turn in parent pane, corresponding watcher turn(s) that observed it are highlighted with a visual indicator
  #   7. When user selects a turn in watcher pane, the parent turn(s) it was observing are highlighted with a visual indicator
  #   8. Cross-pane highlight uses a distinct visual style (e.g., cyan background or border) different from the active selection highlight
  #
  # EXAMPLES:
  #   1. Parent session emits Text chunk → handle_output assigns correlation_id='42' → chunk broadcast to watchers carries correlation_id='42'
  #   2. User selects Turn 3 in parent pane (left) → Turn 5 in watcher pane (right) highlights with cyan indicator because the watcher's Turn 5 was an observation response to parent's Turn 3
  #   3. User selects Turn 5 in watcher pane (right) → Turns 3 and 4 in parent pane (left) highlight with cyan indicator because the watcher observed those turns before responding
  #   4. User selects Turn 1 in watcher pane which is a direct user message to watcher (not an observation) → no highlight appears in parent pane because this turn has no correlation to parent
  #   5. User navigates with Up/Down keys in parent pane in select mode → cross-pane highlight in watcher pane updates immediately as selection moves
  #
  # QUESTIONS (ANSWERED):
  #   Q: The architecture doc shows watcher preserving correlationId when wrapping observations, but the watcher doesn't emit Observation chunks to its UI - it emits its own Text responses. Should watcher responses have an observed_correlation_ids array listing which parent chunks triggered them? Or is there a simpler approach?
  #   A: Yes, watcher responses have observed_correlation_ids array. The array is populated at breakpoint time (Done, ToolResult, or silence timeout) by capturing correlation IDs from all buffered parent chunks before clearing the buffer. This allows precise tracking of which parent content triggered each watcher response.
  #
  #   Q: Should correlation be at chunk level (every Text chunk gets its own ID) or turn level (all chunks in a turn share an ID)? Turn-level seems simpler for UI highlighting since we already group by messageIndex.
  #   A: Chunk-level correlation with turn-level UI aggregation. Each StreamChunk gets its own unique correlation_id assigned by handle_output(). The UI aggregates to turn level by building a map from parent messageIndex to watcher messageIndex using the correlation IDs. This preserves precision in the backend while keeping the UI simple.
  #
  #   Q: What visual style for cross-pane highlight? Options: cyan background on the line, a vertical bar/marker on the left edge, or dimmed/brightened text? The active selection already uses arrow bars (▼▼▼), so this should be visually distinct.
  #   A: Cyan color with bold text and a vertical bar prefix ('│ '). The cross-pane highlighted lines use: color='cyan', bold=true, and prepend '│ ' to content. This is distinct from the active selection (which uses arrow bars ▼▼▼) and clearly visible in the dimmed inactive pane.
  #
  # ========================================

  Background: User Story
    As a user viewing a watcher session in split view
    I want to see visual correlation between parent conversation turns and watcher observations/responses
    So that I understand which parent content the watcher is responding to

  @unit
  Scenario: StreamChunk receives correlation ID in handle_output
    Given a parent session exists
    When the parent session emits a Text chunk via handle_output()
    Then the chunk receives a unique correlation_id assigned by an atomic counter
    And the correlation_id is in format "{session_id}-{counter}"
    And the chunk broadcast to watchers carries the same correlation_id

  Scenario: Parent turn selection highlights correlated watcher turns
    Given I am viewing a watcher session in split view
    And the parent pane shows turns 1, 2, 3 with correlation IDs
    And the watcher responded to turn 3 (watcher turn 5 has observed_correlation_ids pointing to turn 3)
    When I press Tab to enter turn-select mode in parent pane
    And I navigate to select turn 3 in the parent pane
    Then turn 5 in the watcher pane is highlighted with cyan color, bold text, and vertical bar prefix

  Scenario: Watcher turn selection highlights observed parent turns
    Given I am viewing a watcher session in split view
    And the watcher turn 5 observed parent turns 3 and 4 (observed_correlation_ids includes both)
    When I switch to watcher pane and press Tab to enter turn-select mode
    And I navigate to select turn 5 in the watcher pane
    Then turns 3 and 4 in the parent pane are highlighted with cyan color, bold text, and vertical bar prefix

  Scenario: Direct user message to watcher has no correlation
    Given I am viewing a watcher session in split view
    And turn 1 in watcher pane is a direct user message (no observed_correlation_ids)
    When I switch to watcher pane and press Tab to enter turn-select mode
    And I select turn 1 in the watcher pane
    Then no highlight appears in the parent pane
    And the parent pane content remains dimmed

  Scenario: Cross-pane highlight updates as selection moves
    Given I am viewing a watcher session in split view
    And the parent pane has multiple turns with correlated watcher observations
    When I press Tab to enter turn-select mode in parent pane
    And I press Down arrow to move selection to the next turn
    Then the cross-pane highlight in watcher pane updates to show the newly correlated turn
    And the previously highlighted watcher turn returns to dimmed state

  @unit
  Scenario: Watcher evaluation captures observed correlation IDs
    Given a watcher session is observing a parent session
    And the parent emits chunks with correlation_ids "p-0", "p-1", "p-2"
    When a natural breakpoint (Done or ToolResult) triggers watcher evaluation
    Then the ProcessObservations action includes observed_correlation_ids ["p-0", "p-1", "p-2"]
    And the watcher's response chunks can be tagged with these observed IDs
