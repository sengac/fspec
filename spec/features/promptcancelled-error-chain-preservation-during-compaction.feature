@done
@codelet
@BUG-144
@error-handling
@resilience
@agent-core
@context-management
@compaction
Feature: PromptCancelled error chain preservation during compaction
  """
  Architecture notes:
  - The bug: anyhow::anyhow!("Streaming error: {e}") in rig_agent.rs destroys
  the typed error chain by formatting via Display into a bare string.
  Only the Display output is captured, not the original type.
  - Fix: Replace with anyhow::Error::from(e) which preserves the typed chain.
  - StreamingError already has a Display impl that includes "Streaming error"
  prefix, so Error::from(e) preserves both the typed chain AND the Display.
  - No changes needed in error_classifiers.rs production code — the existing downcast logic
  (extract_prompt_cancelled) works once the type chain is preserved.
  - The false positive test was corrected to use Error::from instead of .into().
  - The existing test detects_streaming_error_wrapped_prompt_cancelled previously used .into()
  which was a false positive — it now uses Error::from matching the production code path.
  """

  Background: User Story
    As a developer
    I want PromptCancelled errors to be detected during compaction
    So that session termination is avoided and graceful recovery can occur

  Scenario: StreamingError::Prompt with PromptCancelled is detected via Error::from conversion
    Given a StreamingError::Prompt containing Box(PromptError::PromptCancelled) is created
    When the error is converted using the production path anyhow::Error::from(e)
    Then extract_prompt_cancelled returns Some(chat_history)
    And the typed error chain is preserved for downstream downcast_ref extraction

  Scenario: Bare string errors are not mistaken for PromptCancelled
    Given an anyhow::Error created via anyhow::Error::msg("PromptCancelled")
    When is_compaction_cancelled is called on that error
    Then the function returns false
    And bare string errors are correctly rejected as non-typed cancellations

  Scenario: Error::from path preserves downcast capability for extract_prompt_cancelled
    Given a StreamingError::Prompt(Box(PromptError::PromptCancelled)) is created
    When the error is converted via anyhow::Error::from(e) which matches production
    Then the error chain preserves the original StreamingError and PromptError types
    And extract_prompt_cancelled successfully downcasts and returns Some(chat_history)
