@PROV-018 @PROV-033
Feature: Codex Model Selector Integration — All OpenAI Cloud Models in Codex Section When OAuth Active

  """
  PROV-033 FIX: When Codex OAuth tokens exist, extractCodexSection() must move ALL models
  from the OpenAI cloud provider into the synthetic 'Codex (ChatGPT)' section. The previous
  isCodexModel() filter (which matched only models with 'codex' in the ID) was fundamentally
  wrong — real models.dev OpenAI models are gpt-5.2, o3-pro, gpt-4.1, etc., none of which
  contain 'codex'. The isCodexModel() function must be removed entirely.

  The Rust CodexProvider handles OAuth token refresh and URL rewriting.
  Session creation modelPath format: 'codex/model-id'.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT (PROV-033)
  # ========================================
  #
  # BUSINESS RULES:
  #   0. When Codex OAuth tokens exist, extractCodexSection() must move ALL models from the
  #      OpenAI cloud provider into the Codex (ChatGPT) section — not just models with 'codex'
  #      in the ID. The isCodexModel() filter must be removed from the extraction logic.
  #   1. When Codex OAuth is active, the 'OpenAI' cloud provider section must NOT appear in
  #      the model selector. There should be zero 'OpenAI' cloud section — only the synthetic
  #      'Codex (ChatGPT)' section with ALL the models.
  #   2. The isCodexModel() function must be removed. It must NOT filter by model ID string
  #      matching. When Codex OAuth is active, ALL models from the openai provider are Codex
  #      models by definition.
  #   3. Test fixtures must use realistic model IDs from models.dev (gpt-5.2, gpt-5, o3-pro,
  #      o4-mini, gpt-4.1, etc.) — not fake IDs like 'gpt-5.3-codex'.
  #   4. Local model profiles (openai: profilename with folder icon) must remain unaffected.
  #      Profile sections from loadProfileSections() are separate from cloud sections.
  #
  # EXAMPLES:
  #   0. User has Codex OAuth active, models.dev returns OpenAI provider with gpt-5.2, gpt-5,
  #      o3-pro, o4-mini, gpt-4.1, o1 — model screen shows 'Codex (ChatGPT)' section with ALL
  #      these models. No 'OpenAI' cloud section exists.
  #   1. User has Codex OAuth active AND an OpenAI API key — model screen shows 'Codex (ChatGPT)'
  #      with ALL cloud models. No duplicate 'OpenAI' cloud section. Local profiles still separate.
  #   2. User has NO Codex OAuth but HAS an OpenAI API key — model screen shows 'OpenAI' section
  #      with cloud models. No 'Codex (ChatGPT)' section appears.
  #   3. User selects gpt-5.2 from the Codex (ChatGPT) section — session creates with
  #      providerId 'codex' and routes through Codex OAuth.
  #
  # ========================================

  Background: User Story
    As a user with a ChatGPT Pro/Plus subscription
    I want ALL OpenAI cloud models to appear in the Codex (ChatGPT) section when I have OAuth tokens
    So that I can use my Codex subscription to access any OpenAI model through fspec

  Scenario: All OpenAI cloud models appear in Codex section when OAuth tokens exist
    Given I have authenticated with Codex via OAuth
    And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, o4-mini, gpt-4.1, and o1
    When models are loaded for the model selector
    Then I should see a Codex (ChatGPT) section containing ALL OpenAI cloud models
    And the Codex section should use providerId codex
    And no OpenAI cloud section should exist

  Scenario: No Codex section when OAuth tokens absent
    Given I have not authenticated with Codex via OAuth
    And I have no OpenAI API key configured
    When models are loaded for the model selector
    Then I should not see any Codex or OpenAI section in the model selector

  Scenario: Codex OAuth active with OpenAI API key shows only Codex section for cloud models
    Given I have authenticated with Codex via OAuth
    And I have an OpenAI API key configured
    And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, and gpt-4.1
    When models are loaded for the model selector
    Then I should see a Codex (ChatGPT) section with ALL cloud models
    And no OpenAI cloud section should exist
    And local profile sections should remain unaffected

  Scenario: Selecting a model from Codex section creates session with codex provider
    Given I have authenticated with Codex via OAuth
    And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, o4-mini, gpt-4.1, and o1
    When I select gpt-5.2 from the Codex section
    Then the model path should be codex/gpt-5.2

  Scenario: Persisted Codex model restored on startup
    Given I have authenticated with Codex via OAuth
    And my last used model was codex/gpt-5.2
    And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, o4-mini, gpt-4.1, and o1
    When models are loaded for the model selector
    Then the persisted codex model should be restored as the current model
    And the model providerId should be codex

  Scenario: OpenAI API key without Codex OAuth shows OpenAI section
    Given I have not authenticated with Codex via OAuth
    And I have an OpenAI API key configured
    And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, and gpt-4.1
    When models are loaded for the model selector
    Then I should see an OpenAI section with all cloud models
    And no Codex section should exist
