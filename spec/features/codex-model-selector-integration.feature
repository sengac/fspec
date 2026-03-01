@PROV-018
Feature: Codex Models Not Showing in Model Selector - OAuth Models Hidden Under OpenAI Provider

  """
  buildCloudSections() in modelInitializationService.ts must extract codex models from the OpenAI provider when OAuth tokens exist. The Rust CodexProvider already handles OAuth token refresh and URL rewriting. Session creation modelPath format: 'codex/model-id'.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When Codex OAuth tokens exist, codex-specific models (ID contains 'codex') must be extracted from the OpenAI provider's model list into a separate 'Codex (ChatGPT)' section
  #   2. The synthetic Codex section must use providerId='codex' so that session creation routes to the Rust CodexProvider (not 'openai')
  #   3. Non-codex OpenAI models remain in the OpenAI section (only shown when OpenAI API key exists)
  #   4. When no Codex OAuth tokens exist, no Codex section appears (behavior unchanged)
  #   5. Codex models are identified by model ID containing 'codex' (case-insensitive)
  #   6. provider-mapping.ts must map 'codex' providerId to 'codex' internalName (identity mapping, already default)
  #
  # EXAMPLES:
  #   1. User has Codex OAuth tokens and models.dev returns OpenAI provider with 8 codex models + 1 non-codex model → model selector shows 'Codex (ChatGPT)' section with 8 codex models
  #   2. User has NO Codex OAuth tokens, no OpenAI API key → no OpenAI section, no Codex section (unchanged behavior)
  #   3. User has BOTH OpenAI API key AND Codex OAuth tokens → model selector shows OpenAI section (non-codex models) AND Codex section (codex models)
  #   4. User selects gpt-5.3-codex from Codex section → session created with modelPath 'codex/gpt-5.3-codex' (not 'openai/gpt-5.3-codex')
  #   5. User has persisted model 'codex/gpt-5.3-codex' and has OAuth tokens → model restored correctly on startup
  #
  # ========================================

  Background: User Story
    As a user with a ChatGPT Pro/Plus subscription
    I want to see and select Codex models in the model selector
    So that I can use my Codex subscription through fspec

  Scenario: Codex models appear in model selector when OAuth tokens exist
    Given I have authenticated with Codex via OAuth
    And models.dev returns OpenAI provider with codex models
    When models are loaded for the model selector
    Then I should see a Codex (ChatGPT) section with codex models
    And the Codex section should use providerId codex

  Scenario: No Codex section when OAuth tokens absent
    Given I have not authenticated with Codex via OAuth
    And I have no OpenAI API key configured
    When models are loaded for the model selector
    Then I should not see any Codex or OpenAI section in the model selector

  Scenario: Both OpenAI API key and Codex OAuth show separate sections
    Given I have authenticated with Codex via OAuth
    And I have an OpenAI API key configured
    And models.dev returns OpenAI provider with codex and non-codex models
    When models are loaded for the model selector
    Then I should see an OpenAI section with non-codex models
    And I should see a separate Codex (ChatGPT) section with codex models

  Scenario: Selecting a Codex model creates session with codex provider
    Given I have authenticated with Codex via OAuth
    And models.dev returns OpenAI provider with codex models
    When I select a codex model from the Codex section
    Then the model path should use codex as the provider prefix

  Scenario: Persisted Codex model restored on startup
    Given I have authenticated with Codex via OAuth
    And my last used model was codex/gpt-5.3-codex
    And models.dev returns OpenAI provider with codex models
    When models are loaded for the model selector
    Then the persisted codex model should be restored as the current model
    And the model providerId should be codex
