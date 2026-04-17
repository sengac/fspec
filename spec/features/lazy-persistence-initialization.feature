@BUG-122
Feature: Lazy persistence initialization
  """
  THREE STORES, THREE ACCESS PATTERNS:
  (1) HistoryStore (history.jsonl, 2.6MB) — used by Shift+↑/↓ and /search,
  loads ALL entries cross-session, small enough to keep in memory.
  (2) MessageStore (messages.jsonl, 1GB) — used by SessionSearch and session
  resume, content-addressed by UUID, needs index+seek+LRU.
  (3) SessionStore (4,586 files) — used by session list/resume, individual
  file reads are fine.

  Per-session message sharding was REJECTED because StoredMessage has no
  session_id field — it's content-addressed by UUID. Messages can be shared
  across sessions via Forked/Imported MessageSource.

  The correct approach is:
  1. Per-store lazy initialization (don't init MessageStore when only
  HistoryStore is needed)
  2. Binary index file (messages.idx) mapping UUID → byte offset for
  on-demand seek() loading with LRU cache
  3. TypeScript deferral of persistenceGetHistory() out of initSession()

  Key files: persistence/mod.rs, persistence/storage.rs, AgentView.tsx
  """

  Background: User Story
    As a developer
    I want to start a new session without waiting 5 seconds for loading
    So that I can begin working immediately instead of staring at a loading screen

  @layer-1
  Scenario: Lazy per-store initialization — get_history only inits HistoryStore
    Given the persistence layer has not been initialized
    When get_history() is called
    Then only HistoryStore is initialized
    And MessageStore is NOT initialized
    And SessionStore is NOT initialized

  @layer-1
  Scenario: Lazy per-store initialization — store_message inits MessageStore and SessionStore
    Given the persistence layer has not been initialized
    When a message is stored via append_message()
    Then MessageStore is initialized
    And SessionStore is initialized
    But BlobStore is NOT initialized
    And HistoryStore is NOT initialized

  @layer-1
  Scenario: Lazy per-store initialization — create_session only inits SessionStore
    Given the persistence layer has not been initialized
    When create_session() is called
    Then SessionStore is initialized
    And MessageStore is NOT initialized
    And HistoryStore is NOT initialized

  @layer-2
  Scenario: Session resume loads only that session's messages
    Given a session manifest with 200 message UUIDs
    And a MessageStore with 362000 indexed messages
    When get_session_messages() is called for that session
    Then only 200 messages are loaded from disk via index seek
    And the remaining 361800 messages are not loaded

  @layer-2
  Scenario: SessionSearch loads messages on demand during cross-session search
    Given 10 sessions each with 100 messages
    And a MessageStore with a binary index
    When SessionSearch searches across all sessions with a regex query
    Then messages are loaded per-session via index seek as needed
    And the full 1GB file is NOT loaded into a HashMap

  @cross-session
  Scenario: Shell history recall shows entries from all sessions
    Given 5 sessions exist for the current project with different command histories
    When the developer opens a new session
    And presses Shift+Up to recall history
    Then entries from all 5 previous sessions are available
    And entries are sorted by most recent first

  @cross-session
  Scenario: Search command finds results across all sessions
    Given 3 sessions exist with different command histories
    And session 1 has a command containing "deploy"
    And session 3 has a command containing "deploy production"
    When the developer runs /search and types "deploy"
    Then results from both session 1 and session 3 are shown

  @data-integrity
  Scenario: Content-addressed messages shared via fork are accessible
    Given session A has message UUID-123 via Native source
    And session B references the same UUID-123 via Forked source
    When get_session_messages() is called for session B
    Then message UUID-123 is loaded via index seek
    And its content matches the original message from session A

  @data-integrity
  Scenario: Append and immediate read consistency
    Given a MessageStore with an index
    When a new message is stored via store()
    Then the message is immediately available via get()
    And the in-memory index contains the new entry
    And the binary index is updated on disk
