@done
@AMGR-016
Feature: Subordinate agent hangs indefinitely when DeepSearch sub-agent fails to return

  """
  Primary change in codelet/cli/src/interactive/stream_loop.rs — wrap stream.next() in tokio::time::timeout inside the inner streaming loop
  DeepSearch wall-clock timeout in codelet/napi/src/deep_search_handler.rs — wrap build_and_run_agent() call in tokio::time::timeout
  Drop guard for agent_loop in codelet/napi/src/session_manager.rs — ensure set_status(Idle) executes even on panic via a struct that implements Drop
  The stall timeout error should be a distinct error type (not reuse network/truncation errors) so error_classifiers.rs does not accidentally catch and retry it
  The tokio::select! in the stream loop already has an interrupt_notify branch — the timeout can be added as another branch in the same select
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. stream.next().await in the stream loop must be wrapped in a tokio::time::timeout so that stalled SSE streams are detected and aborted
  #   2. The idle timeout resets on every received streaming chunk (token, tool_call, etc.) — only fires when no data arrives for the full duration
  #   3. When the idle timeout fires, the stream loop must break and return Err so the outer agent_loop sets status to Idle
  #   4. The timeout duration should be configurable but default to 600 seconds
  #   5. After a stall timeout, the error emitted to the UI must clearly indicate it was a stall (not a network error or API error)
  #   6. The existing error classifier cascade must NOT catch stall timeouts — stall is a distinct terminal error that always breaks the loop
  #   7. DeepSearch sub-agent execution must be wrapped in a wall-clock timeout (default 300s) so stalled sub-agents don't block the parent forever
  #   8. The agent_loop post-stream cleanup (set_status(Idle) + emit Done) must ALWAYS execute even if the stream loop panics — use a drop guard or equivalent
  #
  # EXAMPLES:
  #   1. LLM SSE stream stalls after DeepSearch result is injected — stream.next() blocks for 600s — timeout fires — agent transitions to Idle — error message emitted — await_idle caller gets idle result
  #   2. LLM generates tokens normally — each token resets the idle timeout — no timeout fires — agent completes and transitions to Idle normally
  #   3. LLM pauses for 60s between tokens (slow generation) — timeout is 600s — no timeout fires — agent completes successfully
  #   4. DeepSearch sub-agent's LLM stalls during its own generation — wall-clock timeout (300s) fires — parent agent receives timeout error as tool result string — parent continues processing
  #   5. Network error during streaming triggers existing NET-001 retry logic — stall timeout does NOT interfere — retries succeed — agent completes normally
  #   6. Stream loop panics during processing — drop guard fires — agent_loop sets status to Idle — await_idle returns idle
  #   7. LLM starts responding with tokens, then stops mid-sentence for 600s — stall timeout fires — partial text is preserved in history — agent transitions to Idle
  #
  # ========================================

  Background: User Story
    As a supervisor agent
    I want to detect and recover when a subordinate agent's LLM generation stalls indefinitely
    So that subordinates don't hang forever and I get clear error feedback instead of silent timeouts

  @stall-detection
  Scenario: Stalled SSE stream detected and agent recovers to idle
    Given a subordinate agent is running and has received a tool result
    And the agent's stream loop is awaiting the next LLM chunk
    When the LLM SSE stream produces no chunks for 600 seconds
    Then the stream loop should abort with a stall timeout error
    And the agent should transition from running to idle status
    And an error message indicating "generation stalled" should be emitted
    And the supervisor's await_idle should return idle for this agent

  @stall-detection
  Scenario: Normal token generation does not trigger stall timeout
    Given a subordinate agent is running and generating a response
    When the LLM produces tokens continuously with less than 600 seconds between each
    Then no stall timeout should fire
    And the agent should complete its response normally
    And the agent should transition to idle status

  @stall-detection
  Scenario: Slow but active generation does not trigger stall timeout
    Given a subordinate agent is running and generating a response
    When the LLM pauses for 60 seconds between tokens
    And the stall timeout is configured to 600 seconds
    Then no stall timeout should fire
    And the agent should complete its response successfully

  @stall-detection @deep-search
  Scenario: DeepSearch sub-agent stall triggers wall-clock timeout
    Given a subordinate agent invokes a DeepSearch tool
    And the DeepSearch sub-agent's LLM generation stalls indefinitely
    When the DeepSearch wall-clock timeout of 300 seconds expires
    Then the parent agent should receive a timeout error as the tool result string
    And the parent agent should continue processing with the error result
    And the parent agent should not hang or remain in running state

  @stall-detection @integration
  Scenario: Network retry logic is not affected by stall timeout
    Given a subordinate agent is running and streaming a response
    When a transient network error occurs during streaming
    Then the existing NET-001 retry logic should handle the error
    And the stall timeout should not interfere with the retry backoff
    And the agent should complete normally after successful retry

  @stall-detection @safety
  Scenario: Stream loop panic triggers drop guard to restore idle status
    Given a subordinate agent is running and the stream loop is active
    When an unexpected panic occurs in the stream loop
    Then the drop guard should fire and set the agent status to idle
    And the supervisor's await_idle should return idle for this agent

  @stall-detection
  Scenario: Mid-response stall preserves partial text in history
    Given a subordinate agent is running and has received partial response tokens
    When the LLM stops producing tokens for 600 seconds mid-sentence
    Then the stall timeout should fire and abort the generation
    And the partial response text should be preserved in the session history
    And the agent should transition to idle status

  @stall-detection
  Scenario: Stall timeout error is not caught by error classifiers
    Given the stream loop has a stall timeout configured
    When the stall timeout fires due to no tokens received
    Then the error should bypass the error classifier cascade
    And the error should not be retried as a network or truncation error
    And the stream loop should break immediately with a terminal error
