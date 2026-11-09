@BUG-582
@done
@model-selection
@rust
@thinking-level
@providers
@high
@MODEL-017
Feature: Rhai custom-provider models declare their thinking levels
  """
  Plan: the declaration is read raw from the custom provider's config file (selection_metadata already reads the raw config and resolves the model alias by key or id), so ModelDef is unchanged and a bad declaration fails only that model instead of the whole provider load. local_reasoning's ThinkingLevelsDeclaration gains an engine-free resolution (the Rhai script owns the wire). ProviderManager::custom_model_reasoning (manager/custom_reasoning.rs) gives ThinkingTarget::for_provider_manager the custom model's capability for ProviderType::Custom with a new ProviderConfig capability source: none declared means no reasoning control and config_for refuses any level (MODEL-011 plug-in point, thinking-dialog-offers-only-the-models-levels.feature). rhai_config passes the numeric range and template toggle too; Rhai Off needs a declared off form. MODEL-010's Rhai scenario (no declared capability, level sent unchecked) is superseded and removed with its test; MODEL-011's description notes the unchecked send ended for custom providers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A custom-provider model declares its thinking levels under thinkingLevels on its entry in the provider config's models map, in MODEL-016's declaration shape: efforts, numericEfforts {kind whole|decimal, min, max}, tokenBudget {min, max} and templateToggle (enable_thinking | thinking), in any combination; unknown keys are refused (product owner, 2026-11-05, MODEL-011 question 2)
  #   2. A declared model offers exactly its declared levels in canonical order (MODEL-009/MODEL-011), and a level it does not declare is refused before any request is sent, naming the model and its declared levels; nothing is remapped (standing decision)
  #   3. A custom-provider model with no declaration has no reasoning control: the thinking dialog and /thinking say so and say to declare thinkingLevels on its model in the provider config, and a level a session already holds (a saved level or one kept across a model switch) is refused at the turn instead of being sent unchecked. This ends MODEL-010's unchecked send for custom-provider models (product owner, 2026-11-05; standing decision: no capability, no reasoning control)
  #   4. A Rhai script owns its wire, so a declaration on a Rhai model is not checked against an engine table: every part is allowed, and the script receives request.thinking_config {level, capability} with the declared capability (efforts, budget, toggle, numeric range, template toggle). A facade custom provider sends a declared level in its facade's form (MODEL-010)
  #   5. Off is offered on a custom-provider model only when its declaration has an off form, the effort none or a template toggle; otherwise Off is refused like any undeclared level (MODEL-010: Off is refused where the model has no off form)
  #   6. A declaration fspec cannot use (not an object of the declaration shape, an invalid effort name, an empty efforts list, a numeric range or token budget whose minimum is above its maximum, or no level at all) fails only that model, naming the provider, the model and the problem; the provider's other models keep theirs (MODEL-016 pattern)
  #   7. The bundled codex.rhai template's model declares Codex's levels for gpt-5.3-codex, efforts low, medium, high and xhigh (Codex's catalog for that model, research spec/attachments/MODEL-014), so its users keep level choice
  #   8. Scope: a cloud model missing from models.dev has no reasoning control too, like an undeclared custom-provider model: the dialog and /thinking offer only Model default (MODEL-011), and a level a session already holds is refused at the turn instead of being sent unchecked. This ends MODEL-010's unchecked send for every model whose levels fspec cannot know (product owner, 2026-11-05, MODEL-011 question 2; supervisor correction, 2026-11-06)
  #
  # EXAMPLES:
  #   1. The Rhai provider acme's model coder declares efforts low, medium, high: the thinking dialog for acme/coder offers low, medium, high and Model default, and at medium the script's thinking_config is level medium with capability efforts low, medium, high
  #   2. On acme/coder, /thinking xhigh is refused naming acme/coder and listing low, medium, high; nothing is sent
  #   3. acme's model plain declares nothing: the dialog says acme/plain has no reasoning control and to declare thinkingLevels on its model in the acme provider config; a session on acme/plain that already holds high is refused at the turn, where MODEL-010 sent level high with no capability
  #   4. acme's model wide declares whole-number efforts 1 to 100, a token budget 1024 to 32768 and templateToggle enable_thinking: it offers Off, the whole numbers 1 to 100 and the budget range (On is offered only on a toggle-only model, MODEL-016); at 40 the script receives level 40 and at budget 4096 level 4096, with the numeric range, budget and template toggle in its capability
  #   5. acme's model broken declares efforts ["Extra High"]: acme/broken has no reasoning control with a reason naming acme, broken and Extra High, while acme/coder in the same config still offers low, medium, high
  #   6. acme/coder (efforts low, medium, high) refuses Off because it declares no off form; acme's model lean declaring efforts none, low, high offers none, low, high and its Off sends level off to the script
  #   7. The bundled codex.rhai template's model default declares efforts low, medium, high, xhigh: at xhigh the template sends reasoning effort xhigh with summary auto, and none is refused
  #
  # ========================================
  Background: User Story
    As a user of a Rhai custom provider such as the bundled codex.rhai template
    I want to declare the thinking levels each model takes in the provider config, in the same thinkingLevels shape as a local profile
    So that the thinking dialog and /thinking offer exactly those levels and refuse others, instead of sending any level unchecked

  Scenario: A Rhai model offers the thinking levels it declares
    Given the Rhai provider acme's model coder declares efforts "low, medium, high"
    When the thinking options for acme/coder are read
    Then they are low, medium, high in that order
    And at medium the script's thinking_config is level "medium" with capability efforts "low, medium, high"

  Scenario: A level the declaration does not list is refused
    Given the Rhai provider acme's model coder declares efforts "low, medium, high"
    When the level xhigh is chosen for acme/coder
    Then it is refused naming acme/coder and listing low, medium, high

  Scenario: A model with no declaration has no reasoning control and a held level is refused
    Given the Rhai provider acme's model plain declares no thinking levels
    When the thinking options for acme/plain are read
    Then acme/plain has no reasoning control and the reason says to declare thinkingLevels on its model in the acme provider config
    And a turn at high on acme/plain is refused instead of being sent unchecked

  Scenario: A declaration takes numeric efforts, a token budget and a template toggle
    Given the Rhai provider acme's model wide declares whole-number efforts 1 to 100, a token budget 1024 to 32768 and templateToggle enable_thinking
    When the thinking options for acme/wide are read
    Then they are Off, the whole numbers 1 to 100 and the token budget 1024 to 32768, with no On since the model declares more than a toggle
    And at 40 the script receives level 40 and at a budget of 4096 level 4096, with the numeric range, the budget and the template toggle in its capability

  Scenario: A declaration fspec cannot use fails only that model
    Given the Rhai provider acme's model broken declares efforts "Extra High" and its model coder declares efforts "low, medium, high"
    When the thinking options are read for acme/broken and acme/coder
    Then acme/broken has no reasoning control with a reason naming acme, broken and Extra High
    And acme/coder still offers low, medium, high

  Scenario: Off needs a declared off form
    Given the Rhai provider acme's model coder declares efforts "low, medium, high" and its model lean declares efforts "none, low, high"
    When Off is chosen
    Then it is refused for acme/coder
    And acme/lean offers none, low, high and its Off sends level "off" to the script

  Scenario: The bundled codex.rhai template declares Codex's levels
    Given the bundled codex.rhai template installed as the custom provider codex-script
    When the thinking options for its default model are read
    Then they are low, medium, high, xhigh in that order
    And at xhigh the template sends reasoning effort "xhigh" with summary "auto"
    And none is refused

  Scenario: A cloud model missing from models.dev has no reasoning control and a held level is refused
    Given the cloud model openai/gpt-unlisted is missing from models.dev
    When the thinking options for openai/gpt-unlisted are read
    Then openai/gpt-unlisted has no reasoning control
    And a turn at high on openai/gpt-unlisted is refused naming the model instead of being sent unchecked
