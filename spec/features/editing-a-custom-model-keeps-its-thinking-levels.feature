@done
@rust
@profiles
@persistence
@thinking-level
@high
@BUG-511
Feature: Editing a custom model keeps its thinking levels declaration
  """
  Architecture notes:
  - The TUI custom-model dialog edits through SessionManagerHandle::update_custom_model (sessions handle_impl_6.rs), which builds a CustomModelDef from the wire CustomModelDefinition (conversions::custom_model_def_from_wire, thinking_levels: None) and calls profile_sections::save_custom_model.
  - Bug: save_custom_model_at (sessions/src/profile_sections/custom_models.rs) replaced the whole customModels entry with the dialog's definition, so MODEL-016's per-model thinkingLevels declaration (and any key fspec does not know) was deleted by every edit and the model lost its reasoning control.
  - Fix: an edit replaces only the keys the dialog edits (id, displayName, facade, contextWindow, maxOutputTokens, compactionThreshold, reasoning, hasVision), removing those the dialog left empty, and keeps every other key of the stored entry; this mirrors profile_persistence::merge_profile, which already keeps a profile's engine and thinkingLevels. The dialog itself does not show or set thinkingLevels; it is declared in the config file (MODEL-016).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Editing a custom model rewrites only the fields the custom-model dialog edits (id, display name, facade, context window, max output tokens, compaction threshold, reasoning, vision); a dialog field left empty is removed from the entry, as today
  #   2. Every other key of the stored customModels entry is kept unchanged by an edit, including MODEL-016's thinkingLevels declaration and keys fspec does not know, the way a profile edit already keeps the profile's engine and thinkingLevels (merge_profile)
  #   3. An edit that changes the model id keeps the entry's thinkingLevels declaration: it is the same entry under a new id
  #   4. Adding a new custom model writes only the dialog's fields; the dialog does not set thinkingLevels, which is declared in the config file (MODEL-016)
  #
  # EXAMPLES:
  #   1. A Strata profile's custom model qwen3.8-flash-next declares thinkingLevels efforts [none, low, medium, xhigh]; editing its context window to 131072 in the TUI saves the new context window and keeps the declaration, so the model still offers none, low, medium, xhigh
  #   2. Renaming that custom model's id to qwen3.8-flash-next-fp8 in the TUI keeps its thinkingLevels declaration on the renamed entry
  #   3. A custom model entry carrying a key fspec does not know (notes) keeps it after an edit, while a context window cleared in the dialog is removed from the entry
  #
  # ========================================
  Background: User Story
    As a developer with a local-server profile model that declares its thinking levels
    I want to edit that custom model's name, limits or other dialog fields in the TUI
    So that the model keeps its thinkingLevels declaration and its reasoning control

  Scenario: Editing a declared custom model's context window keeps its thinking levels
    Given a Strata profile whose custom model "qwen3.8-flash-next" declares thinking levels with efforts none, low, medium and xhigh
    When the user edits that custom model's context window to 131072 in the TUI
    Then the stored entry has context window 131072
    And the stored entry still declares the efforts none, low, medium and xhigh
    And the model still offers none, low, medium and xhigh

  Scenario: Renaming a declared custom model keeps its thinking levels on the renamed entry
    Given a Strata profile whose custom model "qwen3.8-flash-next" declares thinking levels with efforts none, low, medium and xhigh
    When the user renames that custom model to "qwen3.8-flash-next-fp8" in the TUI
    Then the profile has the custom model "qwen3.8-flash-next-fp8" and no "qwen3.8-flash-next"
    And the renamed entry still declares the efforts none, low, medium and xhigh

  Scenario: An edit keeps keys the dialog does not edit and removes dialog fields left empty
    Given a custom model "local-llm" stored with context window 32768 and a key "notes" that fspec does not know
    When the user edits "local-llm" in the TUI with the context window left empty
    Then the stored entry has no context window
    And the stored entry still has its "notes" key

  Scenario: Adding a custom model writes only the dialog's fields
    Given a Strata profile with no custom models
    When the user adds the custom model "fresh-model" in the TUI
    Then the stored entry for "fresh-model" has no thinking levels declaration
