@done
@NAPI-006 @codelet @session-management
Feature: Session Persistence with Fork and Merge

  """
  All persistence operations implemented in Rust, exposed to TypeScript via NAPI-RS bindings
  Uses content-addressed blob storage with SHA-256 hashing for large content deduplication
  Each StoredMessage has unique id and timestamp; only blobs are deduplicated by hash
  JSONL format for history.jsonl ensures append-only crash-safe writes
  File layout: ~/.fspec/messages/ for message store, ~/.fspec/sessions/ for manifests, ~/.fspec/blobs/ for large content
  """

  # ========================================
  # EVENT STORM SUMMARY
  # ========================================
  #
  # BOUNDED CONTEXTS:
  #   - Message Storage: Immutable message store with content-addressed blob extraction for large content
  #   - Session Management: Session manifests, fork/merge operations, lineage tracking
  #   - Command History: Shift+Arrow navigation, history search, per-project filtering
  #
  # DOMAIN EVENTS:
  #   SessionCreated, SessionLoaded, SessionResumed, SessionForked, SessionDeleted, SessionSwitched,
  #   MessageAppended, MessageStored, MessagesMerged, MessagesCherryPicked,
  #   HistoryEntryAdded, BlobStored, SessionCompacted, OrphanedMessagesCleanedUp
  #
  # COMMANDS:
  #   CreateSession, LoadSession, ResumeLastSession, ForkSession, DeleteSession,
  #   ListSessions, SwitchSession, AppendMessage, MergeMessages, CherryPickMessages,
  #   NavigateHistoryUp, NavigateHistoryDown, SearchHistory, RenameSession,
  #   CleanupOrphanedMessages
  #
  # POLICIES:
  #   - Auto-create session on first message
  #   - Store message content in content-addressed store
  #   - Extract large content to blob storage
  #   - Add user input to command history
  #   - Copy message refs on fork (not content)
  #   - Record fork/merge lineage in manifests
  #   - Deduplicate messages by content hash (blobs only)
  #
  # HOTSPOTS (Uncertainties):
  #   - rig::message::Message serialization (need wrapper type)
  #   - Concurrent session access (file locking needed?)
  #   - Blob storage threshold (what size triggers blob extraction?)
  #   - Session file corruption recovery (partial recovery vs fail?)
  #   - History file rotation (truncate when too large?)
  #   - Message pair integrity on merge (user/assistant pairs)
  #
  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Messages are immutable once written; each has unique id and timestamp; only BLOBS are deduplicated by hash
  #   2. Sessions are ordered lists of message references (manifests), not message copies
  #   3. Fork creates a new session sharing message refs up to the fork point, then diverges independently
  #   4. Merge imports message refs from source session into target session without copying message content
  #   5. Cherry-pick imports a message with N preceding context messages for conversation flow
  #   6. Command history is stored globally in history.jsonl for Shift+Arrow navigation across all sessions
  #   7. Large content (file reads, tool results) is stored in blob storage, referenced by SHA-256 hash
  #   8. All persistence operations are implemented in Rust and exposed to TypeScript via NAPI bindings
  #
  # EXAMPLES:
  #   1. User closes terminal with 20-message conversation, reopens next day, runs 'codelet --resume' and continues with full context
  #   2. User at message 5 runs '/fork 3 Alternative approach', creates new session with messages 0-3, can now diverge
  #   3. User runs '/merge session-b 3,4' to import auth solution from session B into current session A
  #   4. User runs '/cherry-pick session-b 7 --context 1' to import Q&A pair (messages 6-7) with context
  #   5. User presses Shift+Arrow-Up to see previous command, Shift+Arrow-Up again to go further back in history
  #   6. User runs '/sessions' to list all sessions for current project with names, message counts, and timestamps
  #   7. User runs Ctrl+R and types 'implement' to search command history, finds matching previous commands
  #   8. Forked session B and original session A both reference same message objects M1-M3 (no duplication)
  #
  # ========================================

  Background: User Story
    As a developer using codelet
    I want to save and restore conversation sessions, fork to try alternative approaches, and merge insights from other conversations
    So that I can resume interrupted work, explore alternatives without losing progress, and reuse valuable context

  @session-resume
  Scenario: Resume session after closing terminal
    Given I have a 20-message conversation with codelet
    And I close the terminal
    When I reopen codelet the next day
    And I run "codelet --resume"
    Then the session should be restored with all 20 messages
    And I can continue the conversation with full context

  @session-fork
  Scenario: Fork session at specific message to try alternative approach
    Given I have a session with 5 messages
    When I run "/fork 3 Alternative approach"
    Then a new session named "Alternative approach" should be created
    And the new session should contain messages 0 through 3
    And the new session can diverge independently from the original

  @session-merge
  Scenario: Merge messages from another session
    Given I have session A as the current session
    And session B contains an auth solution at messages 3 and 4
    When I run "/merge session-b 3,4"
    Then messages 3 and 4 from session B should be imported into session A
    And the imported messages should be marked with their source session

  @session-cherry-pick
  Scenario: Cherry-pick message with preceding context
    Given session B has a question at message 6 and answer at message 7
    When I run "/cherry-pick session-b 7 --context 1"
    Then both messages 6 and 7 should be imported as a Q&A pair
    And the conversation flow should be preserved

  @command-history
  Scenario: Navigate command history with keyboard shortcuts
    Given I have entered commands in the current project across multiple sessions
    When I press Shift+Arrow-Up
    Then I should see my most recent command from the current project
    When I press Shift+Arrow-Up again
    Then I should see the command before that, regardless of which session it was in
    When I press Shift+Arrow-Down
    Then I should return to the more recent command
    When I press Shift+Arrow-Down again
    Then I should return to the empty prompt for new input
    And history is navigated with Shift+Arrow-Up (older) and Shift+Arrow-Down (newer)

  @session-list
  Scenario: List all sessions for current project
    Given I have multiple sessions for the current project
    When I run "/sessions"
    Then I should see a list of sessions with names
    And each session should show message count
    And each session should show timestamps

  @session-switch
  Scenario: Switch to a different session
    Given I have session "Auth Work" as the current session
    And I have session "Bug Fix" available
    When I run "/switch Bug Fix"
    Then the current session should change to "Bug Fix"
    And the context window should load messages from "Bug Fix"
    And I can continue the conversation in "Bug Fix"

  @history-search
  Scenario: Search command history with Ctrl+R
    Given I have command history containing "implement" keyword
    When I press Ctrl+R and type "implement"
    Then I should see matching previous commands
    And I can select a command to reuse

  @message-deduplication
  Scenario: Forked sessions share message references without duplication
    Given I have session A with messages M1, M2, M3
    When I fork session A to create session B
    Then session B should reference the same message objects M1, M2, M3
    And no duplicate copies of messages should be created in storage

  @blob-storage
  Scenario: Large content stored in blob storage with hash reference
    Given I am in a conversation session
    When the assistant reads a file larger than the blob threshold
    Then the file content should be stored in blob storage
    And the message should contain a blob reference with SHA-256 hash
    And the message should contain a preview of the content
    When I resume the session later
    Then the full blob content should be retrievable via the hash reference

  @cross-session-history
  Scenario: Command history accessible across different sessions
    Given I entered "fix login bug" in session B at 10:00am
    And I entered "implement auth flow" in session A at 11:00am
    When I switch to session A
    And I press Shift+Arrow-Up
    Then I should see "implement auth flow" (most recent command)
    When I press Shift+Arrow-Up again
    Then I should see "fix login bug" from session B (older command)
    And history is ordered by timestamp regardless of which session the command was entered in

  @content-deduplication
  Scenario: Identical content in different sessions shares blob storage
    Given I have session A where the assistant read file "/src/main.rs"
    And I have session B where the assistant read the same file "/src/main.rs"
    Then both messages should reference the same blob hash
    And only one copy of the file content should exist in blob storage
    But each session has its own StoredMessage with unique id and timestamp

  @session-delete
  Scenario: Delete session and cleanup orphaned messages
    Given I have session A with messages M1, M2, M3
    And I have session B that was forked from A sharing M1, M2
    When I delete session A
    Then session A should no longer appear in "/sessions"
    And messages M1, M2 should still exist because session B references them
    And message M3 should be marked as orphaned
    When I run "/cleanup-orphans"
    Then message M3 should be removed from storage

  @session-lineage
  Scenario: Session list shows fork lineage
    Given I have session "Main conversation" with 10 messages (indices 0-9)
    And I forked it at message index 4 to create "Alternative approach"
    When I run "/sessions"
    Then I should see "Main conversation" with 10 messages
    And I should see "Alternative approach" with 5 messages (indices 0-4)
    And "Alternative approach" should indicate it was forked from "Main conversation"

  @merge-preserves-references
  Scenario: Merge imports references without duplicating message content
    Given I have session A with messages MA1, MA2
    And I have session B with messages MB1, MB2, MB3
    And the message store contains 5 unique messages
    When I merge messages MB2, MB3 from session B into session A
    Then session A should have 4 message references
    And the message store should still contain only 5 unique messages
    And MB2 and MB3 in session A should reference the same stored messages as in session B

  @error-invalid-fork-index
  Scenario: Fork with invalid index returns clear error
    Given I have a session with 5 messages (indices 0-4)
    When I run "/fork 10 Invalid fork"
    Then I should see an error indicating the fork index is out of range
    And no new session should be created

  @error-invalid-merge-session
  Scenario: Merge from non-existent session returns clear error
    Given I have session A as the current session
    When I run "/merge nonexistent-session 1,2"
    Then I should see an error indicating the source session does not exist
    And session A should remain unchanged

  @error-cherry-pick-insufficient-context
  Scenario: Cherry-pick with context exceeding available messages
    Given session B has only 3 messages (indices 0, 1, 2)
    When I run "/cherry-pick session-b 1 --context 5"
    Then I should see a warning that only 1 context message is available
    And messages 0 and 1 should be imported
    And the operation should complete successfully with reduced context

  @message-immutability
  Scenario: Stored messages cannot be modified
    Given I have a session with a message "Original content" at index 0
    And the message has content hash "abc123"
    When the session is saved and reloaded
    Then message at index 0 should still contain "Original content"
    And the content hash should still be "abc123"
    And there should be no mechanism to alter stored message content

  @history-project-filter
  Scenario: History can be filtered by project
    Given I have history entries from project "/home/user/project-a"
    And I have history entries from project "/home/user/project-b"
    When I am in project "/home/user/project-a"
    And I run "/history"
    Then I should see only history entries from project-a
    When I run "/history --all-projects"
    Then I should see history entries from both projects

  @session-rename
  Scenario: Rename session for better organization
    Given I have a session named "New Session 2025-01-15"
    When I run "/rename Authentication Implementation"
    Then the session name should be updated to "Authentication Implementation"
    And "/sessions" should show the new name
    And the session ID should remain unchanged

  @compaction-state-on-fork
  Scenario: Forking preserves compaction state appropriately
    Given I have session A with 100 messages (indices 0-99)
    And session A was compacted with summary "User discussed auth implementation"
    And compacted_before_index is 80, so messages 0-79 are compacted
    When I fork session A at message index 90
    Then the new session should include the compaction summary
    And the new session should have the summary plus messages 80-90 (11 post-compaction messages)
    And the context window should be properly reconstructed with summary + messages 80-90

  @compaction-resume
  Scenario: Resuming a compacted session reconstructs context correctly
    Given I have session A with 100 messages (indices 0-99) that was compacted
    And the compaction summary is "Previous discussion covered authentication flow"
    And compacted_before_index is 80, so messages 0-79 were compacted and messages 80-99 remain
    When I run "codelet --resume"
    Then the context window should contain the compaction summary
    And the context window should contain messages 80-99 (20 messages)
    And the assistant should have understanding of the compacted context
    And I can continue the conversation seamlessly

  @compaction-state-storage
  Scenario: Compaction state persisted in session manifest
    Given I am in a conversation with 100 messages (indices 0-99)
    When the system performs context compaction at index 80
    Then the session manifest should store the compaction summary
    And the session manifest should record compacted_before_index as 80
    And the session manifest should record the compaction timestamp
    And messages 0-79 remain in storage because the manifest still references them
    And messages 0-79 are not loaded into context window, only the summary is used
    And messages 80-99 are loaded normally into context window

  @compaction-merge
  Scenario: Merging into a compacted session preserves compaction
    Given I have session A with compacted_before_index at 50 (messages 0-49 compacted)
    And session A currently has the summary plus messages 50-60 (11 active messages)
    And session B has messages MB1, MB2 with useful context
    When I run "/merge session-b 0,1"
    Then MB1 and MB2 should be appended after message 60
    And the compaction summary should remain intact
    And the compacted_before_index should remain at 50
    And the merged messages become part of the active (non-compacted) message list

  @compaction-fork-before-compaction-point
  Scenario: Forking at index before compaction point is rejected
    Given I have session A with compacted_before_index at 50 (messages 0-49 compacted)
    And I want to fork at message index 30 which is within the compacted range
    When I run "/fork 30 Pre-compaction fork"
    Then I should see an error that fork index 30 is before compaction boundary 50
    And the error should explain that compacted messages cannot be individually accessed
    And the error should suggest forking at index 50 or later
    And no new session should be created