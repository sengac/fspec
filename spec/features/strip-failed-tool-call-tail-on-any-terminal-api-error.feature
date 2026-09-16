@done
@cli
@session
@error-handling
@resilience
@context-management
@BUG-185
Feature: Failed tool-call tail only stripped on prompt-too-long — must strip on ALL terminal API errors
  """
  Wiring: in rust/cli/src/interactive/stream_loop.rs the terminal error arm (after the CLI-022 api.error capture, before the final emit_error/return Err) invokes strip_failed_tool_call_tail(&mut session.messages) UNCONDITIONALLY for any non-interrupted terminal API error — the is_prompt_too_long_error gate (BUG-170 rule [0]) is removed. On a successful ToolPair/ToolCallOnly strip: zero session.token_tracker cache fields, recalculate_token_tracker, reset the live TokenState (compaction_needed=false, cache fields 0, output_tokens 0), emit_error with the '[Removed the failed tool call from context]' annotation, and break (session stays interactive — no Err). When the strip yields None (tail is a plain user prompt or assistant text), fall through to the existing terminal behavior (emit_error + return Err) unchanged. The existing recovery cascade (AMGR-016 stall guard, CMPCT-023 prompt-too-long compaction, EXT-016 image sanitize, PROV-040 truncation retry, NET-001 network retry) runs BEFORE the terminal arm and is untouched. Supersedes BUG-170 rule [0]; BUG-170's 'Non-prompt-too-long terminal errors leave the stack untouched' scenario is reversed by this feature.

  Scope boundary: tool-execution failures are OUT OF SCOPE — the ToolResult arm (handle_tool_result in stream_handlers.rs) keeps pushing the result and the turn continues normally. detect_tool_error stays purely a display/debug-capture signal; it does not gate any removal. The strip primitive (strip_failed_tool_call_tail in recovery_unrecoverable.rs, unchanged) already handles all required tail shapes; only its call-site gating in stream_loop.rs changes, plus the corresponding tests in rust/cli/tests/bug170_strip_failed_tool_call_tail.rs (the 'non-prompt-too-long leaves stack untouched' classifier-gate test and the source-shape 'gated on is_prompt_too_long' assertion are inverted).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The strip runs for EVERY terminal API error (any error class that survives the existing recovery cascade: prompt-too-long, exhausted network retries, exhausted truncation retries, image-content, auth, provider errors) whenever the tail of session.messages is a failed tool pair (User(ToolResult) with matching preceding Assistant(ToolCall)) or a tool-call-only Assistant message. BUG-170 rule [0] is superseded: the is_prompt_too_long_error gate is removed from the terminal arm.
  #   2. Tool-execution failures are NOT stripped: when a tool call executes and returns an error (ToolResult with success=false, non-zero exit code, permission error), the ToolResult stays in context as the model's recovery signal and the turn continues as normal — the strip only ever runs from the terminal API-error arm of the stream loop, never from the ToolResult handler.
  #
  # EXAMPLES:
  #   1. Context: network retry budget (NET-001) is exhausted mid tool-loop; the stack tail is Assistant(ToolCall id=abc) + User(ToolResult id=abc). Today the terminal arm returns Err. After BUG-185: the pair is stripped, cache tokens are invalidated, the error is emitted with the '[Removed the failed tool call from context]' annotation, the loop breaks, and the session stays idle/usable.
  #   2. Context: a tool call executes and returns a failure (e.g. Bash exits with code 1, Read hits a permission error). Nothing is stripped — the tool result stays in context so the model sees the failure and can retry with a different approach. The strip never triggers from tool execution, only from a terminal API error.
  #
  # ========================================
  Background: User Story
    As a CLI agent session
    I want to strip the failed tool-call tail from the context stack on ANY terminal API error
    So that no terminal error leaves a failed tool call in context to be replayed on the next message, and the session stays usable

  Scenario: An exhausted network retry with a failed tool pair at the tail strips the pair and stays interactive
    Given a session message stack ends with an Assistant(ToolCall "bash", id=abc) followed by a User(ToolResult id=abc)
    And the network retry budget is exhausted, producing a terminal error that is NOT a prompt-too-long error
    When the terminal error arm runs
    Then the Assistant(ToolCall) and User(ToolResult) pair are removed from the session message stack
    And the remaining stack passes the no-orphan-tool-calls validation
    And the error is shown to the user with the annotation that the failed tool call was removed from context
    And the agent turn does not return an error from the stream loop
    And the user can send the next message without the failed tool call being replayed

  Scenario: A terminal auth error with a failed tool pair at the tail strips the pair and stays interactive
    Given a session message stack ends with an Assistant(ToolCall "fspec", id=xyz) followed by a User(ToolResult id=xyz)
    And the API fails with a terminal authentication error (not prompt-too-long, not retriable)
    When the terminal error arm runs
    Then the Assistant(ToolCall) and User(ToolResult) pair are removed from the session message stack
    And the stale cache-token state is invalidated (cache fields zeroed, input tokens recomputed over the trimmed history)
    And the session remains interactive with the error surfaced to the user

  Scenario: A trailing tool-call-only assistant message is stripped on any terminal error
    Given the session message stack ends with an Assistant message whose items are all ToolCall
    And the API fails with a terminal provider error that is not prompt-too-long
    When the terminal error arm runs
    Then the tool-call-only Assistant message is popped from the stack
    And the remaining stack passes the no-orphan-tool-calls validation
    And the session remains interactive

  Scenario: A terminal error with no tool tail falls through to the existing terminal behavior
    Given the session message stack ends with an Assistant text message (no trailing tool pair)
    And the API fails with a terminal error that is not prompt-too-long
    When the terminal error arm runs
    Then no message is removed from the session message stack
    And the error is emitted and the agent turn returns an error, exactly as before

  Scenario: A failed tool execution is never stripped
    Given a tool call executes and returns a failure (non-zero exit code, permission error, or success=false result)
    When the tool result is added to the session message stack
    Then the ToolResult stays in context as the model's recovery signal
    And the turn continues normally with no strip invoked from the tool-result handler

  Scenario: The prompt-too-long recovery cascade still takes precedence
    Given a session message stack ends with a failed tool pair
    And the API fails with a "prompt is too long" error
    When the error cascade runs
    Then the existing prompt-too-long compaction-recovery path engages before the terminal arm
    And if that recovery fails to reduce context, the terminal arm strips the pair and keeps the session interactive
