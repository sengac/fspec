@done
@agent-loop
@rust
@high
@RIG-014
Feature: Streaming LLM loop detector for thinking/text deltas

  """
  New module `rust/agent-loop/src/stream_loop_detector.rs`: pure, stateless-config `StreamLoopDetector` struct with `feed(delta) -> Option<LoopSignal>` and `reset()`. Word-level tokenization (whitespace, lowercase). Four signals: tail n-gram repetition, diversity collapse, long verbatim suffix, drift-tolerant periodicity. Bounded VecDeque window (~96 words). No I/O, no async — trivially unit-testable and proptest-able. Research + POC evidence in spec/attachments (research-streaming-loop-detection.md, poc-streaming-loop-detector.md, llm-repetition-production.pdf, loopllm-energy-latency.pdf).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Detection MUST be streaming (online): the detector consumes text deltas one at a time and can fire mid-stream. It must NOT require the completed response text.
  #   2. The detector uses word-level tokenization (whitespace split, case-insensitive) because the agent loop receives text deltas, not raw provider tokens.
  #   3. The detector maintains a bounded sliding window of words (default 160) and evaluates four signals in order of specificity: (1) tail n-gram repetition — last n words for n in {3,5,8} appear >= 10 times in the window; (2) diversity collapse — unique-word ratio < 0.15 when window >= 40 words; (3) long verbatim suffix — last >= 16 words appear verbatim >= 3 times earlier in the window; (4) drift-tolerant periodicity — last 24 words match >= 85% of the 24 words immediately before them by word-pair ratio.
  #   4. A minimum-evidence guard prevents triggering before 12 words have accumulated in the window, so the detector cannot fire on the first few deltas of a stream.
  #   5. Detector state is per (turn, channel): thinking deltas and text deltas each get their own detector instance with independent windows. A loop in one channel must not be masked by fresh content in the other.
  #   6. Escalation policy: the FIRST trigger emits a non-fatal warning (status event) and streaming continues. A re-trigger within the 30s cooldown window, OR a second distinct signal type firing, escalates to ABORT — the provider stream is cancelled and the looping tail is truncated from persisted assistant content.
  #   7. After an abort, the next turn receives a corrective system note informing the model that its previous response was cut off due to repetitive output and instructing it to continue with a fresh approach without repeating its earlier reasoning.
  #   8. All thresholds (window size, n-gram sizes, repeat count, diversity floor, long-match length, periodicity length and similarity floor, cooldown) are configurable via a config struct with the POC-validated defaults, so per-model tuning is possible without code changes.
  #   9. Keep the content up to the loop onset (the normal part) and drop the degenerate tail. Append a short marker note to the persisted content indicating the response was cut off due to repetitive output, so the session history is honest about what happened.
  #
  # EXAMPLES:
  #   1. A thinking stream that starts normally for ~30 words then locks into 'the model thinks that the model thinks that...' is flagged as NgramRepeat within ~15 words of loop onset, long before the thinking budget is exhausted.
  #   2. A thinking stream that degenerates into 'yes yes yes yes yes...' is aborted mid-stream; the operator sees a status notification that repetitive output was detected and the response was cut short, instead of watching the model spin for minutes.
  #   3. A normal thinking stream that mentions a short phrase twice (e.g. 'please note carefully' repeated once) is NOT flagged — the detector must resist false positives on natural, non-degenerate prose.
  #   4. A text stream that repeats the same ~30-word paragraph verbatim twice is flagged (LongSuffixMatch) and aborted, even though the loop period is much longer than any single n-gram.
  #   5. A thinking stream that repeats a ~24-word reasoning block but changes 1-2 words per cycle (a 'drifting' loop) is still flagged via the periodicity signal — exact-match-only detection would miss this.
  #   6. A text stream that produces a legitimate numbered list ('Step 1: ... Step 2: ... Step 3: ...') with distinct content per item is NOT flagged — repeated structural scaffolding is not a loop.
  #   7. INTEGRATION: the agent loop's stream output path (BackgroundOutput) feeds every Thinking and Text delta into the detector; when the detector escalates to abort, the loop cancels the in-flight provider stream and the next turn begins with the corrective note already in context.
  #
  # QUESTIONS (ANSWERED):
  #   Q: When the detector aborts a looping stream, should the persisted assistant message keep the looping tail (truncated) or drop it entirely?
  #   A: Keep the content up to the loop onset (the normal part) and drop the degenerate tail. Append a short marker note to the persisted content indicating the response was cut off due to repetitive output, so the session history is honest about what happened.
  #
  # ========================================

  Background: User Story
    As a AI agent session (operator)
    I want to detect LLM repetition collapse in real time while provider tokens stream
    So that abort looping generations early instead of burning the full token budget on garbage output

  @RIG-014 @streaming-loop-detection
  Scenario: Short n-gram lock-in in thinking stream is detected mid-stream
    Given a fresh streaming loop detector with default thresholds
    When I feed ~30 words of normal thinking prose
    And I feed deltas that lock into the repeating phrase "the model thinks that the model thinks"
    Then the detector reports a loop signal (n-gram repetition or long verbatim suffix)
    And the signal is reported within 40 words of the loop onset
    And the detector fired before the full stream was consumed

  @RIG-014 @streaming-loop-detection
  Scenario: Single-token spam in thinking stream is detected mid-stream
    Given a fresh streaming loop detector with default thresholds
    When I feed 15 words of normal thinking prose
    And I feed deltas containing only the word "yes" repeated many times
    Then the detector reports a loop signal
    And the signal is reported within 40 words of the spam onset

  @RIG-014 @streaming-loop-detection
  Scenario: Mild one-off phrase repetition is NOT flagged
    Given a fresh streaming loop detector with default thresholds
    When I feed 40 words of normal thinking prose
    And I feed the short phrase "please note carefully" exactly twice
    And I feed 300 more words of normal thinking prose
    Then the detector never reports any loop signal

  @RIG-014 @streaming-loop-detection
  Scenario: Verbatim paragraph repetition in text stream is detected
    Given a fresh streaming loop detector with default thresholds
    When I feed a 30-word paragraph as text deltas
    And I feed the same 30-word paragraph verbatim five more times
    Then the detector reports a long verbatim suffix signal

  @RIG-014 @streaming-loop-detection
  Scenario: Drifting loop with 1-2 words changing per cycle is detected
    Given a fresh streaming loop detector with default thresholds
    When I feed a 24-word reasoning block as thinking deltas
    And I feed the same block again with 2 words changed
    And I feed the same block again with 2 more words changed
    Then the detector reports a periodicity signal
    And the periodicity similarity is at least 0.85

  @RIG-014 @streaming-loop-detection
  Scenario: Legitimate numbered list with distinct items is NOT flagged
    Given a fresh streaming loop detector with default thresholds
    When I feed text deltas producing "Step 1" through "Step 20" each followed by 5 distinct content words
    Then the detector never reports any loop signal

  @RIG-014 @streaming-loop-detection
  Scenario: Detector cannot fire before minimum evidence is accumulated
    Given a fresh streaming loop detector with default thresholds
    When I feed the word "yes" 29 times in a single stream
    Then the detector never reports any loop signal
    When I feed one more "yes"
    Then the detector reports a loop signal

  @RIG-014 @streaming-loop-detection
  Scenario: Thinking and text channels have independent detector state
    Given a streaming loop detector pair with one instance for thinking and one for text
    When I feed a looping stream into the thinking channel
    And I feed fresh non-repeating prose into the text channel
    Then the thinking channel reports a loop signal
    And the text channel never reports any loop signal

  @RIG-014 @streaming-loop-detection
  Scenario: Reset clears detector state for a new turn
    Given a streaming loop detector that has already triggered on a previous turn
    When I reset the detector
    And I feed 100 words of normal prose
    Then the detector never reports any loop signal

  @RIG-014 @streaming-loop-detection
  Scenario: Thresholds are configurable and override the defaults
    Given a streaming loop detector configured with a diversity floor of 0.9
    When I feed 50 words where the unique-word ratio is 0.5
    Then the detector reports a diversity collapse signal
    Given a streaming loop detector with default thresholds
    When I feed the same 50 words
    Then the detector never reports any loop signal

  @RIG-014 @streaming-loop-detection
  Scenario: First trigger warns without aborting
    Given an escalation policy with a 30 second cooldown
    When the detector triggers for the first time
    Then the policy reports a warning
    And the policy does NOT report an abort

  @RIG-014 @streaming-loop-detection
  Scenario: Re-trigger within cooldown escalates to abort
    Given an escalation policy with a 30 second cooldown
    When the detector triggers for the first time
    And the detector triggers again after 10 seconds
    Then the policy reports an abort

  @RIG-014 @streaming-loop-detection
  Scenario: A second distinct signal type escalates to abort
    Given an escalation policy with a 30 second cooldown
    When the detector triggers with an n-gram repetition signal
    And the detector triggers with a diversity collapse signal after 10 seconds
    Then the policy reports an abort

  @RIG-014 @streaming-loop-detection
  Scenario: Re-trigger after cooldown does not escalate
    Given an escalation policy with a 30 second cooldown
    When the detector triggers for the first time
    And the detector triggers again after 60 seconds
    Then the policy reports a warning
    And the policy does NOT report an abort

  @RIG-014 @streaming-loop-detection
  Scenario: Abort truncates the degenerate tail and appends a marker note
    Given a streamed assistant message whose first 100 words are normal prose followed by a looping tail
    When the loop detector aborts the stream
    Then the persisted assistant content contains the normal prose up to the loop onset
    And the persisted assistant content does NOT contain the degenerate tail
    And the persisted assistant content ends with a marker note stating the response was cut off due to repetitive output

  @RIG-014 @streaming-loop-detection
  Scenario: Next turn receives a corrective note after an abort
    Given a session whose previous turn was aborted by the loop detector
    When the agent loop starts the next turn
    Then the turn context includes a corrective note
    And the corrective note states the previous response was cut off due to repetitive output
    And the corrective note instructs the model to continue with a fresh approach without repeating its earlier reasoning

  @RIG-014 @streaming-loop-detection
  Scenario: Agent loop stream path feeds thinking and text deltas into the detector
    Given a background session running the agent loop
    When the provider streams thinking deltas and text deltas for a turn
    Then each thinking delta is fed to the thinking-channel detector
    And each text delta is fed to the text-channel detector
    And the detector windows are reset at the start of each turn

  @RIG-014 @streaming-loop-detection
  Scenario: Agent loop cancels the in-flight provider stream on abort
    Given a background session running the agent loop with an active provider stream
    When the escalation policy reports an abort
    Then the in-flight provider stream is cancelled
    And the turn completes without waiting for the remaining streamed tokens
    And the next turn begins with the corrective note in context
