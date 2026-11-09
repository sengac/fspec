@done
@BUG-510
@testing
@cli
@context-management
@compaction
Feature: Missing limits stop before compaction recovery
  """
  Selected request preparation validates model capacity before installing the
  streaming compaction hook or preparing a replacement conversation. Missing
  capabilities are configuration errors, not evidence of context overflow.
  BUG-510: since PROV-165 (required-model-limits) prompt preflight refuses a
  missing context window or output limit before admission, with
  ModelLimitUnavailable naming the limit and the model; limits that leave no
  input budget reach admission and are refused with InvalidLimits.
  """

  Background: User story
    As a session owner
    I want missing model limits reported without compaction retries
    So that my conversation is preserved until valid limits are configured

  Scenario: Unavailable or unusable model limits fail before streaming changes history
    Given a selected model with missing context, missing output, or no input budget
    And a fresh or existing conversation
    When the user submits a prompt
    Then a typed configuration error is returned: the unavailable limit and model when a limit is missing, or invalid admission limits when no input budget remains
    And no provider request or compaction lifecycle event occurs
    And conversation messages and token counters remain unchanged
