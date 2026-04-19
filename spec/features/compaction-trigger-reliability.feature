@done
@critical
@context-management
@agent-core
@CMPCT-032
Feature: Compaction triggering broken after CMPCT-023..031 refactor — FinalResponse path bypasses recovery

  """
  CMPCT-027 deleted compaction_retry.rs and moved recovery into an in-loop macro fired only from stream.next() error arms
  stream_loop.rs post-loop block at 1777-1798 is #[cfg(debug_assertions)] only — no production-mode safety net
  Some(Ok(FinalResponse)) branch at stream_loop.rs:968-1295 does not check token_state.compaction_needed before break
  Fix must reinstate production-mode safety net in post-loop AND check flag in FinalResponse branch before emitting done
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. If stream loop exits with token_state.compaction_needed=true AND not interrupted, recovery MUST run
  #   2. FinalResponse branch MUST check compaction_needed before emit_done_with_stop_reason
  #   3. Post-loop safety net MUST be active in release builds (not #[cfg(debug_assertions)])
  #   4. is_interrupted=true MUST take priority over compaction recovery on all paths
  #   5. Error-arm in-loop compaction restart (CMPCT-023..028) MUST remain functional for PromptCancelled, prompt-too-long, and Gemini continuation
  #   6. After execute_compaction runs, compaction_needed MUST reset to false so restart stream does not re-trigger
  #   7. Per-model threshold via resolve_compaction_threshold (Claude=base, Gemini/OpenAI=80%) MUST continue to drive hook firing
  #
  # EXAMPLES:
  #   1. Hook sets compaction_needed on last chunk; stream yields Ok(FinalResponse) with stop_reason=end_turn → recovery runs instead of turn completion
  #   2. Hook sets compaction_needed mid-stream; rig yields PromptError::PromptCancelled → in-loop macro runs recovery
  #   3. Upstream API returns 400 prompt-too-long → in-loop macro runs recovery with EmbedInInstruction(Continue)
  #   4. Gemini continuation exhaustion → in-loop macro runs recovery
  #   5. Thinking-exhaustion retry breaks out of loop with compaction_needed=true → post-loop safety net runs recovery
  #   6. User presses Esc while compaction_needed=true → recovery skipped, interrupt honoured
  #   7. Gemini session hits 80% of context window → hook fires, compaction triggered, session continues cleanly
  #   8. Claude session at user-override threshold → hook fires at override, not default
  #   9. After recovery, restart stream sends 'Continue' and does NOT re-trigger the hook immediately
  #
  # ========================================

  Background: User Story
    As a codelet runtime
    I want to reliably trigger compaction on any stream-loop exit path where compaction_needed becomes true
    So that long-running sessions never silently exceed the context window and fail with 'prompt too long'

  @integration @compaction @regression
  Scenario: FinalResponse branch triggers recovery when compaction_needed is set
    Given a streaming turn is in progress
    And the compaction hook sets token_state.compaction_needed=true on the last chunk
    And the stream yields Ok(FinalResponse) with stop_reason "end_turn"
    When the stream loop processes the FinalResponse branch
    Then begin_compaction_recovery is invoked with policy EmbedInInstruction("Continue")
    And emit_done_with_stop_reason is NOT called for that turn
    And the compaction flow runs to completion and restarts the stream

  @integration @compaction
  Scenario: In-loop macro handles PromptCancelled from mid-stream hook cancellation
    Given a streaming turn is in progress
    And the compaction hook fires mid-stream and cancels the stream
    When rig yields PromptError::PromptCancelled via the error arm
    Then classify_compaction_branch returns Recover
    And in_loop_compaction_restart!() is invoked
    And begin_compaction_recovery runs with policy ResumeFromPartial
    And a fresh stream is built and processed via the retry path

  @integration @compaction
  Scenario: In-loop macro handles upstream prompt-too-long error
    Given a streaming turn is in progress
    And the upstream provider returns 400 prompt-too-long
    When the stream yields an error matching the prompt-too-long classifier
    Then in_loop_compaction_restart!() is invoked
    And begin_compaction_recovery runs with policy EmbedInInstruction("Continue")
    And the retry stream is processed successfully

  @integration @compaction
  Scenario: In-loop macro handles Gemini continuation exhaustion
    Given a Gemini streaming turn is in progress
    And Gemini continuation attempts are exhausted
    When the stream yields the Gemini exhaustion error
    Then in_loop_compaction_restart!() is invoked
    And begin_compaction_recovery runs
    And the retry stream is processed successfully

  @integration @compaction @regression
  Scenario: Post-loop safety net runs recovery when loop exits via break with flag set
    Given a streaming turn is in progress
    And the compaction hook has set token_state.compaction_needed=true
    And the loop breaks via a path that does not invoke the in-loop macro (e.g., thinking-exhaustion retry)
    When the stream loop reaches the post-loop block
    Then in release builds (not only debug_assertions) the safety net fires
    And begin_compaction_recovery is invoked
    And a production-mode warning log identifies which branch missed the check

  @integration @compaction
  Scenario: User interrupt takes priority over compaction recovery
    Given a streaming turn is in progress
    And the compaction hook has set token_state.compaction_needed=true
    And is_interrupted is set to true
    When the stream loop exits via any path
    Then begin_compaction_recovery is NOT invoked
    And the turn terminates with the interrupt state honoured

  @integration @compaction @thresholds
  Scenario: Gemini session triggers compaction at 80% of context window
    Given an active Gemini session with a 1,000,000-token context window
    And resolve_compaction_threshold returns 800,000 tokens for the Gemini family default
    When the session's total input tokens exceed 800,000
    Then the compaction hook fires
    And token_state.compaction_needed becomes true
    And compaction recovery runs on the next stream exit

  @integration @compaction @thresholds
  Scenario: Claude session honours user threshold override
    Given an active Claude session with a user override threshold of 150,000 tokens
    And resolve_compaction_threshold returns the override value
    When the session's total input tokens exceed 150,000
    Then the compaction hook fires at the override threshold, not the default base formula
    And compaction recovery runs

  @integration @compaction
  Scenario: After recovery, restart stream does not immediately re-trigger the hook
    Given begin_compaction_recovery has just completed execute_compaction
    And chat_history has been replaced with the compacted summary
    And token_state.compaction_needed has been reset to false
    When the restart stream sends "Continue" and begins a new turn
    Then the first chunk does not re-trigger the compaction hook
    And the restart stream completes normally
