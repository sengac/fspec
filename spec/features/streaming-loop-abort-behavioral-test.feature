@done
@session-management
@session
@streaming-loop-detection
@agent-loop
@rust
@high
@RIG-015
Feature: Streaming loop abort behavioral test
  """
  RIG-015: Behavioral proof that the RIG-014 streaming loop detector
  ABORT actually stops the in-flight provider stream and re-prompts the
  LLM. RIG-014 shipped the detector + wiring with only structural
  (source-assertion) integration tests; this card adds a 100% behavioral
  test that drives a real looping stream through the FULL production
  agent loop (codelet_agent_loop::agent_loop) via a real SessionManager +
  BackgroundSession + the test-support "stub" provider arm, and asserts
  the observable outcomes:

  1. The in-flight stream is actually CANCELLED mid-stream — the
  synthetic model's stream is polled far fewer times than its full
  length (the remaining looping tokens are never consumed).
  2. The turn ends via the existing interrupt machinery
  (session.is_interrupted is true; StreamChunk::Interrupted/Done
  arrive on the chunks broadcast).
  3. The persisted assistant message ends with the RIG-014 marker note
  ("Response cut off: repetitive output detected") and does NOT
  contain the degenerate looping tail.
  4. The NEXT turn's context actually carries the corrective note
  (injected as a User message by agent_loop.rs's
  take_pending_loop_abort_note path) — verified by the next
  completion request's chat history containing the note.

  No live provider, no network: the stub provider's StubModel gains a
  test-only configurable stream source (behind the existing
  test-support feature) that yields each configured word as a
  RawStreamingChoice::Message delta (incrementing a shared poll
  counter) followed by a FinalResponse.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The test MUST be behavioral, not structural: it drives a real looping stream through the production stream output path (BackgroundOutput + stream_loop interrupt machinery) and asserts observable outcomes. Source-assertion tests (RIG-014's @integration scenarios) are NOT sufficient for this card.
  #   2. The test drives the REAL production stream driver `codelet_cli::interactive::run_agent_stream_with_images` with a synthetic looping `CompletionModel` (no live provider, no network), a real `BackgroundSession`, and a real `BackgroundOutput` as the StreamOutput sink. It must NOT be a source-assertion test.
  #   3. The synthetic looping model must emit a stream long enough that the detector's abort (session.interrupt()) fires MID-stream, and the test must prove the stream actually STOPS: the model's stream is polled fewer times than its full length, i.e. the remaining looping tokens after the abort are never consumed. This is the core 'stops the stream' guarantee the user wants.
  #   4. The test MUST assert the persisted assistant message (loaded from the session manifest via codelet_core::persistence::manifest::load_session, with the data dir pointed at a tempdir via codelet_common::set_data_directory) ends with the RIG-014 marker note ('Response cut off: repetitive output detected') and does NOT contain the degenerate looping tail — proving the truncation is observable in persisted history, not just in-memory.
  #
  # EXAMPLES:
  #   1. A stub-provider session whose model streams 40 normal words then an unbounded 'the model thinks that' loop: the agent loop's RIG-014 detector fires, session.interrupt() cancels the in-flight stream mid-way (the model's stream is polled far fewer times than its full length), the turn ends, and the persisted assistant message ends with the 'Response cut off: repetitive output detected' marker note without the degenerate tail.
  #   2. After a loop abort, the next turn's context actually carries a corrective note telling the LLM its previous response was cut off due to repetitive output and instructing it to continue with a fresh approach without repeating its earlier reasoning — verified by the next completion request's chat history containing the note.
  #
  # ========================================
  Background: User Story
    As a AI agent session operator
    I want to have the loop detector abort actually stop the in-flight stream and re-prompt the LLM with a corrective note
    So that confidence that the RIG-014 wiring works end-to-end, not just structurally

  @RIG-015
  @streaming-loop-detection
  @end-to-end
  Scenario: Looping stream is cancelled mid-stream and the turn ends
    Given a stub-provider session whose model streams 40 normal words then an unbounded "the model thinks that" loop
    When the agent loop processes the turn
    Then the in-flight provider stream is cancelled before the full stream is consumed
    And the model's stream is polled far fewer times than its full length
    And the session reports interrupted
    And the turn completes with an interrupted/done chunk on the chunks broadcast

  @RIG-015
  @streaming-loop-detection
  @end-to-end
  Scenario: Persisted assistant message is truncated with the marker note
    Given a stub-provider session whose model streams 40 normal words then an unbounded "the model thinks that" loop
    When the agent loop processes the turn and the loop detector aborts
    Then the persisted assistant message contains the normal prose up to the loop onset
    And the persisted assistant message ends with the marker note stating the response was cut off due to repetitive output
    And the persisted assistant message does NOT contain the degenerate looping tail

  @RIG-015
  @streaming-loop-detection
  @end-to-end
  Scenario: Next turn's context carries the corrective note
    Given a session whose previous turn was aborted by the streaming loop detector
    When the agent loop starts the next turn
    Then the next completion request's chat history contains the corrective note
    And the corrective note states the previous response was cut off due to repetitive output
    And the corrective note instructs the model to continue with a fresh approach without repeating its earlier reasoning
