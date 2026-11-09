@BUG-582
@done
@model-selection
@rust
@thinking-level
@providers
@high
@MODEL-009
Feature: Thinking levels come from each model's own capabilities
  """
  Architecture notes:
  - Lives in codelet-providers, rust/providers/src/models: ModelInfo gains reasoning_options (the models.dev field it dropped until now), and a reasoning module defines ReasoningOption, ReasoningLevel and ReasoningCapability.
  - ReasoningOption is the models.dev entry, tagged by "type": effort {values}, budget_tokens {min?, max?} or toggle. Any other type is kept as an unrecognised entry, so the rest of the catalog still loads, and resolving that model's capability is an error naming the type.
  - ReasoningCapability is resolved per model: efforts in canonical order (none < minimal < low < medium < high < xhigh < max < ultra; names outside that list keep their advertised order after it), an optional budget range and a toggle flag. No reasoning_options, or an empty list, resolves to no capability, which means no reasoning control is offered.
  - A level is checked against the capability before any request is built: an unadvertised effort, a budget the model does not take, or a budget outside min..max is an error naming the model and what it supports. Nothing is remapped or defaulted.
  - Decisions: spec/attachments/MODEL-009/thinking-levels-research.md section 8. The wire mapping, TUI selector, per-model default, keyword removal, Codex /models, Claude per-turn effort and local-server levels are MODEL-010 to MODEL-016.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A reasoning level is either a named effort (none, minimal, low, medium, high, xhigh, max, ultra, or another name the model advertises) or a numeric token budget
  #   2. Named efforts have one canonical order: none < minimal < low < medium < high < xhigh < max < ultra
  #   3. A model's reasoning capability is read from its models.dev reasoning_options: an effort list, a budget range, a toggle, or any combination
  #   4. A model with no reasoning_options, or an empty list, has no reasoning control; no level is offered for it
  #   5. A level the model does not advertise is refused before the request is sent, naming the model and its supported levels
  #   6. A numeric budget outside the model's advertised min..max is refused the same way
  #   7. fspec never remaps a level to a nearby supported one
  #   8. An unrecognised reasoning_options type is an error for that model, naming the type; other models are unaffected
  #
  # EXAMPLES:
  #   1. openai/gpt-5.5 advertises efforts [none, low, medium, high, xhigh]; its capability lists exactly those, in canonical order
  #   2. anthropic/claude-opus-4-6 advertises efforts [low, medium, high, max] and budget_tokens min 1024; its capability has both
  #   3. zai/glm-4.5 advertises only a toggle; its capability is a toggle with no effort list
  #   4. A model whose reasoning_options is an empty list has no reasoning capability
  #   5. Choosing xhigh for anthropic/claude-opus-4-6 is refused with an error naming the model and listing low, medium, high, max
  #   6. Choosing a 512-token budget for anthropic/claude-haiku-4-5 (min 1024) is refused with an error naming the model and its range
  #   7. Choosing high for openai/gpt-5.5 is accepted and stays high
  #   8. A model whose reasoning_options holds {"type": "temperature_curve"} fails to resolve a capability with an error naming temperature_curve, while openai/gpt-5.5 in the same catalog still resolves
  #
  # ========================================
  Background: User Story
    As a developer choosing how hard a model thinks
    I want to pick from exactly the thinking levels the selected model supports
    So that every level I choose is one the model really honours, and none is silently dropped or remapped

  Scenario: An effort-list model offers exactly its advertised efforts in canonical order
    Given a models.dev catalog where openai/gpt-5.5 advertises efforts "xhigh, none, high, low, medium"
    When the reasoning capability of openai/gpt-5.5 is resolved
    Then its efforts are "none, low, medium, high, xhigh"
    And it takes no token budget
    And it has no toggle

  Scenario: A model can advertise both efforts and a token budget
    Given a models.dev catalog where anthropic/claude-opus-4-6 advertises efforts "low, medium, high, max" and a token budget of at least 1024
    When the reasoning capability of anthropic/claude-opus-4-6 is resolved
    Then its efforts are "low, medium, high, max"
    And it takes a token budget from 1024 with no upper limit

  Scenario: A toggle-only model has no effort list
    Given a models.dev catalog where zai/glm-4.5 advertises only a toggle
    When the reasoning capability of zai/glm-4.5 is resolved
    Then it has a toggle
    And its efforts are empty
    And it takes no token budget

  Scenario: A model with no reasoning options has no reasoning control
    Given a models.dev catalog where example/plain-model has an empty reasoning_options list
    And example/older-model has no reasoning_options at all
    When the reasoning capabilities of both models are resolved
    Then neither model has a reasoning capability
    And choosing effort "low" for example/plain-model is refused with an error naming example/plain-model

  Scenario: An effort the model does not advertise is refused, not remapped
    Given a models.dev catalog where anthropic/claude-opus-4-6 advertises efforts "low, medium, high, max"
    When effort "xhigh" is chosen for anthropic/claude-opus-4-6
    Then the choice is refused with an error naming anthropic/claude-opus-4-6
    And the error lists "low, medium, high, max"

  Scenario: A token budget outside the advertised range is refused
    Given a models.dev catalog where anthropic/claude-haiku-4-5 takes a token budget of at least 1024
    And google/gemini-2.5-flash takes a token budget from 0 to 24576
    When a 512-token budget is chosen for anthropic/claude-haiku-4-5
    And a 30000-token budget is chosen for google/gemini-2.5-flash
    Then both choices are refused with errors naming the model and its budget range
    And a 2048-token budget is accepted for anthropic/claude-haiku-4-5

  Scenario: A supported effort is accepted unchanged
    Given a models.dev catalog where openai/gpt-5.5 advertises efforts "none, low, medium, high, xhigh"
    When effort "high" is chosen for openai/gpt-5.5
    Then the choice is accepted as effort "high"

  Scenario: An unrecognised reasoning option type fails only that model
    Given a models.dev catalog where example/odd-model advertises a reasoning option of type "temperature_curve"
    And openai/gpt-5.5 in the same catalog advertises efforts "none, low, medium, high, xhigh"
    When the catalog is loaded
    Then resolving the capability of example/odd-model is an error naming "temperature_curve"
    And the capability of openai/gpt-5.5 still resolves

  Scenario: A reasoning option with unusable data fails only that model
    Given a models.dev catalog where nvidia/nemotron-3-ultra-550b-a55b lists a token budget with min -1 and max 32768
    When the catalog is loaded
    Then resolving the capability of nvidia/nemotron-3-ultra-550b-a55b is an error naming the bound -1
    And openai/gpt-5.5 in the same catalog advertises efforts "none, low, medium, high, xhigh"
    And the capability of openai/gpt-5.5 still resolves
