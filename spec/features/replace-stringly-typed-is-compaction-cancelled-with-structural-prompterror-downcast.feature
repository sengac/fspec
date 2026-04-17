@CMPCT-025
Feature: Replace stringly-typed is_compaction_cancelled with structural PromptError downcast

  """
  Uses anyhow::Error::chain() to walk the entire source chain; at each link, attempts downcast_ref::<PromptError>() to match the typed variant. This preserves the public signature while replacing stringly-typed substring matching.
  Introduces extract_prompt_cancelled(&anyhow::Error) -> Option<&Vec<rig::message::Message>> as the structural primitive; is_compaction_cancelled delegates to extract_prompt_cancelled(e).is_some() so both helpers share one code path.
  Imports rig::completion::PromptError (re-exported via rig::completion::*) and rig::message::Message. Does NOT need to match StreamingError directly because StreamingError::Prompt wraps Box<PromptError> and the chain walks into it.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. is_compaction_cancelled must use structural downcast via anyhow chain traversal, not string matching
  #   2. bare anyhow::Error::msg("PromptCancelled") must NOT be detected as compaction cancellation; only typed PromptError variants count
  #   3. PromptError::PromptCancelled wrapped in StreamingError::Prompt or .context() must still be detected
  #   4. public signature of is_compaction_cancelled(&anyhow::Error) -> bool must be preserved
  #
  # EXAMPLES:
  #   1. anyhow::Error::from(PromptError::PromptCancelled{...}) returns true from is_compaction_cancelled
  #   2. anyhow::Error::msg("PromptCancelled") returns false from is_compaction_cancelled
  #   3. StreamingError::Prompt wrapping PromptCancelled returns true from is_compaction_cancelled
  #   4. PromptError::MaxDepthError returns false from is_compaction_cancelled
  #
  # ========================================

  Background: User Story
    As a developer
    I want to detect compaction cancellations robustly
    So that compaction recovery works reliably even when errors are wrapped

  Scenario: Direct PromptError::PromptCancelled is detected
    Given an anyhow::Error built directly from PromptError::PromptCancelled
    When is_compaction_cancelled is called with that error
    Then the function returns true
    And extract_prompt_cancelled returns Some(chat_history)

  Scenario: .context()-wrapped PromptCancelled is detected via chain traversal
    Given an anyhow::Error built from PromptError::PromptCancelled
    And the error is then wrapped with anyhow::Context::context
    When is_compaction_cancelled is called with that error
    Then the function returns true

  Scenario: StreamingError::Prompt-wrapped PromptCancelled is detected
    Given a StreamingError::Prompt carrying a boxed PromptError::PromptCancelled
    And the StreamingError is converted to anyhow::Error
    When is_compaction_cancelled is called with that error
    Then the function returns true

  Scenario: Bare string errors that merely say PromptCancelled are rejected
    Given an anyhow::Error created via anyhow::Error::msg("PromptCancelled")
    When is_compaction_cancelled is called with that error
    Then the function returns false

  Scenario: Other PromptError variants are not mistaken for cancellation
    Given an anyhow::Error built from PromptError::MaxDepthError
    When is_compaction_cancelled is called with that error
    Then the function returns false
