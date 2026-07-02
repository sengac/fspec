@done
@model-selector
@tui
@RPC-342
Feature: Model selector default expansion inverted (all-expanded vs all-collapsed)
  """
  TS parity port of ModelSelectorScreen.tsx:93-119 / useModelSelectorState.ts:148-150: expandedProviders starts as an empty set (all collapsed), auto-expanding only the current model's section. Replaces Rust set_providers expand-all (mod.rs:93).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On load, all providers start collapsed (TS parity: expandedProviders begins as an empty set)
  #   2. On load, only the provider section containing the current model is auto-expanded
  #   3. When no current model is set, or it is not found in any provider, every provider stays collapsed
  #   4. model_count and the title count ALL models across providers, not just the rows currently projected/visible
  #   5. Filtering still force-expands surviving providers at render-time, independent of the expanded set (no change to rows.rs)
  #   6. Re-loading providers (e.g. after refresh) re-applies the collapse-default plus auto-expand-current
  #
  # EXAMPLES:
  #   1. No current model set: load openai and anthropic, both providers stay collapsed, title still reads (3 models)
  #   2. Current model is claude-sonnet: load openai and anthropic, anthropic is expanded, openai stays collapsed, cursor lands on claude-sonnet
  #   3. Current model is gpt-4o: load openai and anthropic, openai is expanded, anthropic stays collapsed
  #   4. Filtering with collapsed providers: type o3 while openai is collapsed, o3-mini is still revealed by the force-expand-on-filter behavior
  #   5. Refresh re-collapse: with current gpt-4o loaded (openai expanded), a second set_providers call re-applies the default leaving only openai expanded
  #
  # ========================================
  Background: User Story
    As a fspec TUI user
    I want to have the model selector open with every provider collapsed except the section holding my current model
    So that the list fits the viewport on first open instead of overflowing

  Scenario: No current model set leaves every provider collapsed
    Given no current model is set
    When the model selector loads the "openai" and "anthropic" providers
    Then the "openai" provider is collapsed
    And the "anthropic" provider is collapsed
    And the title reads "Select Model (3 models)"

  Scenario: Only the current model's provider section is auto-expanded
    Given my current model is "claude-sonnet"
    When the model selector loads the "openai" and "anthropic" providers
    Then the "anthropic" provider is expanded
    And the "openai" provider is collapsed
    And the cursor is on the selectable row for "claude-sonnet"

  Scenario: A current model in the first provider expands only that section
    Given my current model is "gpt-4o"
    When the model selector loads the "openai" and "anthropic" providers
    Then the "openai" provider is expanded
    And the "anthropic" provider is collapsed

  Scenario: Filtering reveals matches inside collapsed providers
    Given no current model is set
    And the model selector has loaded the "openai" and "anthropic" providers all collapsed
    When I type the filter "o3"
    Then the model list shows the "o3-mini" model even though "openai" was collapsed

  Scenario: Reloading providers re-applies the collapse default
    Given my current model is "gpt-4o"
    And the model selector has loaded the providers with only "openai" expanded
    When the providers are reloaded
    Then the "openai" provider is expanded
    And the "anthropic" provider is collapsed
