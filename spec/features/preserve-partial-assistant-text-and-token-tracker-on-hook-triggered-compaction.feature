@done
@CMPCT-024
Feature: Preserve partial assistant text and token tracker on hook-triggered compaction

  """
  Extract a pure helper flush_partial_state_before_compaction(session, assistant_text, display) that performs handle_final_response + token tracker update + clears assistant_text. This is directly unit-testable without spinning up a real rig stream.
  Do NOT emit compaction_started/compaction_progress here — compaction_retry.rs:59-61 (handle_compaction_retry) already emits them right after our break returns. Double-emit would confuse the TUI progress UI. Add a debug! log noting where the events will fire.
  Call site stream_loop.rs:1156-1161 replaces bare break with: clear tool progress callback via set_tool_progress_callback(Uuid::nil(), None); call flush_partial_state_before_compaction helper; then break. Note: gemini_continuation already diverges from handle_compaction_retry by emitting the compaction events itself because it returns GeminiContinuationResult::CompactionNeeded BACK to stream_loop which still re-enters post-loop compaction path. The main stream_loop break doesn't have that extra layer, so emission is still done exactly once (in compaction_retry.rs).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Partial assistant_text accumulated before hook cancellation MUST be appended to session.messages via handle_final_response before breaking
  #   2. Token tracker MUST be flushed with current streaming_display values (via update_from_usage) before breaking
  #   3. The tool progress callback MUST be cleared before breaking (mirrors the post-loop cleanup pattern)
  #   4. compaction_started and compaction_progress events MUST NOT be double-emitted — the authoritative emit site is compaction_retry.rs:59-61 which runs right after the break
  #   5. The assistant_text buffer MUST be cleared after being flushed to avoid double-writing if any downstream path rereads it
  #
  # EXAMPLES:
  #   1. Given partial assistant text 'Here is my analy' accumulated and hook cancels: session.messages gains an Assistant message with that text before compaction runs
  #   2. Given the streaming display has accumulated 1200 input and 340 output tokens before hook cancellation: the session's cumulative billing reflects both values after the break
  #   3. Given assistant_text is empty at cancel time: no extra empty Assistant message is appended to session.messages
  #
  # ========================================

  Background: User Story
    As a user
    I want to have my partially-streamed assistant text preserved when compaction fires mid-turn
    So that I don't lose any of the model's output during automatic context management

  Scenario: Partial assistant text is saved into the conversation before compaction
    Given a session whose last message is a user prompt
    And the streaming loop has accumulated some assistant text in its buffer
    When the compaction hook cancels the current stream
    Then the accumulated assistant text is appended to the session as an Assistant message
    And the assistant text buffer is cleared

  Scenario: Token tracker is flushed with the latest streaming display values
    Given a session with a fresh token tracker
    And the streaming display shows a non-zero input token count
    And the streaming display shows a non-zero output token count
    When the compaction hook cancels the current stream
    Then the session token tracker reflects the streaming display input tokens
    And the session token tracker reflects the streaming display output tokens
    And cumulative billed output accumulation is deferred to a separate card as a pre-existing pattern

  Scenario: Empty partial text does not pollute the conversation
    Given a session whose last message is a user prompt
    And the streaming loop buffer contains no assistant text
    When the compaction hook cancels the current stream
    Then the number of messages in the session is unchanged
    And no Assistant message with empty content is appended
