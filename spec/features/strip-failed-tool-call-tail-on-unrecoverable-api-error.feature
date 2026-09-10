@done
@cli
@session
@agent-core
@session-management
@BUG-170
Feature: Unrecoverable API error leaves failed tool call in context — replays same error on next message and skips cache-token invalidation
  """
  Token invalidation at the call site in stream_loop.rs after a successful strip: zero session.token_tracker cache_read_input_tokens/cache_creation_input_tokens and call recalculate_token_tracker(session) (interactive_helpers.rs:205) so input_tokens reflects the trimmed list and output_tokens resets for the new turn.
  New file rust/cli/src/interactive/recovery_unrecoverable.rs (sibling of recovery_image.rs) with pub fn strip_failed_tool_call_tail(messages: &mut Vec<Message>) -> StrippedTail. StrippedTail enum: {ToolPair, ToolCallOnly, TrailingUser, None}. Uses tool_call_correlation_key semantics from interactive_helpers (call_id.unwrap_or(id)) to match the ToolResult to its ToolCall item. Postcondition: validate_no_orphan_tool_calls passes. Wired into stream_loop.rs terminal arm (before emit_error/return Err) when is_prompt_too_long_error(error_str) is true and the tail matches; on success → emit_error with annotation + break (session stays interactive), never return Err.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The strip only runs when the error is a 'prompt too long' / context-length-exceeded error AND the tail of session.messages is a failed tool pair (User(ToolResult) with matching preceding Assistant(ToolCall), or Assistant ending in ToolCall). Any other terminal error keeps today's terminal behavior exactly (emit error, return Err).
  #   2. After a successful strip, stale cache-token state is invalidated: session.token_tracker cache fields (cache_read_input_tokens, cache_creation_input_tokens) are zeroed and input_tokens/output_tokens are recomputed from the trimmed message list (recalculate_token_tracker pattern), so the next turn's pre-prompt estimate and compaction-hook seed are honest.
  #   3. A new surgical primitive strip_failed_tool_call_tail(messages) removes, from the tail of the message stack: (1) a trailing User message whose content is (only) a ToolResult — popping that message AND removing the matching ToolCall item from the preceding Assistant message (keeping any Text items; dropping the whole Assistant message if it becomes empty); (2) else a trailing Assistant message whose items are all ToolCall — popping it; (3) else a trailing plain User prompt — popping it (existing begin_compaction_recovery semantics). It never removes system reminders and never leaves an orphan ToolCall (validate_no_orphan_tool_calls must pass after the strip).
  #   4. The session remains usable after a strip: the stream loop breaks out of the streaming loop (like the EXT-016 image recovery path), surfaces the error to the user via output.emit_error with a short '[removed failed tool call from context]' annotation, and does NOT return Err from the stream loop.
  #
  # EXAMPLES:
  #   1. Context: after a successful strip, session.token_tracker shows cache_read_input_tokens = 0 and cache_creation_input_tokens = 0, and input_tokens equals the recompute over the trimmed message list (the pre-strip value was seeded from a history that no longer exists).
  #   2. Context: session stack ends with ... Assistant(ToolCall "bash", id=abc) User(ToolResult id=abc with 2MB of output); the next API call fails with 'prompt is too long'. After the terminal error arm runs, the stack ends at the message BEFORE the failed pair: Assistant(ToolCall) and User(ToolResult) are gone, validate_no_orphan_tool_calls passes, and the user's next prompt sends a history without the 2MB result.
  #   3. Context: the failed Assistant message carried Text + ToolCall (model narrated, then called a tool that produced the oversized result). After the strip, the Assistant message still contains its Text items, only the ToolCall item is removed, the ToolResult User message is popped, and no new message is created.
  #   4. Context: a transient network error (NOT prompt-too-long) hits the terminal arm with a tool pair at the tail. Nothing is stripped — the terminal behavior is unchanged (error emitted, Err returned), so the strip is surgical and scoped to context-length errors only.
  #
  # ASSUMPTIONS:
  #   1. Persistence (removing the stripped MessageRefs from the manifest so /resume does not replay the pair) is OUT OF SCOPE for this card — the strip operates on the in-memory session.messages stack only; the append-only store has no delete API and that work gets a follow-up card.
  #   2. Strip first, let the existing threshold machinery handle the rest: if the context is still over threshold after stripping the pair, the pre-prompt estimate / compaction hook will catch it on the next call (the pre-prompt gate also counts the trailing tool result once relaxed).
  #
  # ========================================
  Background: User Story
    As a CLI agent session
    I want to strip a failed tool-call pair from the context stack on a terminal API error
    So that the next user message does not replay the oversized tool call and the session stays usable

  Scenario: A failed tool pair at the tail is stripped on a prompt-too-long terminal error
    Given a session message stack ends with an Assistant(ToolCall "bash", id=abc) followed by a User(ToolResult id=abc with a 2MB output)
    When the next API call fails with a "prompt is too long" error
    Then the Assistant(ToolCall) and User(ToolResult) messages are removed from the session message stack
    And the remaining stack passes the no-orphan-tool-calls validation
    And the user's next prompt sends a history that no longer contains the 2MB tool result

  Scenario: A Text plus ToolCall assistant turn keeps its text after the strip
    Given the failing Assistant message contains both a Text item and a ToolCall item (id=xyz)
    And a matching User(ToolResult id=xyz) follows it as the stack tail
    When the terminal error arm runs the strip for a "prompt too long" error
    Then the Assistant message still contains its Text items
    And the ToolCall item is removed from the Assistant message
    And the User(ToolResult) message is popped
    And no new message is created

  Scenario: A trailing tool-call-only assistant message is popped when the tool never produced a result
    Given the session message stack ends with an Assistant message whose items are all ToolCall
    When the terminal error arm runs the strip for a "prompt too long" error
    Then the Assistant message is popped from the stack
    And the remaining stack passes the no-orphan-tool-calls validation

  Scenario: A trailing plain user prompt is popped as a last-resort strip
    Given the session message stack ends with a plain User prompt (no tool content)
    When the terminal error arm runs the strip for a "prompt too long" error
    Then the trailing User prompt is popped
    And no system reminder message is ever removed

  Scenario: System reminders survive the strip
    Given the session message stack starts with system reminder messages
    When the strip removes a failed tool pair from the tail
    Then all system reminder messages remain at the front of the stack, unchanged

  Scenario: Non-prompt-too-long terminal errors leave the stack untouched
    Given a session message stack ends with a failed tool pair (Assistant(ToolCall) + User(ToolResult))
    When the API fails with a transient network error (not prompt-too-long)
    Then no message is removed from the session message stack
    And the terminal behavior is unchanged: the error is emitted and the agent turn returns an error

  Scenario: Stale cache-token state is invalidated after a successful strip
    Given the session token tracker carries cache_read_input_tokens and cache_creation_input_tokens from the last successful request
    When a successful strip removes the failed tool pair from the session message stack
    Then the token tracker cache fields are zeroed (cache_read_input_tokens = 0, cache_creation_input_tokens = 0)
    And input_tokens equals the recompute over the trimmed message list
    And the next turn's pre-prompt estimate and compaction-hook seed are computed from the trimmed history

  Scenario: The session remains interactive after the strip
    Given the strip removes a failed tool pair from the session message stack
    When the stream loop surfaces the terminal error
    Then the error is shown to the user with an annotation that the failed tool call was removed from context
    And the agent turn does not return an error from the stream loop
    And the user can send the next message without the oversized tool call being replayed
