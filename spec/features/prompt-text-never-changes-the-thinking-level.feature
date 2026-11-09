@done
@agent-core
@rust
@model-selection
@thinking-level
@high
@MODEL-013
Feature: Prompt text never changes the thinking level
  """
  Architecture notes:
  - codelet-agent-loop: a turn's thinking level is the session's chosen base level (base_thinking_level (MODEL-011: from the stored ThinkingLevel) over BackgroundSession::get_base_thinking_level); agent_loop/thinking.rs::resolve_turn_thinking no longer reads the prompt text.
  - The keyword detection in thinking_level_detection.rs (detect_thinking_level, has_disable_keywords, compute_effective_thinking_level and its regexes) is removed, and effective_turn_thinking_level with it.
  - Decision: spec/attachments/MODEL-009/thinking-levels-research.md section 8, item 8 (product owner, 2026-11-04). This supersedes dynamic-thinking-level-detection-via-keywords.feature and the keyword scenarios of thinking-level-dialog.feature and sessions-without-a-chosen-thinking-level.feature.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Prompt text never changes the thinking level: ultrathink, megathink, think hard(er), think about, nothink and quickly/briefly/short/fast/rapid are ordinary words
  #   2. A session with a chosen level thinks at exactly that level on every turn
  #   3. A session with no chosen level sends no thinking parameter whatever the prompt says
  #   4. The level changes only through the thinking dialog or /thinking
  #
  # EXAMPLES:
  #   1. A session set to High is prompted 'quickly list files'; the turn thinks at High, not Off
  #   2. A session set to Low is prompted 'ultrathink about the architecture'; the turn thinks at Low, not High
  #   3. A session with no chosen level is prompted 'ultrathink about this' and 'nothink, just answer'; neither turn sends a thinking parameter
  #
  # ========================================
  Background: User Story
    As a developer prompting an agent
    I want to have the thinking level change only when I choose it in the dialog or with /thinking
    So that words like 'quickly' or 'ultrathink' in my prompt never silently change how hard the model thinks

  Scenario: Prompt words never lower a chosen level
    Given a session whose thinking level is High
    When a turn is prompted with "quickly list files"
    Then the turn thinks at High

  Scenario: Prompt words never raise a chosen level
    Given a session whose thinking level is Low
    When a turn is prompted with "ultrathink about the architecture"
    Then the turn thinks at Low
