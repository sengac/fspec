@KGRAPH-008
Feature: Scheduled Indexing via Skills File
  """
  Two responsibilities:
  1. Skills file config parsing (codelet/napi/src/graph/indexing.rs) — parses markdown JSON blocks, validates with defaults
  2. Session scanning pipeline (codelet/napi/src/graph/indexing.rs:scan_and_index_sessions) — reads sessions from persistence layer, extracts structural entities from tool call metadata in messages, loads into nanograph, updates watermarks
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   [5] dispatch_index scope='current' flushes structural extractor queue; scope='all' scans all unindexed sessions via persistence layer
  #   [6] Session scanning reads turns from persistence layer (list_all_sessions + get_session_messages_full), NOT via SessionSearch tool handler
  #   [7] For each session, only turns after the watermark in index-state.json are fetched — watermarks are updated after successful load
  #   [10] Session scanning uses list_sessions_for_project() and get_session_messages_full() from the persistence layer
  #   [13] The index action scans session messages looking for tool call patterns (Write/Edit file_path, Fspec command) and re-runs structural extractors
  #   [14] After successful indexing, update_session_watermark is called with the last indexed turn number
  #   [15] Skills file config parsing remains as-is (already working correctly)
  #
  # EXAMPLES:
  #   [4] dispatch_index(scope='all') with 2 sessions scans only unindexed turns, extracts entities, loads into graph, updates watermarks
  #   [5] dispatch_index(scope='current') still flushes only the pending entity queue — no session scanning
  #   [6] Session message with tool call for Write tool with file_path → produces CodeEntity + Turn + Modifies edge
  #
  # ========================================
  Background: User Story
    As an agent developer
    I want to configure scheduled graph indexing via a skills file so sessions are incrementally indexed
    So that the knowledge graph stays up-to-date without manual intervention

  Scenario: Valid skills file is parsed with correct configuration
    Given a skills markdown file with a JSON config block specifying frequency, batchSize, and extraction mode
    When the skills file is loaded
    Then the config is parsed with the specified frequency, batchSize, and extraction mode

  Scenario: Missing config fields fall back to defaults
    Given a skills markdown file with an empty JSON config block
    When the skills file is loaded
    Then the frequency defaults to "*/15 * * * *"
    And the batchSize defaults to 10
    And the extraction mode defaults to "hybrid"

  Scenario: Incremental indexing processes only unindexed turns
    Given a session with 100 turns and a watermark at turn 80
    When the indexing pipeline runs for that session
    Then only turns 81 through 100 are fetched for processing

  Scenario: Missing skills file does not cause an error
    Given no skills file exists at the expected path
    When the skills file loader is invoked
    Then no indexing schedule is registered
    And no error is raised

  Scenario: Index all scans sessions from persistence and extracts structural entities
    Given a nanograph database is initialized
    And a session exists with 5 messages containing Write and Edit tool calls
    And the session watermark is at turn 0
    When dispatch_index is called with scope "all"
    Then CodeEntity nodes are created for each file path in the tool calls
    And Turn nodes are created for each tool call turn
    And Modifies edges link each Turn to its CodeEntity
    And the session watermark is updated to the last indexed turn

  Scenario: Index all skips fully indexed sessions
    Given a nanograph database is initialized
    And a session exists with 5 messages and watermark at turn 5
    When dispatch_index is called with scope "all"
    Then no new entities are loaded into the graph
    And the response status is "no_unindexed"

  Scenario: Index current only flushes pending entity queue
    Given a nanograph database is initialized
    And the pending entity queue has 3 entities from real-time tool calls
    When dispatch_index is called with scope "current"
    Then only the 3 queued entities are loaded
    And no session scanning occurs
