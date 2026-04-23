@done
@context-window
@resilience
@error-handling
@cli
@CMPCT-028
Feature: Correct post-compaction retry prompt semantics
  """
  Architecture notes:
  - CompactionRecoveryPolicy is re-exported via interactive/mod.rs so integration
  tests can import it without reaching into private modules.
  - Path A (pre-prompt compaction) is NOT changed — it never enters
  begin_compaction_recovery; its existing compaction_just_ran → "Continue"
  flow is already correct (no partial text can exist pre-prompt).
  - Path D (Gemini continuation) plumbs the policy through
  GeminiContinuationResult::CompactionNeeded(CompactionRecoveryPolicy) so the
  primary stream loop can honor it during the in-loop restart.
  - The helper flush_partial_state_before_compaction returns Result<bool>
  (true when partial assistant text was appended), which begin_compaction_recovery
  consumes to decide which CompactionRecoveryPolicy to return.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. begin_compaction_recovery MUST return a CompactionRecoveryPolicy that callers thread into the retry prompt selector
  #   2. When no partial assistant text was appended, the policy is EmbedInInstruction and the retry prompt is "Continue"
  #   3. When partial assistant text WAS appended, the policy is ResumeFromPartial and the retry prompt references the preserved work
  #   4. The selected policy MUST be recorded in a debug log entry so operators can audit which branch fired
  #
  # EXAMPLES:
  #   1. Hook cancels before any API tokens are emitted → flush appends nothing → policy is EmbedInInstruction → retry sends "Continue"
  #   2. Hook cancels after partial assistant text was streamed → flush appends Assistant message → policy is ResumeFromPartial → retry sends the resume-from-partial prompt
  #   3. compaction_retry_prompt(EmbedInInstruction) returns "Continue" and compaction_retry_prompt(ResumeFromPartial) returns the resume message
  #
  # ========================================
  Background: User Story
    As a fspec user recovering from an in-loop post-compaction restart
    I want to see the retry prompt chosen explicitly based on whether partial assistant text was preserved
    So that the LLM resumes the original task correctly instead of getting the ambiguous hardcoded "Continue" signal

  Scenario: flush_partial_state_before_compaction reports whether partial assistant text was appended
    Given a session with some accumulated partial assistant text in the streaming buffer
    When flush_partial_state_before_compaction is invoked with that buffer
    Then the helper returns true to indicate an Assistant message was appended
    And the session contains the partial text as a new Assistant message

  Scenario: flush_partial_state_before_compaction reports no append when the buffer is empty
    Given a session with an empty partial assistant text buffer
    When flush_partial_state_before_compaction is invoked with that buffer
    Then the helper returns false to indicate no Assistant message was appended
    And the session message count is unchanged

  Scenario: begin_compaction_recovery returns EmbedInInstruction when no partial text exists
    Given a session whose streaming loop has not yet emitted any assistant text
    When begin_compaction_recovery runs for a hook-cancel compaction
    Then it returns CompactionRecoveryPolicy::EmbedInInstruction
    And a debug log records that the EmbedInInstruction policy was selected

  Scenario: begin_compaction_recovery returns ResumeFromPartial when partial text was preserved
    Given a session whose streaming loop accumulated partial assistant text before the hook cancelled
    When begin_compaction_recovery runs for that hook-cancel compaction
    Then it returns CompactionRecoveryPolicy::ResumeFromPartial
    And a debug log records that the ResumeFromPartial policy was selected

  Scenario: compaction_retry_prompt maps EmbedInInstruction to the literal "Continue" string
    Given the EmbedInInstruction policy
    When compaction_retry_prompt is invoked
    Then the returned prompt is exactly "Continue"

  Scenario: compaction_retry_prompt maps ResumeFromPartial to a resume prompt that references the preserved work
    Given the ResumeFromPartial policy
    When compaction_retry_prompt is invoked
    Then the returned prompt mentions continuing from where the assistant left off before the context limit was reached
