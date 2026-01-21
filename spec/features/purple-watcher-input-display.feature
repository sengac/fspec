@tui
@watcher-sessions
@done
@WATCH-012
Feature: Purple Watcher Input Display

  """
  ConversationLine role union in src/tui/types/conversation.ts extended with 'watcher' role
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. WatcherInput stream chunks have chunk_type='WatcherInput' with a formatted message in the text field
  #   2. Watcher message format is: [WATCHER: role | Authority: level | Session: id] followed by newline and content
  #   3. parseWatcherPrefix function extracts role, authority, sessionId, and content from the formatted message
  #   4. Watcher input is displayed in magenta (purple) color, distinct from green (user) and white (assistant)
  #   5. Watcher input prefix shows eye emoji and role name: '👁️ RoleName>' followed by the message content
  #   6. ConversationMessage type 'watcher-input' is added to MessageType union for semantic identification
  #   7. ConversationLine role 'watcher' is added for display color determination
  #   8. processChunksToConversation handles WatcherInput chunk type and creates watcher-input messages
  #
  # EXAMPLES:
  #   1. WatcherInput chunk with text='[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]\nSQL injection vulnerability detected' → displays as '👁️ Security Reviewer> SQL injection vulnerability detected' in magenta
  #   2. parseWatcherPrefix('[WATCHER: Code Reviewer | Authority: Peer | Session: xyz-789]\nConsider adding error handling') → { role: 'Code Reviewer', authority: 'Peer', sessionId: 'xyz-789', content: 'Consider adding error handling' }
  #   3. parseWatcherPrefix('Regular user message without prefix') → null (no watcher prefix detected)
  #   4. Multiline watcher message: '[WATCHER: Arch Advisor | Authority: Peer | Session: def-456]\nFirst line\nSecond line\nThird line' → content preserves all three lines after prefix
  #   5. Conversation display: line.role === 'watcher' → baseColor = 'magenta', line.role === 'user' → baseColor = 'green', line.role === 'assistant' → baseColor = 'white'
  #   6. processChunksToConversation receives WatcherInput chunk → creates ConversationMessage with type='watcher-input' containing parsed role and content
  #
  # ========================================

  Background: User Story
    As a user observing a parent session with active watchers
    I want to see watcher injections displayed distinctly in purple with the watcher's role name as a prefix
    So that I can immediately identify which watcher is providing feedback and distinguish watcher input from regular user input

  Scenario: Parse watcher prefix with supervisor authority
    Given a message with watcher prefix "[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]"
    And the message content is "SQL injection vulnerability detected"
    When the parseWatcherPrefix function parses the message
    Then the result should contain role "Security Reviewer"
    And the result should contain authority "Supervisor"
    And the result should contain sessionId "abc-123"
    And the result should contain content "SQL injection vulnerability detected"


  Scenario: Parse watcher prefix with peer authority
    Given a message with watcher prefix "[WATCHER: Code Reviewer | Authority: Peer | Session: xyz-789]"
    And the message content is "Consider adding error handling"
    When the parseWatcherPrefix function parses the message
    Then the result should contain role "Code Reviewer"
    And the result should contain authority "Peer"
    And the result should contain sessionId "xyz-789"
    And the result should contain content "Consider adding error handling"


  Scenario: Parse regular message without watcher prefix
    Given a message "Regular user message without prefix"
    When the parseWatcherPrefix function parses the message
    Then the result should be null


  Scenario: Parse multiline watcher message
    Given a message with watcher prefix "[WATCHER: Arch Advisor | Authority: Peer | Session: def-456]"
    And the message content is multiline:
      """
      First line
      Second line
      Third line
      """
    When the parseWatcherPrefix function parses the message
    Then the result should contain all three lines in content


  Scenario: Display watcher input in magenta color
    Given a ConversationLine with role "watcher"
    When the line is rendered in the conversation view
    Then the base color should be "magenta"
    And it should be distinct from user lines which are "green"
    And it should be distinct from assistant lines which are "white"


  Scenario: Process WatcherInput chunk to conversation message
    Given a StreamChunk with type "WatcherInput"
    And the text field contains "[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]\nVulnerability detected"
    When processChunksToConversation processes the chunk
    Then a ConversationMessage with type "watcher-input" should be created
    And the message content should show "👁️ Security Reviewer> Vulnerability detected"

