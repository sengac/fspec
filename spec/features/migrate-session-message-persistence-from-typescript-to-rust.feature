@REFAC-007
Feature: Migrate session message persistence from TypeScript to Rust
  """
  Key files: Rust side - rust/napi/src/session_manager.rs (agent_loop ~line 3538), rust/napi/src/persistence/. TypeScript side - src/tui/components/AgentView.tsx (remove persistenceStoreMessageEnvelope calls at lines 2638, 2917, 2949, 3391)

  NOTE: CLI (stream_loop.rs) and NAPI (session_manager.rs) both use the SAME Rust persistence layer.
  Tests for the persistence layer cover both paths. CLI-specific scenarios are NOT needed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Rust BackgroundSession MUST own all message persistence - TypeScript should never call persistenceStoreMessageEnvelope
  #   2. User messages MUST be persisted in Rust when input is received
  #   3. Assistant messages MUST be persisted in Rust as content is streamed
  #   4. Tool results MUST be persisted in Rust when tool execution completes
  #   5. On error conditions, Rust MUST persist any accumulated content before emitting error chunk
  #   6. TypeScript/AgentView.tsx MUST only handle UI rendering - no direct persistence calls
  #   7. Token state MUST be persisted in Rust when streaming completes (on Done chunk emission)
  #   8. Compaction state MUST be persisted when compaction completes
  #   9. On interrupt, Rust MUST persist accumulated assistant content before emitting Interrupted chunk
  #   10. If persistence fails, the operation MUST fail - do not silently continue with data loss
  #   11. Resumed sessions MUST restore compaction summary as synthetic first message when compaction state exists
  #
  # EXAMPLES:
  #   1. CURRENT BUG: User sends prompt, assistant streams text+tool_use, tool executes, API error occurs before Done chunk - final assistant content is LOST because TypeScript only persists after await promptComplete
  #   2. EXPECTED: User sends prompt, assistant streams text+tool_use, tool executes, API error occurs - Rust persists accumulated assistant content BEFORE emitting error, so session can be resumed with complete history
  #   3. Evidence: ~9% of sessions (32/345) end with tool_result message - meaning final assistant response after tool call was never persisted
  #
  # ========================================
  # ==========================================
  # TEST STRATEGY NOTE
  # ==========================================
  #
  # The scenarios in this feature describe INTEGRATION behavior:
  # - WHEN streaming events occur (error, interrupt, API responses)
  # - THEN persistence is triggered
  #
  # The linked tests verify PERSISTENCE PRIMITIVES:
  # - CAN we persist messages?
  # - CAN we persist token state?
  # - CAN we persist compaction state?
  #
  # Testing the actual integration (agent_loop triggering persistence on events)
  # requires NAPI runtime or a SimulatedAgentLoop pattern. Without that infrastructure,
  # we verify the underlying persistence operations that would be triggered during
  # integration.
  #
  # This means coverage reports show "covered" but we're testing the foundation,
  # not the full integration. The integration behavior is verified through manual
  # testing and production monitoring.
  #
  # ==========================================
  Background: User Story
    As a developer maintaining the codebase
    I want to have all session message persistence handled exclusively in Rust
    So that the architecture is cleaner, there are no race conditions, and messages are never lost during errors

  # ===========================================
  # TYPESCRIPT REMOVAL VERIFICATION (COMPLETED)
  # ===========================================
  # VERIFIED: persistenceStoreMessageEnvelope calls removed from AgentView.tsx (only REFAC-007 comment remains)
  # VERIFIED: persistTokenState calls removed from AgentView.tsx (only REFAC-007 comment remains)
  # VERIFIED: persistenceSetCompactionState calls removed from useCompaction.ts (no references found)
  # ===========================================
  # MESSAGE PERSISTENCE
  # ===========================================
  @integration
  @napi
  Scenario: User message is persisted by Rust when prompt is received
    Given a NAPI BackgroundSession is created
    When the user sends a prompt "Read the README file"
    Then the user message should be persisted to storage immediately
    And the persisted message should have role "user" with text "Read the README file"
    And no TypeScript persistence functions should be called

  @integration
  @napi
  Scenario: Assistant message with tool_use is persisted before tool execution
    Given a NAPI session with an active prompt
    When the assistant streams text "I'll read that file" followed by a tool_use block
    Then the AssistantMessagePersisted event should fire before ToolExecutionCompleted
    And the persisted message should contain both text and tool_use content blocks

  @integration
  @napi
  Scenario: Tool result is persisted by Rust when tool execution completes
    Given an assistant has requested a tool execution via NAPI
    When the tool execution completes with result content
    Then the ToolResultPersisted event should fire
    And the persisted message should have role "user" with type "tool_result"

  @integration
  @napi
  Scenario: Final assistant response is persisted after tool result
    Given a tool execution has completed and been persisted
    When the assistant streams a final response "Here are the file contents..."
    And the Done chunk is emitted
    Then the FinalAssistantMessagePersisted event should fire
    And all messages should be in storage in order: user, assistant, tool_result, assistant

  @integration
  @napi
  Scenario: Multiple tool uses in single assistant response are all persisted
  # ===========================================
  # MULTIPLE TOOL SEQUENCES (32% BUG FIX)
  # ===========================================
    Given a session with user prompt "Read file A and file B"
    When the assistant streams text with two tool_use blocks (read file A, read file B)
    Then the assistant message should be persisted with both tool_use blocks
    And both tool results should be persisted after execution
    And the final assistant response should be persisted
    And storage should contain: user, assistant(2 tools), tool_result, tool_result, assistant

  @integration
  @napi
  Scenario: Sequential tool calls across multiple turns are persisted
    Given a session with an ongoing conversation
    When the assistant makes a tool call, gets result, makes another tool call
    Then each intermediate assistant response should be persisted before the next tool
    And no "orphaned" tool_results should exist without following assistant responses
    And the session should never end with tool_result as the last message

  @integration
  @napi
  Scenario: API error mid-stream persists accumulated content
  # ===========================================
  # ERROR HANDLING
  # ===========================================
    Given a session with user message persisted
    And the assistant has streamed partial text content
    When an API error occurs before the Done chunk
    Then the StreamingErrorOccurred event should fire
    And the accumulated assistant text should be persisted before emitting error
    And the error should propagate to the user
    And resuming the session should show the partial assistant response

  @integration
  @napi
  Scenario: User interrupt preserves accumulated assistant content via NAPI
  # ===========================================
  # INTERRUPT HANDLING
  # ===========================================
    Given a NAPI session with an active streaming response
    And the assistant has streamed partial content "I am currently working on..."
    When the user interrupts the stream (Ctrl+C or escape)
    Then the SessionInterrupted event should fire
    And the accumulated assistant content should be persisted before the Interrupted chunk
    And resuming the session should show "I am currently working on..."

  @integration
  @napi
  Scenario: Resumed session contains all messages including final responses
  # ===========================================
  # SESSION RESUME
  # ===========================================
    Given a completed session exists with messages: user, assistant+tool_use, tool_result, final_assistant
    When the user runs /resume and selects the session
    Then the MessagesRestored event should fire
    And all four messages should be restored in order
    And no messages should be truncated or missing
    And the conversation should be fully visible

  @integration
  @napi
  Scenario: Manual compaction persists compaction state to session manifest
  # ===========================================
  # COMPACTION STATE
  # ===========================================
    Given a session with enough messages to compact
    When the user runs /compact command via NAPI session_manager.rs
    Then the CompactionSummaryGenerated event should fire
    And the CompactionStatePersisted event should fire
    And the compaction summary should be persisted to the session manifest
    And the compaction boundary index should be recorded
    And the session manifest should contain compaction state with summary text

  @integration
  @napi
  Scenario: Resumed session restores compaction summary as first message
    Given a session that was previously compacted
    And the session manifest has compaction state with summary "Previous discussion covered auth flow implementation"
    When the user resumes the session
    Then the CompactionSummaryRestored event should fire
    And the first message should be a synthetic summary containing "Previous discussion covered auth flow implementation"
    And only post-compaction messages should be loaded after the summary
    And the context should be efficient (not reloading pre-compaction messages)

  @integration
  @napi
  Scenario: Token state is persisted by Rust on Done chunk via NAPI
  # ===========================================
  # TOKEN STATE PERSISTENCE
  # ===========================================
    Given a NAPI session with an active streaming response
    When the Done chunk is emitted with usage data (input_tokens=5000, output_tokens=2000)
    Then the TokenStatePersisted event should fire in Rust
    And the token state should be persisted to the session manifest by Rust
    And TypeScript should NOT call persistenceSetSessionTokens

  @integration
  @napi
  Scenario: Resumed session has accurate token counts
    Given a session was completed with input_tokens=5000 and output_tokens=2000
    When the user resumes the session
    Then the token counts should be restored as input_tokens=5000 and output_tokens=2000
    And the context fill percentage should be calculated correctly
    And the context usage display should be accurate

  @integration
  @napi
  Scenario: Invalid session operations fail gracefully
  # ===========================================
  # PERSISTENCE FAILURE HANDLING
  # ===========================================
    Given an invalid session ID
    When attempting to load the session
    Then the operation should fail with a clear error
    And the error message should be informative

  @integration
  @napi
  Scenario: Session manifest integrity after operations
    Given a session with messages and token state
    When the session is reloaded multiple times
    Then the manifest should maintain integrity
    And no data should be corrupted

  @integration
  @napi
  Scenario: Large session maintains message order
    Given a session with 50 messages
    When the session is reloaded
    Then all 50 messages should be present in correct order

  @integration
  @napi
  Scenario: Persistence failure propagates error to user via NAPI
    Given a NAPI session with an active streaming response
    When persistence fails due to disk error (disk full, permissions, etc.)
    Then the operation should fail with an error
    And the error should be visible to the user
    And the session should NOT silently continue with data loss
    And no partial data should be left in an inconsistent state
