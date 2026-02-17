@done
@context-management
@codelet
@CTX-004
Feature: Replace Rule-Based Anchor Detection with Pure LLM Analysis
  """
  15-second timeout with synthetic anchor fallback to prevent hanging on LLM analysis
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. LLM must completely replace all string matching and pattern detection logic
  #   2. AnchorDetector.detect() interface must remain unchanged to preserve backward compatibility
  #   3. LLM analysis must use session's existing llm_prompt function without breaking existing call sites
  #
  # EXAMPLES:
  #   1. When compaction runs on conversation with successful task completion, LLM identifies key moment and creates TaskCompletion anchor without analyzing string patterns
  #   2. When context compactor calls anchor detection, it seamlessly uses session's llm_prompt function for analysis without additional configuration
  #   3. When LLM analysis takes longer than 15 seconds per turn, system creates synthetic anchor and continues processing without hanging
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the LLM analysis accept the compactor's llm_prompt function as a parameter to avoid session integration issues?
  #   A: Yes, pass llm_prompt function as parameter to AnchorDetector.detect() to avoid session coupling issues. The compactor already has access to the session's llm_prompt function and can pass it down cleanly.
  #
  #   Q: Should existing CTX-001 tests be updated to expect LLM behavior, or should we create new test mocks that simulate LLM responses?
  #   A: Create new test mocks that simulate LLM responses. Keep existing CTX-001 tests as integration tests but add new unit tests with deterministic LLM mock responses to ensure predictable test behavior.
  #
  #   Q: Should LLM analysis stick to the existing 4 anchor types (ErrorResolution, TaskCompletion, UserCheckpoint, FeatureMilestone) or introduce new ones?
  #   A: Stick to existing 4 anchor types initially for backward compatibility. LLM should map detected meaningful moments to these types: ErrorResolution, TaskCompletion, UserCheckpoint, FeatureMilestone. Can extend later if needed.
  #
  # ========================================
  Background: User Story
    As a developer using context compaction
    I want to get accurate anchor point detection from LLM analysis
    So that I have context preserved at truly meaningful conversation moments instead of false positives from string matching

  Scenario: LLM identifies meaningful moments without string pattern analysis
    Given a conversation turn with successful task completion
    And the compaction system runs anchor detection
    When LLM analyzes the conversation turn content
    Then it creates a TaskCompletion anchor based on semantic understanding
    And it does not use any string matching or pattern detection logic

  Scenario: Context compactor seamlessly integrates with session's LLM function
    Given the context compactor has access to session's llm_prompt function
    When anchor detection is triggered
    Then the compactor passes llm_prompt function to AnchorDetector.detect()
    And LLM analysis runs without additional configuration or setup

  Scenario: LLM analysis timeout creates synthetic anchor without hanging
    Given LLM analysis is taking longer than 15 seconds per turn
    When the timeout threshold is reached
    Then the system creates a synthetic anchor as fallback
    And processing continues without hanging or blocking
    And the synthetic anchor maintains system reliability
