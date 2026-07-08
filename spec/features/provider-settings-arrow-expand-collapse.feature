@done
@settings-navigation
@rust
@tui
@provider-settings
@PROV-134
Feature: Left/Right arrow does not expand/collapse providers in /provider view
  """
  Uses existing view.toggle_expansion(provider_id) (unconditional flip) guarded by the focused item's current expanded state so Right only expands and Left only collapses, mirroring model_selector's directional set/clear semantics.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Right arrow on a collapsed provider header expands it
  #   2. Right arrow on an already-expanded provider header is a no-op
  #   3. Left arrow on an expanded provider header collapses it
  #   4. Left arrow on an already-collapsed provider header is a no-op
  #   5. Arrow expand/collapse keeps the cursor on the same provider row
  #
  # EXAMPLES:
  #   1. Cursor on collapsed OpenAI header, press Right, OpenAI expands showing its child rows
  #   2. Cursor on expanded OpenAI header, press Left, OpenAI collapses hiding child rows
  #   3. Cursor on expanded OpenAI header, press Right again, nothing changes (still expanded, cursor unmoved)
  #   4. Cursor on a child row (e.g. a profile row), press Left, nothing changes (header-only toggle)
  #
  # ASSUMPTIONS:
  #   1. Left/Right only act on NavItemKind::Provider header rows; on non-provider rows they are no-ops (minimal scope, does not re-anchor to parent like model_selector). This keeps the fix header-scoped and avoids child-to-parent navigation complexity.
  #
  # ========================================
  Background: User Story
    As a provider settings user
    I want to expand and collapse provider rows with the Right and Left arrow keys
    So that I can navigate the provider tree the same way I do in the model selector

  Scenario: Right arrow expands a collapsed provider header
    Given the provider list is showing with the OpenAI provider collapsed
    Given the cursor is focused on the OpenAI provider header row
    When I press the Right arrow key
    Then the OpenAI provider becomes expanded
    Then the cursor remains on the OpenAI provider header row

  Scenario: Right arrow on an already-expanded provider header does nothing
    Given the provider list is showing with the OpenAI provider expanded
    Given the cursor is focused on the OpenAI provider header row
    When I press the Right arrow key
    Then the OpenAI provider remains expanded
    Then the cursor remains on the OpenAI provider header row

  Scenario: Left arrow collapses an expanded provider header
    Given the provider list is showing with the OpenAI provider expanded
    Given the cursor is focused on the OpenAI provider header row
    When I press the Left arrow key
    Then the OpenAI provider becomes collapsed
    Then the cursor remains on the OpenAI provider header row

  Scenario: Left arrow on an already-collapsed provider header does nothing
    Given the provider list is showing with the OpenAI provider collapsed
    Given the cursor is focused on the OpenAI provider header row
    When I press the Left arrow key
    Then the OpenAI provider remains collapsed
    Then the cursor remains on the OpenAI provider header row

  Scenario: Left arrow on a child row does nothing
    Given the provider list is showing with the OpenAI provider expanded
    Given the cursor is focused on a child row under the OpenAI provider
    When I press the Left arrow key
    Then the OpenAI provider remains expanded
    Then the cursor remains on the same child row
